use karbeat_utils::define_id;
// src/core/track/mod.rs
pub mod audio_waveform;
pub mod midi;

use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};

use crate::{
    commands::AudioCommand,
    context::ctx,
    core::project::{
        clip::ClipId, generator::GeneratorId, mixer::MixerChannel, ApplicationState, Clip,
        GeneratorInstance, GeneratorInstanceType, KarbeatSource, PluginInstance,
    },
};
use karbeat_utils::color::Color;

define_id!(TrackId);

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    pub fn clips(&self) -> &BTreeSet<Arc<Clip>> {
        return &self.clips;
    }

    pub fn clips_to_vec(&self) -> Vec<Arc<Clip>> {
        self.clips.iter().cloned().collect()
    }

    pub fn track_type(&self) -> &TrackType {
        return &self.track_type;
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
            return Err(anyhow::anyhow!(
                "Warning: Mismatched Clip Source for Track Type"
            ));
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

        clips_set.retain(|clip_arc| match &clip_arc.source {
            KarbeatSource::Audio(source_id) => {
                if !is_generator {
                    source_id != &source_id_u32
                } else {
                    true
                }
            }
            KarbeatSource::Midi { .. } => true,
            KarbeatSource::Automation(_) => true,
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
        self.max_sample_index = self
            .clips
            .iter()
            .map(|c| c.start_time + c.loop_length)
            .max()
            .unwrap_or(0);
    }
}

impl ApplicationState {
    pub fn add_new_track(&mut self, track_type: TrackType) {
        let new_track_id = TrackId::next(&mut self.track_counter);
        let new_track = KarbeatTrack {
            track_type,
            id: new_track_id,
            name: format!("Track {}", new_track_id.to_string()),
            ..Default::default()
        };
        self.tracks.insert(new_track_id, Arc::new(new_track));

        // Create a corresponding mixer channel and default routing
        self.mixer
            .channels
            .insert(new_track_id, Arc::new(MixerChannel::default()));
        self.mixer.add_track_default_routing(new_track_id);
    }

    /// Add a new MIDI track with a generator by its registry ID (preferred method).
    pub fn add_new_midi_track_with_generator_id(&mut self, registry_id: u32) -> anyhow::Result<()> {
        let gen_id = GeneratorId::next(&mut self.generator_counter);
        let track_id = TrackId::next(&mut self.track_counter);

        // Create the plugin via registry using ID
        let (generator_plugin, generator_name, default_params) = {
            let registry = ctx()
                .plugin_registry
                .read()
                .expect("Failed to lock registry");

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
        if let Some(sender) = ctx().command_sender.lock().unwrap().as_mut() {
            let _ = sender.push(AudioCommand::AddGenerator {
                generator_id: gen_id,
                track_id,
                plugin: generator_plugin,
            });
        }

        // Create plugin instance descriptor with registry ID and default parameters
        let plugin_instance =
            PluginInstance::new_with_params(registry_id, &generator_name, default_params);

        let generator = GeneratorInstance {
            id: gen_id,
            effects: HashMap::new(),
            instance_type: GeneratorInstanceType::Plugin(plugin_instance),
        };
        self.generator_pool
            .insert(gen_id, Arc::new(RwLock::new(generator.clone())));

        let new_track = KarbeatTrack {
            track_type: TrackType::Midi,
            id: track_id,
            name: generator_name.clone(),
            color: Color::new_from_string("#FF8A65").unwrap(),
            generator: Some(generator),
            ..Default::default()
        };

        self.tracks.insert(track_id, Arc::new(new_track));

        // Create a corresponding mixer channel and default routing
        self.mixer
            .channels
            .insert(track_id, Arc::new(MixerChannel::default()));
        self.mixer.add_track_default_routing(track_id);

        log::info!(
            "New MIDI track with generator {} (registry_id={}) is successfully created",
            generator_name,
            registry_id
        );
        Ok(())
    }

    /// Add a new MIDI track with a generator by name (backwards compatible).
    /// Internally looks up the registry ID and delegates to the ID-based method.
    pub fn add_new_midi_track_with_generator(
        &mut self,
        generator_name: &str,
    ) -> anyhow::Result<()> {
        // Look up the registry ID by name
        let registry_id = {
            let registry = ctx()
                .plugin_registry
                .read()
                .expect("Failed to lock registry");

            registry
                .get_generator_id_by_name(generator_name)
                .ok_or_else(|| {
                    anyhow::anyhow!("Generator '{}' not found in registry", generator_name)
                })?
        };

        // Delegate to ID-based method
        self.add_new_midi_track_with_generator_id(registry_id)
    }

    /// Remove a track and clean up its mixer channel, routing, generator, and automation lanes.
    pub fn remove_track(&mut self, track_id: TrackId) -> anyhow::Result<()> {
        // Get the generator ID before removing the track
        let generator_id = self
            .tracks
            .get(&track_id)
            .and_then(|t| t.generator.as_ref().map(|g| g.id));

        // Remove the track
        if self.tracks.remove(&track_id).is_none() {
            return Err(anyhow::anyhow!("Track {:?} not found", track_id));
        }

        // Remove the mixer channel
        self.mixer.channels.remove(&track_id);

        // Remove all routing connections for this track
        self.mixer.remove_track_routing(track_id);

        // Remove the generator from the pool if the track had one
        if let Some(gen_id) = generator_id {
            self.generator_pool.remove(&gen_id);
        }

        // Remove all automation lanes for this track
        self.remove_automation_lanes_for_track(track_id);

        self.update_max_sample_index();

        Ok(())
    }
}
