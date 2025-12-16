use std::{collections::HashMap, ops::Deref};

use serde::Serialize;

use crate::{
    broadcast_state_change,
    core::{
        file_manager::loader::AudioLoader,
        project::{
            Clip, GeneratorInstance, GeneratorInstanceType, KarbeatSource, KarbeatTrack,
            ProjectMetadata, SessionState, TrackType, TransportState,
        },
        track::audio_waveform::AudioWaveform,
    },
    APP_STATE,
};

pub struct UiProjectState {
    // REUSE: FRB sees this type and generates a Dart class for it
    pub metadata: ProjectMetadata,

    // TRANSFORM: Only the Tracks need specific UI handling (Vec vs HashMap)
    pub tracks: Vec<UiTrack>,
}

pub struct UiTrack {
    pub id: u32,
    pub name: String,
    pub track_type: TrackType,
    pub clips: Vec<UiClip>,
}

impl From<&KarbeatTrack> for UiTrack {
    fn from(value: &KarbeatTrack) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            track_type: value.track_type.clone(),
            clips: value
                .clips_to_vec()
                .iter()
                .map(|c| UiClip::from(c.deref()))
                .collect(),
        }
    }
}

pub struct UiClip {
    pub name: String,
    pub id: u32,
    pub start_time: u64,
    pub source: UiClipSource,
    pub offset_start: u64,
    pub loop_length: u64,
}

pub enum UiClipSource {
    Audio { source_id: u32 },
    Midi { pattern_id: u32 },
    None, // represent clip with empty source, this is placeholder, as this will be removed when I already implement MIDI Pattern and automation
}

impl From<&Clip> for UiClip {
    fn from(value: &Clip) -> Self {
        // Map source to either AudioWaveform, midi
        let source = match &value.source {
            KarbeatSource::Audio(_) => UiClipSource::Audio {
                source_id: value.source_id,
            },
            KarbeatSource::Midi(_) => UiClipSource::Midi {
                pattern_id: value.source_id,
            },
            _ => UiClipSource::None,
        };
        Self {
            name: value.name.clone(),
            id: value.id,
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
    pub preview_buffer: Vec<f32>, // Store sample buffer, essential to draw waveform
    pub file_path: String,
    pub name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration: f64,
    pub root_note: u8,
    pub fine_tune: i16,
    pub trim_start: u64,
    pub trim_end: u64,
    pub is_looping: bool,
    pub normalized: bool,
    pub muted: bool, // this only affects when play stream, not when doing preview sound
}

pub struct AudioWaveformUiForClip {
    pub name: String,
    pub preview_buffer: Vec<f32>,
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
        Self {
            preview_buffer: value.buffer.to_vec(),
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
        Self {
            preview_buffer: value.buffer.as_ref().clone(),
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
    pub internal_type: String,
    pub parameters: HashMap<u32, f32>,
}

impl From<&GeneratorInstance> for UiGeneratorInstance {
    fn from(generator_instance: &GeneratorInstance) -> Self {
        match &generator_instance.instance_type {
            GeneratorInstanceType::Plugin(plugin_instance) => Self {
                id: generator_instance.id,
                name: plugin_instance.name.clone(),
                internal_type: plugin_instance.internal_type.clone(),
                parameters: plugin_instance.parameters.clone(),
            },
            GeneratorInstanceType::Sampler { .. } => {
                Self {
                    id: generator_instance.id,
                    name: "Sampler".to_string(),
                    internal_type: "Sampler".to_string(),
                    parameters: HashMap::new(), // Add sampler params later if needed
                }
            }
            GeneratorInstanceType::AudioInput { .. } => Self {
                id: generator_instance.id,
                name: "Audio Input".to_string(),
                internal_type: "AudioInput".to_string(),
                parameters: HashMap::new(),
            },
        }
    }
}

// ============================================================
// =====================SESSION STATE==========================
// ============================================================

pub struct UiSessionState {
    // What is the user clicking on right now?
    pub selected_track_id: Option<u32>,
    pub selected_clip_id: Option<u32>,
    // Undo/Redo Stack
    // We don't save this usually, or we save it separately
    // pub undo_stack: Vec<AudioCommandUi>,
    // pub redo_stack: Vec<AudioCommandUi>,

    // Clipboard for Copy/Paste
    // pub clipboard: Option<Clip>, Option<Clipboard>
}

impl From<&SessionState> for UiSessionState {
    fn from(session: &SessionState) -> Self {
        Self {
            selected_clip_id: session.selected_clip_id,
            selected_track_id: session.selected_track_id,
        }
    }
}

impl Into<UiSessionState> for SessionState {
    fn into(self) -> UiSessionState {
        UiSessionState {
            selected_track_id: self.selected_track_id,
            selected_clip_id: self.selected_clip_id,
        }
    }
}
// ============================ APIs ==================================

pub fn get_ui_state() -> Option<UiProjectState> {
    let Ok(app) = APP_STATE.read() else {
        return None; // send empty
    };

    let project_state = UiProjectState {
        // Cloning shared structs is cheap and clean
        metadata: app.metadata.clone(),

        // Only write custom logic for the parts that actually change structure
        tracks: app
            .tracks
            .values()
            .map(|t| UiTrack {
                clips: t.clips.iter().map(|e| UiClip::from(e.deref())).collect(),
                id: t.id,
                name: t.name.clone(),
                track_type: t.track_type().clone(),
            })
            .collect(),
    };

    Some(project_state)
}

pub fn get_project_metadata() -> Result<ProjectMetadata, String> {
    let Ok(app) = APP_STATE.read() else {
        return Err("Failed to acquire read lock on APP_STATE".to_string());
    };

    let metadata = app.metadata.clone();
    Ok(metadata)
}

pub fn get_transport_state() -> Result<TransportState, String> {
    let Ok(app) = APP_STATE.read() else {
        return Err("Failed to acquire read lock on APP_STATE".to_string());
    };

    let ts = app.transport.clone();
    Ok(ts)
}

pub fn get_audio_source_list() -> Option<HashMap<u32, AudioWaveformUiForAudioProperties>> {
    // Read from app state
    let Ok(app) = APP_STATE.read() else {
        return None; // Send empty
    };
    let map = app
        .asset_library
        .source_map
        .iter()
        .map(|(&id, arc_waveform)| {
            let ui = AudioWaveformUiForAudioProperties::from(arc_waveform.as_ref());
            (id, ui)
        })
        .collect();

    Some(map)
}

/// Get generator list used in the project
pub fn get_generator_list() -> Result<HashMap<u32, UiGeneratorInstance>, String> {
    let app = APP_STATE
        .read()
        .map_err(|e| format!("Poisoned error: {}", e))?;

    let generators = app
        .generator_pool
        .iter()
        .map(|(&id, generator_guard)| {
            let generator = generator_guard
                .read()
                .expect("Failed to read generator lock");
            let ui_gen = UiGeneratorInstance::from(&*generator);
            (id, ui_gen)
        })
        .collect();

    Ok(generators)
}

pub fn add_audio_source(file_path: &str) {
    {
        if let Ok(mut app) = APP_STATE.write() {
            // Add audio source
            match app.load_audio(file_path.to_string(), None) {
                Ok(id) => {
                    let Some(audio) = app.asset_library.source_map.get(&id) else {
                        log::error!("[error] can't get the audiowave");
                        return;
                    };

                    log::info!("Sucessfully add {}", audio.name);
                }
                Err(e) => {
                    log::error!("[error] failed to load the audio: {}", e);
                }
            }
        };
    }

    broadcast_state_change();
}

/// Add new track to the track list. Throws an error, so it must handled gracefully
pub fn add_new_track(track_type: TrackType) -> Result<(), String> {
    {
        let mut app = match APP_STATE.write() {
            Ok(a) => a,
            Err(e) => {
                return Err(format!(
                    "[error] error when acquaring write lock for APP_STATE: {}",
                    e
                ))
            }
        };
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
    let app = APP_STATE
        .read()
        .map_err(|e| format!("Error acquiring read lock of APP_STATE: {}", e))?;

    // convert the tracks into UI-Friendly type
    let return_data = app
        .tracks
        .iter()
        .map(|(id, track_arc)| (*id, UiTrack::from(track_arc.as_ref())))
        .collect();

    Ok(return_data)
}

pub fn get_max_sample_index() -> Result<u64, String> {
    let app = APP_STATE
        .read()
        .map_err(|e| format!("Error acquiring read lock of APP_STATE: {}", e))?;

    Ok(app.max_sample_index)
}

pub fn get_session_state() -> Result<UiSessionState, String> {
    let app = APP_STATE
        .read()
        .map_err(|e| format!("APP_STATE got poisoned: {}", e))?;

    let session = UiSessionState::from(&app.session);
    Ok(session)
}
