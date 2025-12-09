// src/core/project/mod.rs

use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};

use crate::{api::track, core::track::audio_waveform::AudioWaveform};

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
    pub pattern_pool: HashMap<u32, Arc<Pattern>>,
    pub pattern_counter: u32,

    // Tracks contain Clips, but Clips are just "Containers"
    pub tracks: HashMap<u32, Arc<KarbeatTrack>>,
    pub track_counter: u32,

    // Counter for clips
    pub clip_counter: u32,

    // Max samples index in the timeline
    pub max_sample_index: u64,

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
    pub track_type: TrackType,
    pub clips: Arc<BTreeSet<Arc<Clip>>>,
    pub max_sample_index: u64,

    // This tells the engine: "Any audio/midi generated on this track
    // gets sent to Mixer Channel X".
    pub target_mixer_channel_id: Option<u32>,
}

impl Default for KarbeatTrack {
    fn default() -> Self {
        Self {
            id: Default::default(),
            name: Default::default(),
            color: Default::default(),
            track_type: TrackType::Audio,
            clips: Arc::new(BTreeSet::new()),
            target_mixer_channel_id: None,
            max_sample_index: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Clone)]
pub enum KarbeatSource {
    /// Points to an AudioWaveform
    Audio(Arc<AudioWaveform>),

    /// Points to a Pattern
    Midi(Arc<Pattern>),

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

#[derive(Serialize, Deserialize, Clone)]
pub struct TransportState {
    pub is_playing: bool,
    pub is_recording: bool,
    pub is_looping: bool,
    pub playhead_position_samples: u64,
    pub loop_start_samples: u64,
    pub loop_end_samples: u64,

    // general state
    pub bpm: f32,
    pub time_signature: (u8, u8),

    // Beat and bar tracker
    pub beat_tracker: usize,
    pub bar_tracker: usize,
}

impl Default for TransportState {
    fn default() -> Self {
        Self {
            bpm: 67.0,
            time_signature: (4, 4),
            is_playing: Default::default(),
            is_recording: Default::default(),
            is_looping: Default::default(),
            playhead_position_samples: Default::default(),
            loop_start_samples: Default::default(),
            loop_end_samples: Default::default(),
            beat_tracker: 0,
            bar_tracker: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Pattern {
    pub id: u32,
    pub name: String,
    pub length_bars: u32,

    pub notes: HashMap<u32, Vec<Note>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Note {
    pub start_tick: u64,
    pub duration: u64,
    pub key: u8, // 0 - 127 MIDI key
    pub velocity: u8,

    pub probability: f32,
    pub micro_offset: i8,
    pub mute: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Clip {
    pub name: String,
    pub id: u32,
    /// Refer to where it sits on the global timeline
    pub start_time: u64,
    pub source: KarbeatSource,
    pub offset_start: u64, // currently this does nothing since we set it always to 0
    pub loop_length: u64,  // Refer to length of the entire clip when not shrinked
}

impl PartialEq for Clip {
    fn eq(&self, other: &Self) -> bool {
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

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct MixerState {
    // Map Track ID -> Mixer Channel
    pub channels: HashMap<u32, Arc<MixerChannel>>,
    pub master_bus: Arc<MixerChannel>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MixerChannel {
    pub volume: f32, // 0.0 to 1.0 (or dB)
    pub pan: f32,    // -1.0 to 1.0
    pub mute: bool,
    pub solo: bool,
    pub inverted_phase: bool,

    // The effects chain (EQ, Compressor) comes AFTER the generator
    pub effects: Arc<Vec<PluginInstance>>,
}

impl Default for MixerChannel {
    fn default() -> Self {
        Self {
            volume: 0.0,
            pan: 0.0,
            mute: Default::default(),
            solo: Default::default(),
            inverted_phase: Default::default(),
            effects: Arc::new(Vec::new()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum GeneratorInstance {
    // A Synth (Internal or VST)
    Plugin(PluginInstance),

    // A Sampler (Plays a file from AssetLibrary)
    Sampler { asset_id: u32, root_note: u8 },

    // Audio Input (Microphone / Line In)
    AudioInput { device_channel_index: u32 },
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
    pub sample_paths: HashMap<u32, PathBuf>,
    pub next_id: u32,
    #[serde(skip)]
    pub source_map: HashMap<u32, Arc<AudioWaveform>>,
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
            sample_rate: 44100,
            buffer_size: 256,
            cpu_load: Default::default(),
        }
    }
}

impl KarbeatTrack {
    pub fn clips(&self) ->&BTreeSet<Arc<Clip>> {
        return self.clips.as_ref();
    }

    pub fn clips_to_vec(&self) -> Vec<Arc<Clip>> {
        self.clips.iter().cloned().collect()
    }

    pub fn track_type(&self) -> &TrackType {
        return &self.track_type;
    }
    pub fn add_clip(&mut self, clip: Clip) -> Option<u64> {
        let is_valid = match (&self.track_type, &clip.source) {
            (TrackType::Audio, KarbeatSource::Audio(_)) => true,
            (TrackType::Midi, KarbeatSource::Midi(_)) => true,
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
            let clips_set = Arc::make_mut(&mut self.clips);

            // 4. Insert pointer (Cheap!)
            clips_set.insert(clip_arc);

            // update the max sample index
            if clip_end_sample > self.max_sample_index {
                self.max_sample_index = clip_end_sample;
            }

            // Return the end sample of this new clip
            return Some(clip_end_sample);
        } else {
            // In a real app, maybe return Result<Error>
            eprintln!("Warning: Mismatched Clip Source for Track Type");
            return None;
        }
    }

    /// Remove the clip, change max_index_sample if the deleted clip are the latest end sample index
    pub fn remove_clip(&mut self, clip_id: u32) -> bool {
        let clips_set = Arc::make_mut(&mut self.clips);

        let initial_len = clips_set.len();
        
        clips_set.retain(|c| c.id != clip_id);

        if clips_set.len() < initial_len {
            // Recalculate max sample index if something was removed
            self.max_sample_index = clips_set
                .iter()
                .map(|c| c.start_time + c.loop_length)
                .max()
                .unwrap_or(0);

            true
        } else {
            false
        }
    }

    /// Optimized for adding multiple clips (e.g., Paste / Duplicate).
    pub fn add_clips_bulk(&mut self, new_clips: Vec<Arc<Clip>>) {
        let clips_vec = Arc::make_mut(&mut self.clips);
        clips_vec.extend(new_clips);

        self.max_sample_index = clips_vec
            .iter()
            .map(|c| c.start_time + c.loop_length)
            .max()
            .unwrap_or(0);
    }

    pub fn update_max_sample_index(&mut self) {
        self.clips.iter().map(|c| c.start_time + c.loop_length).max().unwrap_or(0);
    }
}

impl ApplicationState {
    pub fn add_clip_to_track(&mut self, track_id: u32, clip: Clip) {
        // 1. Get the track
        if let Some(track_arc) = self.tracks.get_mut(&track_id) {
            // 2. COW: Get mutable track
            let track = Arc::make_mut(track_arc);

            // 3. Add Clip & Check bounds
            // We pass the Clip by value. The track takes ownership and wraps it in Arc.
            if let Some(_) = track.add_clip(clip) {
                // 4. Update Global Max (Cheap u64 comparison)
                self.update_max_sample_index();
            }
        }
    }

    pub fn delete_clip_from_track(&mut self, track_id: u32, clip_id: u32) {
        if let Some(track_arc) = self.tracks.get_mut(&track_id) {
            let track = Arc::make_mut(track_arc);
            if track.remove_clip(clip_id) {
                // Only recompute global max if that track actually changed
                self.update_max_sample_index();
            }
        }
    }

    pub fn update_max_sample_index(&mut self) {
         self.max_sample_index = self
            .tracks
            .values()
            .map(|t| t.max_sample_index)
            .max()
            .unwrap_or(0);
    }
}
