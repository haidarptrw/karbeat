use std::{collections::BTreeSet, sync::Arc};

use karbeat_plugin_types::ParamBounds;
use serde::{Deserialize, Serialize};

use crate::{
    commands::AudioCommand,
    context::ctx,
    core::project::{
        automation::{AutomationLane, AutomationTarget},
        clip::ClipTimeUnit,
        mixer::MixerChannel,
        ApplicationState, Clip, DawSource, GeneratorInstance, GeneratorInstanceType,
        PluginInstance,
    },
    shared::{
        id::{ClipId, TrackId},
        BusId, GeneratorId,
    },
};
use karbeat_utils::color::Color;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct AudioTrack {
    pub id: TrackId,
    pub name: String,
    pub color: Color,
    pub track_type: TrackType,
    pub clips: BTreeSet<Arc<Clip>>,
    pub max_sample_index: u32,
    pub generator: Option<GeneratorInstance>,
    /// ======================================
    /// Track Sorting Order
    /// Represents the track index position to be displayed in the UI.
    /// Used for preserving layout on load/save and custom reordering.
    /// ======================================
    pub order_idx: usize,
}

impl Default for AudioTrack {
    fn default() -> Self {
        Self {
            id: Default::default(),
            name: Default::default(),
            color: Color::new_from_rgb(255, 255, 255),
            track_type: TrackType::Audio,
            clips: BTreeSet::new(),
            max_sample_index: 0,
            generator: None,
            order_idx: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TrackType {
    Audio,
    Midi,
    Automation,
}

pub type RemovedTrackType = TrackType;

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

impl AudioTrack {
    pub fn new(id: TrackId, name: &str, color: Color, track_type: TrackType) -> Self {
        Self {
            id,
            name: name.to_string(),
            color,
            track_type,
            clips: BTreeSet::new(),
            max_sample_index: 0,
            generator: None,
            order_idx: 0,
        }
    }

    /// ======================================
    /// Set Track Order
    /// Updates the order index, which dictates UI track positioning.
    /// ======================================
    pub fn set_order_idx(&mut self, new_idx: usize) {
        self.order_idx = new_idx;
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
        self.clips.iter().find(|c| c.id == *clip_id).cloned()
    }

    /// Add a new clip to the track. it will return Err if
    /// the clip type is incompatible with the track type
    pub fn add_clip(&mut self, clip: Clip) -> anyhow::Result<u32> {
        let is_valid = match (&self.track_type, &clip.source) {
            (TrackType::Audio, Some(DawSource::Audio(_))) => true,
            (TrackType::Midi, Some(DawSource::Midi { .. })) => true,
            (TrackType::Automation, Some(DawSource::Automation(_))) => true,
            // Allow Automation on Audio/Midi tracks? usually yes, but strictly speaking:
            _ => false,
        };

        if is_valid {
            // Calculate potential new max index BEFORE moving clip (in native units)
            let clip_end = clip.time.start_time_raw() + clip.time.loop_length_raw();

            // 1. Wrap in Arc immediately
            let clip_arc = Arc::new(clip);

            // 2. COW: Get mutable access to the vector
            let clips_set = &mut self.clips;

            clips_set.insert(clip_arc);

            // update the max sample index
            if clip_end > self.max_sample_index as u64 {
                self.max_sample_index = clip_end as u32;
            }

            // Return the end sample of this new clip
            return Ok(clip_end as u32);
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
                .map(|c| (c.time.start_time_raw() + c.time.loop_length_raw()) as u32)
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
            Some(DawSource::Audio(source_id)) => {
                if !is_generator {
                    source_id != &source_id_u32
                } else {
                    true
                }
            }
            Some(DawSource::Midi { .. }) => true,
            Some(DawSource::Automation(_)) => true,
            None => true,
        });
    }

    /// Optimized for adding multiple clips (e.g., Paste / Duplicate).
    pub fn add_clips_bulk(&mut self, new_clips: &[Arc<Clip>]) {
        let clips_vec = &mut self.clips;
        clips_vec.extend(new_clips.iter().cloned());

        self.max_sample_index = clips_vec
            .iter()
            .map(|c| (c.time.start_time_raw() + c.time.loop_length_raw()) as u32)
            .max()
            .unwrap_or(0);
    }

    pub fn update_max_sample_index(&mut self, bpm: f32, sample_rate: u32) {
        self.max_sample_index = self
            .clips
            .iter()
            .map(|c| c.time.end_samples(bpm, sample_rate) as u32) // Perfectly accurate!
            .max()
            .unwrap_or(0);
    }

    pub fn slice_clip(
        &mut self,
        clip_id: &ClipId,
        cut_point: u64,
        clip_counter: &mut u32,
    ) -> anyhow::Result<(Clip, Clip)> {
        let clip_arc = self.get_clip(clip_id).ok_or_else(|| {
            anyhow::anyhow!("Clip ID {:?} not found in track {:?}", clip_id, self.id)
        })?;

        let clip_start = clip_arc.time.start_time_raw();
        let clip_length = clip_arc.time.loop_length_raw();
        let clip_offset = clip_arc.time.offset_start_raw();

        if cut_point > clip_start && cut_point < clip_start + clip_length {
            // Remove using the exact Arc reference
            self.clips.remove(&clip_arc);

            let clip = clip_arc.as_ref();
            let first_length = cut_point - clip_start;
            let second_length = clip_length - first_length;
            let second_offset = clip_offset + first_length;

            // Create left clip
            let mut left_clip = clip.clone();
            left_clip.id = *clip_id; // Retain original ID
            match &mut left_clip.time {
                ClipTimeUnit::Samples { loop_length, .. } => *loop_length = first_length,
                ClipTimeUnit::Ticks { loop_length, .. } => *loop_length = first_length as u32,
            }
            self.clips.insert(Arc::new(left_clip.clone()));

            // Create right clip
            let mut right_clip = clip.clone();
            right_clip.id = ClipId::next(clip_counter);
            match &mut right_clip.time {
                ClipTimeUnit::Samples {
                    start_time,
                    loop_length,
                    offset_start,
                } => {
                    *start_time = cut_point;
                    *loop_length = second_length;
                    *offset_start = second_offset;
                }
                ClipTimeUnit::Ticks {
                    start_time,
                    loop_length,
                    offset_start,
                } => {
                    *start_time = cut_point as u32;
                    *loop_length = second_length as u32;
                    *offset_start = second_offset as u32;
                }
            }
            self.clips.insert(Arc::new(right_clip.clone()));

            log::info!("Successfully cut the clip");
            Ok((left_clip, right_clip))
        } else {
            return Err(anyhow::anyhow!("Cannot cut clip outside its boundaries"));
        }
    }
}

impl ApplicationState {
    /// ======================================
    /// Update Track Order
    /// Updates the order_idx of a specific track by ID
    /// ======================================
    pub fn update_track_order(&mut self, track_id: TrackId, new_idx: usize) -> anyhow::Result<()> {
        if let Some(track_arc) = self.tracks.get_mut(&track_id) {
            Arc::make_mut(track_arc).set_order_idx(new_idx);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Track {:?} not found", track_id))
        }
    }

    pub fn add_new_audio_track(&mut self) -> Arc<AudioTrack> {
        let new_track_id = TrackId::next(&mut self.track_counter);
        let track_order = self.tracks.len();
        let new_track = AudioTrack {
            track_type: TrackType::Audio,
            id: new_track_id,
            name: format!("Track {}", new_track_id),
            order_idx: track_order,
            ..Default::default()
        };
        let track_arc = Arc::new(new_track);
        self.tracks.insert(new_track_id, track_arc.clone());

        // Create a corresponding mixer channel and default routing
        self.mixer
            .channels
            .insert(new_track_id, Arc::new(MixerChannel::default()));
        self.mixer.add_track_default_routing(new_track_id);
        track_arc
    }

    /// Add a new MIDI track with a generator by its registry ID.
    pub fn add_new_midi_track_with_generator_id(
        &mut self,
        registry_id: u32,
    ) -> anyhow::Result<Arc<AudioTrack>> {
        let gen_id = GeneratorId::next(&mut self.generator_counter);
        let track_id = TrackId::next(&mut self.track_counter);

        // Create the plugin via registry using ID
        let (generator_plugin, generator_name) = {
            let registry = ctx().plugin_registry.read();

            if let Some((generator_box, name)) = registry.create_generator_by_id(registry_id) {
                (generator_box, name)
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

        // Create plugin instance descriptor with registry ID
        let plugin_instance = PluginInstance::new_with_id(registry_id, &generator_name);

        let generator = GeneratorInstance {
            id: gen_id,
            instance_type: GeneratorInstanceType::Plugin(plugin_instance),
        };
        self.generator_pool
            .insert(gen_id, Arc::new(generator.clone()));

        let track_order = self.tracks.len();
        let new_track = AudioTrack {
            track_type: TrackType::Midi,
            id: track_id,
            name: generator_name.clone(),
            color: Color::new_from_string("#FF8A65").unwrap_or(Color::default()),
            generator: Some(generator),
            order_idx: track_order,
            ..Default::default()
        };

        let track_arc = Arc::new(new_track);
        self.tracks.insert(track_id, track_arc.clone());

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
        Ok(track_arc)
    }

    pub fn add_new_automation_track_from_bus(
        &mut self,
        bus_id: BusId,
        automation_target: AutomationTarget,
    ) -> anyhow::Result<Arc<AutomationLane>> {
        let track_id = TrackId::next(&mut self.track_counter);

        // Find the Bus
        let bus = self
            .get_mixer_state()
            .buses
            .get(&bus_id)
            .ok_or_else(|| anyhow::anyhow!("Cannot find the mixer bus"))?;

        let (current_value, min, max) = match &automation_target {
            AutomationTarget::BusVolume(_) => {
                let curr_value = bus.channel.volume.get_base();
                let (min_val, max_val) = {
                    if let ParamBounds::Continuous { min, max, .. } = bus.channel.volume.bounds {
                        (min, max)
                    } else {
                        (-60.0, 6.0)
                    }
                };
                (curr_value, min_val, max_val)
            }
            AutomationTarget::BusPan(_) => {
                let curr_value = bus.channel.pan.get_base();
                let (min_val, max_val) = {
                    if let ParamBounds::Continuous { min, max, .. } = bus.channel.pan.bounds {
                        (min, max)
                    } else {
                        (-60.0, 6.0)
                    }
                };
                (curr_value, min_val, max_val)
            }
            AutomationTarget::BusPluginParam { .. } => (0.5, 0.0, 1.0), // Standard fallback for plugins
            _ => {
                return Err(anyhow::anyhow!("Not an Bus Target"));
            }
        };

        // Create a descriptive label for the track
        let label = match &automation_target {
            AutomationTarget::BusVolume(_) => format!("{} - Volume", bus.name),
            AutomationTarget::BusPan(_) => format!("{} - Pan", bus.name),
            AutomationTarget::BusPluginParam { .. } => format!("{} - Plugin", bus.name),
            _ => bus.name.clone(),
        };

        // Initiate the points in the first and last max sample index with the current value
        // Fallback to ~4 seconds if the project is completely empty
        let end_sample = self.max_sample_index.max(
            (self.audio_config.sample_rate * 4) / ((self.transport.bpm * 60.0).floor() as u32),
        );

        let new_automation_lane = self.add_automation_lane_return_lane(
            automation_target,
            label.clone(),
            min,
            max,
            current_value,
        )?;

        let track_order = self.tracks.len();
        // Create an automation track explicitly for the timeline (because it is a Bus target)
        let mut new_track = AudioTrack {
            track_type: TrackType::Automation,
            id: track_id,
            name: label.clone(),
            color: Color::new_from_rgb(150, 150, 150),
            order_idx: track_order,
            ..Default::default()
        };

        // Assign it to the AutomationId by creating an empty clip spanning the project timeline
        let automation_clip = Clip {
            id: ClipId::next(&mut self.clip_counter),
            name: label,
            source: Some(DawSource::Automation(new_automation_lane.id)),
            time: ClipTimeUnit::Ticks {
                start_time: 0,
                loop_length: end_sample,
                offset_start: 0,
            },
        };

        // Add the clip using your existing validation (which correctly allows Automation clips on Automation tracks)
        new_track.add_clip(automation_clip)?;

        // Finally, add the dedicated track into the timeline's main collection
        self.tracks.insert(track_id, Arc::new(new_track));

        Ok(new_automation_lane)
    }

    /// Remove a track and clean up its mixer channel, routing, generator, and automation lanes.
    pub fn remove_track(&mut self, track_id: TrackId) -> anyhow::Result<RemovedTrackType> {
        let mut deleted_track_type = RemovedTrackType::Audio;
        // Get the generator ID before removing the track
        let generator_id = self
            .tracks
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
            deleted_track_type = RemovedTrackType::Midi;
        }

        // Remove all automation lanes for this track
        self.remove_automation_lanes_for_track(track_id);

        self.update_max_sample_index();

        Ok(deleted_track_type)
    }
}
