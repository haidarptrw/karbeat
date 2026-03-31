use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use karbeat_utils::define_id;
// src/core/project/mod.rs

pub mod automation;
pub mod clip;
pub mod clipboard;
pub mod generator;
pub mod mixer;
pub mod plugin;
pub mod track;
pub mod transport;

use std::{
    cmp::Ordering,
    sync::Arc,
};

use hashbrown::HashMap;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

pub use clip::{Clip, ClipId};
pub use clipboard::ClipboardContent;
pub use generator::{GeneratorId, GeneratorInstance, GeneratorInstanceType};
pub use plugin::{instance::PluginInstance, KarbeatPlugin};
pub use track::{
    audio_waveform::{AudioSourceId, AudioWaveform},
    midi::{Pattern, PatternId},
    KarbeatTrack, TrackId, TrackType,
};
pub use transport::TransportState;

use crate::{
    core::project::{
        automation::{AutomationId, AutomationLane, AutomationPoint, AutomationTarget},
        mixer::MixerState,
    },
    
};

define_id!(SourceId);
define_id!(NoteId);

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct ApplicationState {
    // Things store inside ApplicationState
    // - Project Metadata
    // - Mixer
    // - Tracks timeline
    // - Settings
    //
    // - File explorer to access resources
    // - Audio related stuff (device, source, playback etc)
    pub metadata: ProjectMetadata,
    pub mixer: MixerState,
    pub transport: TransportState,
    pub asset_library: Arc<AssetLibrary>,

    // All musical data lives here. The timeline just references these.
    pub pattern_pool: IndexMap<PatternId, Arc<Pattern>>,
    pub pattern_counter: u32,

    // Generator sources
    pub generator_pool: IndexMap<GeneratorId, Arc<GeneratorInstance>>,
    pub generator_counter: u32,

    // Tracks contain Clips, but Clips are just "Containers"
    pub tracks: IndexMap<TrackId, Arc<KarbeatTrack>>,
    pub track_counter: u32,

    // Automation lanes pool (lives at the same level as tracks/patterns/generators)
    pub automation_pool: IndexMap<AutomationId, Arc<AutomationLane>>,
    pub automation_counter: u32,

    // Counter for clips
    pub clip_counter: u32,

    // Max samples index in the timeline
    pub max_sample_index: u32,

    // ========== NON-SERIALIZABLE SESSION DATA ===============
    // These fields are marked to be skipped during Save/Load
    #[serde(skip)]
    pub audio_config: AudioHardwareConfig,

    #[serde(skip)]
    pub clipboard: ClipboardContent,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum KarbeatSource {
    /// Points to an AudioWaveform
    Audio(AudioSourceId),

    /// Points to Generators paired with Patterns
    /// Each entry in the vector is a (GeneratorInstance, Pattern) pair.
    /// This allows a single clip to trigger multiple generators (layering) or just one.
    Midi(PatternId),

    /// Points to an Automation ID (Future implementation)
    Automation(u32),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProjectMetadata {
    pub name: String,
    pub author: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
}

impl Default for ProjectMetadata {
    fn default() -> Self {
        Self {
            name: "Untitled".to_string(),
            author: Default::default(),
            version: Default::default(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Note {
    pub id: NoteId,
    pub start_tick: u64,
    pub duration: u64,
    pub key: u8, // 21 - 127 MIDI key (Keep LB at A0)
    pub velocity: u8,

    pub probability: f32,
    pub micro_offset: i8,
    pub mute: bool,
}

impl PartialEq for Note {
    fn eq(&self, other: &Self) -> bool {
        self.start_tick == other.start_tick
    }
}

impl Eq for Note {}

impl PartialOrd for Note {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Note {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.start_tick.cmp(&other.start_tick) {
            Ordering::Equal => {
                // Secondary: if start times are equal, sort by key (pitch)
                match self.key.cmp(&other.key) {
                    Ordering::Equal => {
                        // Tertiary: if keys are equal, sort by velocity
                        self.velocity.cmp(&other.velocity)
                    }
                    other => other,
                }
            }
            other => other,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AssetLibrary {
    pub next_id: u32,
    #[serde(skip)]
    pub source_map: HashMap<AudioSourceId, Arc<AudioWaveform>>,
}

impl Default for AssetLibrary {
    fn default() -> Self {
        Self {
            next_id: 1,
            source_map: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioHardwareConfig {
    pub selected_input_device: String,
    pub selected_output_device: String,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub cpu_load: f32, // For UI monitoring
}

impl Default for AudioHardwareConfig {
    fn default() -> Self {
        Self {
            selected_input_device: Default::default(),
            selected_output_device: Default::default(),
            sample_rate: 48000,
            buffer_size: 256,
            cpu_load: Default::default(),
        }
    }
}

impl ApplicationState {
    pub fn update_max_sample_index(&mut self) {
        self.max_sample_index = self
            .tracks
            .values_mut()
            .map(|t| {
                let track_mut = Arc::make_mut(t);
                track_mut.update_max_sample_index();
                track_mut.max_sample_index
            })
            .max()
            .unwrap_or(0);
    }

    /// Deletes an audio source and removes all clips referencing it.
    pub fn remove_audio_source(
        &mut self,
        source_id: AudioSourceId,
    ) -> anyhow::Result<AudioSourceId> {
        // we check whether the source exists
        let library = Arc::make_mut(&mut self.asset_library);

        if library.source_map.remove(&source_id).is_none() {
            return Err(anyhow!("Source does not exist"));
        }

        // cascade delete
        for track_arc in self.tracks.values_mut() {
            let track = Arc::make_mut(track_arc);
            track.remove_clip_by_source_id(source_id, false);
        }

        self.update_max_sample_index();

        Ok(source_id)
    }

    // =========================================================================
    // Automation Pool Management
    // =========================================================================

    /// Add an automation lane to the pool.
    pub fn add_automation_lane(
        &mut self,
        target: AutomationTarget,
        label: impl Into<String>,
        min: f32,
        max: f32,
        default_value: f32,
    ) -> anyhow::Result<AutomationId> {
        // Prevent duplicate lanes for the same target
        if self.automation_pool.values().any(|l| l.target == target) {
            return Err(anyhow!("Automation lane for this target already exists"));
        }

        let lane_id = AutomationId::next(&mut self.automation_counter);
        let lane = AutomationLane::new(lane_id, target, label, min, max, default_value);
        self.automation_pool.insert(lane_id, Arc::new(lane));

        log::info!("Added automation lane {:?}", lane_id);
        Ok(lane_id)
    }

    /// Remove an automation lane from the pool by its ID.
    pub fn remove_automation_lane(&mut self, lane_id: AutomationId) -> anyhow::Result<()> {
        if self.automation_pool.shift_remove(&lane_id).is_none() {
            return Err(anyhow!("Automation lane {:?} not found", lane_id));
        }

        log::info!("Removed automation lane {:?}", lane_id);
        Ok(())
    }

    /// Add an automation point to a lane.
    pub fn add_automation_point(
        &mut self,
        lane_id: AutomationId,
        point: AutomationPoint,
    ) -> anyhow::Result<()> {
        let lane_arc = self
            .automation_pool
            .get_mut(&lane_id)
            .ok_or_else(|| anyhow!("Automation lane {:?} not found", lane_id))?;

        let lane = Arc::make_mut(lane_arc);
        lane.add_point(point);
        Ok(())
    }

    /// Remove an automation point from a lane by its index.
    pub fn remove_automation_point(
        &mut self,
        lane_id: AutomationId,
        point_index: usize,
    ) -> anyhow::Result<AutomationPoint> {
        let lane_arc = self
            .automation_pool
            .get_mut(&lane_id)
            .ok_or_else(|| anyhow!("Automation lane {:?} not found", lane_id))?;

        let lane = Arc::make_mut(lane_arc);
        lane.remove_point(point_index).ok_or_else(|| {
            anyhow!(
                "Point index {} out of bounds (lane has {} points)",
                point_index,
                lane.points.len()
            )
        })
    }

    /// Update an automation point (move in time and/or value).
    pub fn update_automation_point(
        &mut self,
        lane_id: AutomationId,
        point_index: usize,
        time_ticks: u32,
        value: f32,
    ) -> anyhow::Result<()> {
        let lane_arc = self
            .automation_pool
            .get_mut(&lane_id)
            .ok_or_else(|| anyhow!("Automation lane {:?} not found", lane_id))?;

        let lane = Arc::make_mut(lane_arc);
        if !lane.update_point(point_index, time_ticks, value) {
            return Err(anyhow!(
                "Point index {} out of bounds (lane has {} points)",
                point_index,
                lane.points.len()
            ));
        }

        Ok(())
    }

    /// Get all automation lanes that reference a specific track.
    pub fn get_automation_lanes_for_track(&self, track_id: TrackId) -> Vec<Arc<AutomationLane>> {
        self.automation_pool
            .values()
            .filter(|l| l.target.references_track(track_id))
            .cloned()
            .collect()
    }

    /// Remove all automation lanes that reference a track (used when deleting tracks).
    pub fn remove_automation_lanes_for_track(&mut self, track_id: TrackId) {
        self.automation_pool
            .retain(|_, lane| !lane.target.references_track(track_id));
    }
}
