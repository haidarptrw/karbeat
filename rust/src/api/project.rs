use std::{collections::HashMap, ops::Deref};

use serde::Serialize;

use crate::{
    broadcast_state_change,
    core::{
        file_manager::loader::AudioLoader,
        project::{
            clip::Clip,
            generator::{GeneratorInstance, GeneratorInstanceType},
            track::{audio_waveform::AudioWaveform, KarbeatTrack, TrackType},
            transport::TransportState,
            KarbeatSource, ProjectMetadata, SessionState,
        },
    },
    utils::lock::{get_app_read, get_app_write},
};

pub struct UiTrack {
    pub id: u32,
    pub name: String,
    pub track_type: TrackType,
    pub clips: Vec<UiClip>,
    pub generator_id: Option<u32>,
}

impl From<&KarbeatTrack> for UiTrack {
    fn from(value: &KarbeatTrack) -> Self {
        let generator_id = match &value.generator {
            Some(gen_instance) => Some(gen_instance.id.to_u32()),
            None => None,
        };
        Self {
            id: value.id.to_u32(),
            name: value.name.clone(),
            track_type: value.track_type.clone(),
            clips: value
                .clips_to_vec()
                .iter()
                .map(|c| UiClip::from(c.deref()))
                .collect(),
            generator_id,
        }
    }
}

#[derive(Clone)]
pub struct UiClip {
    pub name: String,
    pub id: u32,
    pub start_time: u32,
    pub source: UiClipSource,
    pub offset_start: u32,
    pub loop_length: u32,
}

#[derive(Clone)]
pub enum UiClipSource {
    Audio { source_id: u32 },
    Midi { pattern_id: u32 },
    None, // represent clip with empty source, this is placeholder, as this will be removed when I already implement MIDI Pattern and automation
}

impl From<&Clip> for UiClip {
    fn from(value: &Clip) -> Self {
        // Map source to either AudioWaveform, midi
        let source = match &value.source {
            KarbeatSource::Audio(source_id) => UiClipSource::Audio {
                source_id: source_id.to_u32(),
            },
            KarbeatSource::Midi(pattern_id) => UiClipSource::Midi {
                pattern_id: pattern_id.to_u32(),
            },
            _ => UiClipSource::None,
        };
        Self {
            name: value.name.clone(),
            id: value.id.to_u32(),
            start_time: value.start_time,
            source: source,
            offset_start: value.offset_start,
            loop_length: value.loop_length,
        }
    }
}

// UI Data Structure for Audio Waveform window information (to change vol, pitch fine tune, normalization, panning, adsr envelope,
// play the audio when pressing the waveform etc)

#[derive(Clone, Debug, Serialize)]
pub struct AudioWaveformUiForSourceList {
    pub name: String,
    pub muted: bool,
    pub sample_rate: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct AudioWaveformUiForAudioProperties {
    pub preview_buffer: Vec<i8>, // Quantized i8 samples (-127..127) for waveform display
    pub file_path: String,
    pub name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration: f64,
    pub root_note: u8,
    pub fine_tune: i16,
    pub trim_start: u32,
    pub trim_end: u32,
    pub is_looping: bool,
    pub normalized: bool,
    pub muted: bool, // this only affects when play stream, not when doing preview sound
}

pub struct AudioWaveformUiForClip {
    pub name: String,
    pub preview_buffer: Vec<i8>,
    pub sample_rate: u32,
}

impl From<&AudioWaveform> for AudioWaveformUiForSourceList {
    fn from(value: &AudioWaveform) -> Self {
        Self {
            name: value.name.clone(),
            muted: value.muted,
            sample_rate: value.sample_rate,
        }
    }
}

impl From<&AudioWaveform> for AudioWaveformUiForAudioProperties {
    fn from(value: &AudioWaveform) -> Self {
        let preview_buffer: Vec<i8> = value
            .buffer
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 127.0) as i8)
            .collect();
        Self {
            preview_buffer,
            file_path: value.file_path.clone(),
            name: value.name.clone(),
            sample_rate: value.sample_rate,
            channels: value.channels,
            duration: value.duration,
            root_note: value.root_note,
            fine_tune: value.fine_tune,
            trim_start: value.trim_start,
            trim_end: value.trim_end,
            is_looping: value.is_looping,
            normalized: value.normalized,
            muted: value.muted,
        }
    }
}

impl From<&AudioWaveform> for AudioWaveformUiForClip {
    fn from(value: &AudioWaveform) -> Self {
        let preview_buffer: Vec<i8> = value
            .buffer
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 127.0) as i8)
            .collect();
        Self {
            preview_buffer,
            name: value.name.clone(),
            sample_rate: value.sample_rate,
        }
    }
}
// ============================================================
// =================== GENERATOR INSTANCE =====================
// ============================================================
pub struct UiGeneratorInstance {
    pub id: u32,
    pub name: String,
    pub parameters: HashMap<u32, f32>,
}

impl From<&GeneratorInstance> for UiGeneratorInstance {
    fn from(generator_instance: &GeneratorInstance) -> Self {
        match &generator_instance.instance_type {
            GeneratorInstanceType::Plugin(plugin_instance) => Self {
                id: generator_instance.id.to_u32(),
                name: plugin_instance.name.clone(),
                parameters: plugin_instance.parameters.clone(),
            },
            GeneratorInstanceType::Sampler { .. } => {
                Self {
                    id: generator_instance.id.to_u32(),
                    name: "Sampler".to_string(),
                    parameters: HashMap::new(), // Add sampler params later if needed
                }
            }
            GeneratorInstanceType::AudioInput { .. } => Self {
                id: generator_instance.id.to_u32(),
                name: "Audio Input".to_string(),
                parameters: HashMap::new(),
            },
        }
    }
}

// ============================================================
// =====================SESSION STATE==========================
// ============================================================

pub struct UiSessionState {
    // Track-locked multi-selection
    pub selected_track_id: Option<u32>,
    pub selected_clip_ids: Vec<u32>,

    // For piano roll navigation - most recently interacted clip
    pub focus_clip_id: Option<u32>,

    // Optional override for piano roll preview generator
    pub preview_generator_id: Option<u32>,
}

impl From<&SessionState> for UiSessionState {
    fn from(session: &SessionState) -> Self {
        Self {
            selected_track_id: session.selected_track_id.map(|id| id.to_u32()),
            selected_clip_ids: session
                .selected_clip_ids
                .iter()
                .map(|id| id.to_u32())
                .collect(),
            focus_clip_id: session.focus_clip_id.map(|id| id.to_u32()),
            preview_generator_id: session.preview_generator_id.map(|id| id.to_u32()),
        }
    }
}

impl Into<UiSessionState> for SessionState {
    fn into(self) -> UiSessionState {
        UiSessionState {
            selected_track_id: self.selected_track_id.map(|id| id.to_u32()),
            selected_clip_ids: self
                .selected_clip_ids
                .iter()
                .map(|id| id.to_u32())
                .collect(),
            focus_clip_id: self.focus_clip_id.map(|id| id.to_u32()),
            preview_generator_id: self.preview_generator_id.map(|id| id.to_u32()),
        }
    }
}
// ============================ APIs ==================================

/// Get the current project metadata state from the backend
pub fn get_project_metadata() -> Result<ProjectMetadata, String> {
    let app = get_app_read();

    let metadata = app.metadata.clone();
    Ok(metadata)
}

/// Get the transport state from the backend
pub fn get_transport_state() -> Result<TransportState, String> {
    let app = get_app_read();

    let ts = app.transport.clone();
    Ok(ts)
}

/// Get all audio waveform source list from the backend
pub fn get_audio_source_list() -> Option<HashMap<u32, AudioWaveformUiForAudioProperties>> {
    // Read from app state
    let app = get_app_read();
    let map = app
        .asset_library
        .source_map
        .iter()
        .map(|(&id, arc_waveform)| {
            let ui = AudioWaveformUiForAudioProperties::from(arc_waveform.as_ref());
            (id.to_u32(), ui)
        })
        .collect();

    Some(map)
}

/// Get generator list used in the project
pub fn get_generator_list() -> Result<HashMap<u32, UiGeneratorInstance>, String> {
    let app = get_app_read();

    let generators = app
        .generator_pool
        .iter()
        .map(|(&id, generator_guard)| {
            let generator = generator_guard
                .read()
                .expect("Failed to read generator lock");
            let ui_gen = UiGeneratorInstance::from(&*generator);
            (id.to_u32(), ui_gen)
        })
        .collect();

    Ok(generators)
}

/// Add a new audio source to the project
///
/// ## Parameters:
/// - file_path: Path to the audio file to be added
pub fn add_audio_source(file_path: &str) {
    {
        let mut app = get_app_write();
        // Add audio source
        match app.load_audio(file_path.to_string(), None) {
            Ok(id) => {
                let Some(audio) = app.asset_library.source_map.get(&id.into()) else {
                    log::error!("[error] can't get the audiowave");
                    return;
                };

                log::info!("Sucessfully add {}", audio.name);
            }
            Err(e) => {
                log::error!("[error] failed to load the audio: {}", e);
            }
        }
    }

    broadcast_state_change();
}

/// Add new track to the track list. Throws an error, so it must handled gracefully
pub fn add_new_track(track_type: TrackType) -> Result<(), String> {
    {
        let mut app = get_app_write();
        app.add_new_track(track_type);
        log::info!("[add_new_track] successfully add new track")
    }
    broadcast_state_change();
    Ok(())
}

/// Get all tracks on the session/project.
///
/// Returns Map<u32, UiTrack> upon success, and Error when it fails
pub fn get_tracks() -> Result<HashMap<u32, UiTrack>, String> {
    let app = get_app_read();

    // convert the tracks into UI-Friendly type
    let return_data = app
        .tracks
        .iter()
        .map(|(id, track_arc)| ((*id).to_u32(), UiTrack::from(track_arc.as_ref())))
        .collect();

    Ok(return_data)
}

/// Get the newest max sample index of the project
pub fn get_max_sample_index() -> Result<u32, String> {
    let app = get_app_read();

    Ok(app.max_sample_index)
}

/// Get the session state of from the project
pub fn get_session_state() -> Result<UiSessionState, String> {
    let app = get_app_read();

    let session = UiSessionState::from(&app.session);
    Ok(session)
}
