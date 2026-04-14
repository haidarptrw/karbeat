use std::{ collections::BTreeSet, sync::Arc };

use serde::{ Deserialize, Serialize };

use crate::{
    commands::AudioCommand,
    context::ctx,
    core::project::{
        ApplicationState,
        Clip,
        GeneratorInstance,
        GeneratorInstanceType,
        KarbeatSource,
        PluginInstance,
        mixer::MixerChannel,
    },
    shared::{ BusId, GeneratorId, id::{ ClipId, TrackId } },
};
use karbeat_utils::color::Color;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct KarbeatTrack {
    pub id: TrackId,
    pub name: String,
    pub color: Color,
    pub track_type: TrackType,
    pub clips: BTreeSet<Arc<Clip>>,
    pub max_sample_index: u32,
    pub generator: Option<GeneratorInstance>,
}

impl Default for KarbeatTrack {
    fn default() -> Self {
        Self {
            id: Default::default(),
            name: Default::default(),
            color: Color::new_from_rgb(255, 255, 255),
            track_type: TrackType::Audio,
            clips: BTreeSet::new(),
            max_sample_index: 0,
            generator: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TrackType {
    Audio,
    Midi,
    Automation,
}

impl std::str::FromStr for TrackType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "audio" => Ok(TrackType::Audio),
            "midi" => Ok(TrackType::Midi),
            "automation" => Ok(TrackType::Automation),
            _ => Err("Invalid track type".into()),
        }
    }
}

impl KarbeatTrack {
    pub fn new(
        id: TrackId,
        name: &str,
        color: Color,
        track_type: TrackType,
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            color,
            track_type,
            clips: BTreeSet::new(),
            max_sample_index: 0,
            generator: None
        }
    }

    pub fn clips(&self) -> &BTreeSet<Arc<Clip>> {
        return &self.clips;
    }

    pub fn clips_to_vec(&self) -> Vec<Arc<Clip>> {
        self.clips.iter().cloned().collect()
    }

    pub fn track_type(&self) -> &TrackType {
        return &self.track_type;
    }

    pub fn get_clip(&self, clip_id: &ClipId) -> Option<Arc<Clip>> {
        self.clips
            .iter()
            .find(|c| c.id == *clip_id)
            .cloned()
    }

    /// Add a new clip to the track. it will return Err if
    /// the clip type is incompatible with the track type
    pub fn add_clip(&mut self, clip: Clip) -> anyhow::Result<u32> {
        let is_valid = match (&self.track_type, &clip.source) {
            (TrackType::Audio, KarbeatSource::Audio(_)) => true,
            (TrackType::Midi, KarbeatSource::Midi { .. }) => true,
            (TrackType::Automation, KarbeatSource::Automation(_)) => true,
            // Allow Automation on Audio/Midi tracks? usually yes, but strictly speaking:
            _ => false,
        };

        if is_valid {
            // Calculate potential new max index BEFORE moving clip
            let clip_end_sample = clip.start_time + clip.loop_length;

            // 1. Wrap in Arc immediately
            let clip_arc = Arc::new(clip);

            // 2. COW: Get mutable access to the vector
            let clips_set = &mut self.clips;

            clips_set.insert(clip_arc);

            // update the max sample index
            if clip_end_sample > self.max_sample_index {
                self.max_sample_index = clip_end_sample;
            }

            // Return the end sample of this new clip
            return Ok(clip_end_sample);
        } else {
            return Err(anyhow::anyhow!("Warning: Mismatched Clip Source for Track Type"));
        }
    }

    /// Remove the clip, change max_index_sample if the deleted clip are the latest end sample index
    pub fn remove_clip(&mut self, clip_id: &ClipId) -> anyhow::Result<Arc<Clip>> {
        let clips_set = &mut self.clips;

        let initial_len = clips_set.len();
        let clip = clips_set
            .iter()
            .find(|c| c.id == *clip_id)
            .ok_or(anyhow::anyhow!("Clip not found"))?
            .clone();
        clips_set.retain(|c| c.id != *clip_id);

        if clips_set.len() < initial_len {
            // Recalculate max sample index if something was removed
            self.max_sample_index = clips_set
                .iter()
                .map(|c| c.start_time + c.loop_length)
                .max()
                .unwrap_or(0);

            Ok(clip)
        } else {
            Err(anyhow::anyhow!("Clip not found"))
        }
    }

    /// Remove all clips that have the same source ID (only remove
    /// audio clip because it needs a cascading remove upon an audio source deletion)
    pub fn remove_clip_by_source_id(&mut self, source_id: impl Into<u32>, is_generator: bool) {
        let source_id_u32: u32 = source_id.into();
        let clips_set = &mut self.clips;

        clips_set.retain(|clip_arc| {
            match &clip_arc.source {
                KarbeatSource::Audio(source_id) => {
                    if !is_generator { source_id != &source_id_u32 } else { true }
                }
                KarbeatSource::Midi { .. } => true,
                KarbeatSource::Automation(_) => true,
            }
        });
    }

    /// Optimized for adding multiple clips (e.g., Paste / Duplicate).
    pub fn add_clips_bulk(&mut self, new_clips: &[Arc<Clip>]) {
        let clips_vec = &mut self.clips;
        clips_vec.extend(new_clips.iter().cloned());

        self.max_sample_index = clips_vec
            .iter()
            .map(|c| c.start_time + c.loop_length)
            .max()
            .unwrap_or(0);
    }

    pub fn update_max_sample_index(&mut self) {
        self.max_sample_index = self.clips
            .iter()
            .map(|c| c.start_time + c.loop_length)
            .max()
            .unwrap_or(0);
    }

    pub fn cut_clip(
        &mut self,
        clip_id: &ClipId,
        cut_point_sample: u32,
        clip_counter: &mut u32
    ) -> anyhow::Result<(Clip, Clip)> {
        let clip_arc = self
            .get_clip(clip_id)
            .ok_or_else(|| {
                anyhow::anyhow!("Clip ID {:?} not found in track {:?}", clip_id, self.id)
            })?;

        if
            cut_point_sample > clip_arc.start_time &&
            cut_point_sample < clip_arc.start_time + clip_arc.loop_length
        {
            // Remove using the exact Arc reference, not the inner data reference
            self.clips.remove(&clip_arc);

            let clip = clip_arc.as_ref();

            // Create left clip
            let mut left_clip = clip.clone();
            left_clip.loop_length = cut_point_sample - left_clip.start_time;
            left_clip.id = *clip_id; // Retain original ID for the first half
            self.clips.insert(Arc::new(left_clip.clone()));

            // Create right clip
            let mut right_clip = clip.clone();
            right_clip.id = ClipId::next(clip_counter); // Consume counter for new ID
            right_clip.start_time = cut_point_sample;
            right_clip.offset_start += cut_point_sample - clip.start_time;
            right_clip.loop_length = clip.start_time + clip.loop_length - cut_point_sample;
            self.clips.insert(Arc::new(right_clip.clone()));

            self.update_max_sample_index();

            log::info!("Successfully cut the clip");
            Ok((left_clip, right_clip))
        } else {
            return Err(anyhow::anyhow!("Cannot cut clip outside its boundaries"));
        }
    }
}

impl ApplicationState {
    pub fn add_new_audio_track(&mut self) -> Arc<KarbeatTrack> {
        let new_track_id = TrackId::next(&mut self.track_counter);
        let new_track = KarbeatTrack {
            track_type: TrackType::Audio,
            id: new_track_id,
            name: format!("Track {}", new_track_id.to_string()),
            ..Default::default()
        };
        let track_arc = Arc::new(new_track);
        self.tracks.insert(new_track_id, track_arc.clone());

        // Create a corresponding mixer channel and default routing
        self.mixer.channels.insert(new_track_id, Arc::new(MixerChannel::default()));
        self.mixer.add_track_default_routing(new_track_id);
        track_arc
    }

    /// Add a new MIDI track with a generator by its registry ID (preferred method).
    pub fn add_new_midi_track_with_generator_id(
        &mut self,
        registry_id: u32
    ) -> anyhow::Result<Arc<KarbeatTrack>> {
        let gen_id = GeneratorId::next(&mut self.generator_counter);
        let track_id = TrackId::next(&mut self.track_counter);

        // Create the plugin via registry using ID
        let (generator_plugin, generator_name, default_params) = {
            let registry = ctx().plugin_registry.read();

            if let Some((generator_box, name)) = registry.create_generator_by_id(registry_id) {
                // Get default parameters BEFORE sending to audio thread
                let params = generator_box.default_parameters();
                (generator_box, name, params)
            } else {
                let message = format!("Generator with ID {} not found in registry", registry_id);
                log::error!("{}", message);
                // Decrement counters if failed to prevent gaps/orphans
                self.generator_counter -= 1;
                self.track_counter -= 1;
                return Err(anyhow::anyhow!("{}", message));
            }
        };

        // Send the plugin to the audio thread (lock-free)
        if let Some(sender) = ctx().command_sender.lock().as_mut() {
            let _ = sender.push(AudioCommand::AddGenerator {
                generator_id: gen_id,
                track_id,
                plugin: generator_plugin,
            });
        }

        // Create plugin instance descriptor with registry ID and default parameters
        let plugin_instance = PluginInstance::new_with_params(
            registry_id,
            &generator_name,
            default_params
        );

        let generator = GeneratorInstance {
            id: gen_id,
            instance_type: GeneratorInstanceType::Plugin(plugin_instance),
        };
        self.generator_pool.insert(gen_id, Arc::new(generator.clone()));

        let new_track = KarbeatTrack {
            track_type: TrackType::Midi,
            id: track_id,
            name: generator_name.clone(),
            #[allow(clippy::unwrap_used)]
            color: Color::new_from_string("#FF8A65").unwrap(),
            generator: Some(generator),
            ..Default::default()
        };

        let track_arc = Arc::new(new_track);
        self.tracks.insert(track_id, track_arc.clone());

        // Create a corresponding mixer channel and default routing
        self.mixer.channels.insert(track_id, Arc::new(MixerChannel::default()));
        self.mixer.add_track_default_routing(track_id);

        log::info!(
            "New MIDI track with generator {} (registry_id={}) is successfully created",
            generator_name,
            registry_id
        );
        Ok(track_arc)
    }

    pub fn add_new_automation_track_from_bus(&mut self, bus_id: BusId) {
        let track_id = TrackId::next(&mut self.track_counter);

        // find the Bus
        // TODO: Continue this part


    }

    /// Remove a track and clean up its mixer channel, routing, generator, and automation lanes.
    pub fn remove_track(&mut self, track_id: TrackId) -> anyhow::Result<()> {
        // Get the generator ID before removing the track
        let generator_id = self.tracks
            .get(&track_id)
            .and_then(|t| t.generator.as_ref().map(|g| g.id));

        // Remove the track
        if self.tracks.shift_remove(&track_id).is_none() {
            return Err(anyhow::anyhow!("Track {:?} not found", track_id));
        }

        // Remove the mixer channel
        self.mixer.channels.shift_remove(&track_id);

        // Remove all routing connections for this track
        self.mixer.remove_track_routing(track_id);

        // Remove the generator from the pool if the track had one
        if let Some(gen_id) = generator_id {
            self.generator_pool.shift_remove(&gen_id);
        }

        // Remove all automation lanes for this track
        self.remove_automation_lanes_for_track(track_id);

        self.update_max_sample_index();

        Ok(())
    }

    pub fn cut_clip(
        &mut self,
        track_id: &TrackId,
        clip_id: &ClipId,
        cut_point_sample: u32
    ) -> anyhow::Result<(Clip, Clip)> {
        let track_arc = self.tracks
            .get_mut(track_id)
            .ok_or_else(|| anyhow::anyhow!("Track not found"))?;

        let track_mut: &mut KarbeatTrack = Arc::make_mut(track_arc);

        let clip_counter = &mut self.clip_counter;

        let res = track_mut.cut_clip(clip_id, cut_point_sample, clip_counter)?;

        Ok(res)
    }
}
