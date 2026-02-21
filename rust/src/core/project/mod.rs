// src/core/project/mod.rs

pub mod clip;
pub mod clipboard;
pub mod generator;
pub mod mixer;
pub mod plugin;
pub mod track;
pub mod transport;

use std::{
    cmp::Ordering,
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

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

use crate::{core::project::mixer::MixerState, define_id};

define_id!(SourceId);
define_id!(NoteId);

#[derive(Serialize, Deserialize, Clone, Default)]
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
    pub pattern_pool: HashMap<PatternId, Arc<Pattern>>,
    pub pattern_counter: u32,

    // Generator sources
    pub generator_pool: HashMap<GeneratorId, Arc<RwLock<GeneratorInstance>>>,
    pub generator_counter: u32,

    // Tracks contain Clips, but Clips are just "Containers"
    pub tracks: HashMap<TrackId, Arc<KarbeatTrack>>,
    pub track_counter: u32,

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

#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectMetadata {
    pub name: String,
    pub author: String,
    pub version: String,
    pub created_at: u64,
}

impl Default for ProjectMetadata {
    fn default() -> Self {
        Self {
            name: "Untitled".to_string(),
            author: Default::default(),
            version: Default::default(),
            created_at: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Note {
    pub id: NoteId,
    pub start_tick: u64,
    pub duration: u64,
    pub key: u8, // 0 - 127 MIDI key
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

#[derive(Serialize, Deserialize, Clone)]
pub struct AssetLibrary {
    // Map ID -> File Path on Disk
    // When loading a project, we check if these paths still exist
    pub sample_paths: HashMap<AudioSourceId, PathBuf>,
    pub next_id: u32,
    #[serde(skip)]
    pub source_map: HashMap<AudioSourceId, Arc<AudioWaveform>>,
}

impl Default for AssetLibrary {
    fn default() -> Self {
        Self {
            sample_paths: HashMap::new(),
            next_id: 1, // Start IDs at 1 (0 can be null/empty)
            source_map: HashMap::new(),
        }
    }
}

#[derive(Clone)]
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
        // ASSUME: the id inside source_map and sample_paths are same based on the existing insertion logic
        let library = Arc::make_mut(&mut self.asset_library);

        if library.source_map.remove(&source_id).is_none() {
            return Err(anyhow!("Source does not exist"));
        }

        library.sample_paths.remove(&source_id);

        // cascade delete
        for track_arc in self.tracks.values_mut() {
            let track = Arc::make_mut(track_arc);
            track.remove_clip_by_source_id(source_id, false);
        }

        self.update_max_sample_index();

        Ok(source_id)
    }
}
