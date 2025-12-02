// src/core/project/mod.rs

use std::{cmp::Ordering, collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
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
    pub asset_library: AssetLibrary,
    // All musical data lives here. The timeline just references these.
    pub pattern_pool: HashMap<u32, Pattern>,

    // Tracks contain Clips, but Clips are just "Containers"
    pub tracks: HashMap<u32, KarbeatTrack>,

    // ========== NON-SERIALIZABLE SESSION DATA ===============
    // These fields are marked to be skipped during Save/Load
    #[serde(skip)]
    pub session: SessionState,
    
    #[serde(skip)]
    pub audio_config: AudioHardwareConfig,
}


#[derive(Serialize, Deserialize, Clone)]
pub struct KarbeatTrack {
    pub id: u32,
    pub name: String,
    pub color: String,
    track_type: TrackType,
    clips: Vec<Clip>,

    // This tells the engine: "Any audio/midi generated on this track 
    // gets sent to Mixer Channel X".
    pub target_mixer_channel_id: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum TrackType {
    Audio,
    Midi,
    Bus
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectMetadata {
    pub name: String,
    pub author: String,
    pub version: String,
    pub created_at: u64,
    pub bpm: f32,
    pub time_signature: (u8, u8)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TransportState {
    pub is_playing: bool,
    pub is_recording: bool,
    pub is_looping: bool,
    pub playhead_position_samples: u64,
    pub loop_start_samples: u64,
    pub loop_end_samples: u64
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Pattern {
    pub id: u32,
    pub name: String,
    pub length_bars: u32,

    pub notes: HashMap<u32, Vec<Note>>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Note {
    pub start_tick: u64,
    pub duration: u64,
    pub key: u8, // 0 - 127 MIDI key
    pub velocity: u8,
    
    pub probability: f32,
    pub micro_offset: i8,
    pub mute: bool
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Clip {
    pub id: u32,
    /// Refer to where it sits on the global timeline
    pub start_time: u64,
    pub pattern_id: Option<u32>,
    pub asset_id: Option<u32>,
    pub offset_start: u64,
    pub loop_length: u64
}

impl PartialEq for Clip {
    fn eq(&self, other: &Self) -> bool {
        // Clips are equal if they have the same start_time and id
        self.start_time == other.start_time && self.id == other.id
    }
}

impl Eq for Clip {}

impl PartialOrd for Clip {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Primary ordering by start_time, then by id for tie-breaking
        match self.start_time.cmp(&other.start_time) {
            Ordering::Equal => Some(self.id.cmp(&other.id)),
            ordering => Some(ordering),
        }
    }
}

impl Ord for Clip {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MixerState {
    // Map Track ID -> Mixer Channel
    pub channels: HashMap<u32, MixerChannel>,
    pub master_bus: MixerChannel,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MixerChannel {
    pub volume: f32, // 0.0 to 1.0 (or dB)
    pub pan: f32,    // -1.0 to 1.0
    pub mute: bool,
    pub solo: bool,
    pub inverted_phase: bool,
    pub generator: Option<GeneratorInstance>,

    // The effects chain (EQ, Compressor) comes AFTER the generator
    pub effects: Vec<PluginInstance>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum GeneratorInstance {
    // A Synth (Internal or VST)
    Plugin(PluginInstance),
    
    // A Sampler (Plays a file from AssetLibrary)
    Sampler { asset_id: u32, root_note: u8 },
    
    // Audio Input (Microphone / Line In)
    AudioInput { device_channel_index: u32 }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginInstance {
    pub id: u32,
    pub name: String,
    pub internal_type: String, // e.g., "EQ_3BAND", "COMPRESSOR"
    pub bypass: bool,
    pub parameters: HashMap<u32, f32>, // Param ID -> Value
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AssetLibrary {
    // Map ID -> File Path on Disk
    // When loading a project, we check if these paths still exist
    pub samples: HashMap<u32, PathBuf>, 
}

// ========= NON-SAVED STATE (Runtime Only) =================

#[derive(Default, Clone)]
pub struct SessionState {
    // What is the user clicking on right now?
    pub selected_track_id: Option<u32>,
    pub selected_clip_id: Option<u32>,
    
    // Undo/Redo Stack
    // We don't save this usually, or we save it separately
    // pub undo_stack: Vec<AudioCommand>, 
    // pub redo_stack: Vec<AudioCommand>,

    // Clipboard for Copy/Paste
    pub clipboard: Option<Clip>,
    
    // UI Zoom Levels (Seconds per pixel)
    pub horizontal_zoom: f32,
}

#[derive(Clone, Default)]
pub struct AudioHardwareConfig {
    pub selected_input_device: String,
    pub selected_output_device: String,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub cpu_load: f32, // For UI monitoring
}

impl KarbeatTrack {
    pub fn clips(&self) -> &[Clip]  {
        return &self.clips;
    }
    pub fn track_type(&self) -> &TrackType {
        return &self.track_type;
    }
    pub fn add_clip(&mut self, clip: Clip) {
        let pos = match self.clips.binary_search(&clip) {
            Ok(index) => index,
            Err(index) => index,
        };

        self.clips.insert(pos, clip);
    }

    /// Optimized for adding multiple clips (e.g., Paste / Duplicate).
    pub fn add_clips_bulk(&mut self, new_clips: Vec<Clip>) {
        self.clips.extend(new_clips);
        self.clips.sort(); 
    }
}




