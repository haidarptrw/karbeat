use std::{collections::HashMap, ops::Deref};

use chrono::{DateTime, Utc};
use flutter_rust_bridge::frb;
use karbeat_utils::audio_utils::quantize_to_i8;
use serde::Serialize;

use crate::broadcast_state_change;
use karbeat_core::lock::{get_app_read, get_app_write};
use karbeat_core::{
    core::{
        file_manager::audio_loader::AudioLoader,
        project::{
            clip::Clip,
            generator::{GeneratorInstance, GeneratorInstanceType},
            track::{audio_waveform::AudioWaveform, KarbeatTrack, TrackType},
            transport::TransportState,
            AudioHardwareConfig, AudioSourceId, KarbeatSource, ProjectMetadata,
        },
    },
    utils::get_waveform_buffer,
};

pub enum UiTrackType {
    Audio,
    Midi,
    Automation,
}

impl From<UiTrackType> for TrackType {
    fn from(value: UiTrackType) -> Self {
        match value {
            UiTrackType::Audio => TrackType::Audio,
            UiTrackType::Midi => TrackType::Midi,
            UiTrackType::Automation => TrackType::Automation,
        }
    }
}

impl Into<UiTrackType> for TrackType {
    fn into(self) -> UiTrackType {
        match self {
            TrackType::Audio => UiTrackType::Audio,
            TrackType::Midi => UiTrackType::Midi,
            TrackType::Automation => UiTrackType::Automation,
        }
    }
}

pub struct UiTrack {
    pub id: u32,
    pub name: String,
    pub color: String,
    pub track_type: UiTrackType,
    pub clips: Vec<UiClip>,
    pub generator_id: Option<u32>,
}

#[derive(Clone, Default)]
pub struct UiProjectMetadata {
    pub name: String,
    pub author: String,
    pub version: String,
    pub created_at: String,
}

impl From<ProjectMetadata> for UiProjectMetadata {
    fn from(m: ProjectMetadata) -> Self {
        Self {
            name: m.name,
            author: m.author,
            version: m.version,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

impl From<UiProjectMetadata> for ProjectMetadata {
    fn from(m: UiProjectMetadata) -> Self {
        Self {
            name: m.name,
            author: m.author,
            version: m.version,
            created_at: m.created_at.parse::<DateTime<Utc>>().unwrap_or(Utc::now()),
        }
    }
}

pub struct UiAudioHardwareConfig {
    pub selected_input_device: String,
    pub selected_output_device: String,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub cpu_load: f32,
}

impl From<AudioHardwareConfig> for UiAudioHardwareConfig {
    fn from(c: AudioHardwareConfig) -> Self {
        Self {
            selected_input_device: c.selected_input_device,
            selected_output_device: c.selected_output_device,
            sample_rate: c.sample_rate,
            buffer_size: c.buffer_size,
            cpu_load: c.cpu_load,
        }
    }
}

impl From<UiAudioHardwareConfig> for AudioHardwareConfig {
    fn from(c: UiAudioHardwareConfig) -> Self {
        Self {
            selected_input_device: c.selected_input_device,
            selected_output_device: c.selected_output_device,
            sample_rate: c.sample_rate,
            buffer_size: c.buffer_size,
            cpu_load: c.cpu_load,
        }
    }
}

#[derive(Default, Clone)]
pub struct UiTransportState {
    pub bpm: f32,
    pub time_signature: (u8, u8),
}

impl From<TransportState> for UiTransportState {
    fn from(s: TransportState) -> Self {
        Self {
            bpm: s.bpm,
            time_signature: s.time_signature,
        }
    }
}

impl From<UiTransportState> for TransportState {
    fn from(s: UiTransportState) -> Self {
        Self {
            bpm: s.bpm,
            time_signature: s.time_signature,
        }
    }
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
            color: value.color.to_string(),
            track_type: value.track_type.clone().into(),
            clips: value
                .clips_to_vec()
                .iter()
                .map(|c| UiClip::from(c.deref()))
                .collect(),
            generator_id,
        }
    }
}

#[frb(sync)]
pub fn project_metadata_new() -> UiProjectMetadata {
    UiProjectMetadata::default()
}

#[frb(sync)]
pub fn audio_hardware_config_new() -> UiAudioHardwareConfig {
    UiAudioHardwareConfig::from(AudioHardwareConfig::default())
}

#[frb(sync)]
pub fn audio_hardware_config_new_with_param(
    selected_input_device: String,
    selected_output_device: String,
    sample_rate: u32,
    buffer_size: u32,
    cpu_load: f32,
) -> UiAudioHardwareConfig {
    UiAudioHardwareConfig {
        selected_input_device,
        selected_output_device,
        sample_rate,
        buffer_size,
        cpu_load,
    }
}

#[frb(sync)]
pub fn transport_state_new() -> UiTransportState {
    UiTransportState::default()
}

#[frb(sync)]
pub fn transport_state_new_with_param(bpm: f32, time_signature: (u8, u8)) -> UiTransportState {
    UiTransportState {
        bpm,
        time_signature,
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
    pub channels: u16,
    pub duration: f64,
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
        let preview_buffer: Vec<i8> = get_waveform_buffer(&value.buffer)
            .map(|slice| {
                slice
                    .iter()
                    .map(|&s| (s.clamp(-1.0, 1.0) * 127.0) as i8)
                    .collect()
            })
            .unwrap_or_default();
        Self {
            preview_buffer,
            file_path: value.file_path.display().to_string(),
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
        let preview_buffer: Vec<i8> = get_waveform_buffer(&value.buffer)
            .map(|slice| {
                slice
                    .iter()
                    .map(|&s| (s.clamp(-1.0, 1.0) * 127.0) as i8)
                    .collect()
            })
            .unwrap_or_default();
        Self {
            preview_buffer,
            name: value.name.clone(),
            sample_rate: value.sample_rate,
            channels: value.channels,
            duration: value.duration,
        }
    }
}

#[frb(ignore)]
impl AudioWaveformUiForClip {
    pub fn try_from_audio_waveform_with_target_sample_bin(source_id: u32) -> Result<Self, String> {
        let app = get_app_read();
        Self::try_from_audio_waveform_with_target_sample_bin_internal(&app, source_id)
    }

    pub fn try_from_audio_waveform_with_target_sample_bin_internal(
        app: &karbeat_core::core::project::ApplicationState,
        source_id: u32,
    ) -> Result<Self, String> {
        let audio_waveform = app
            .get_audio_source(source_id)
            .ok_or("cannot find audio source")?;

        let preview_buffer = get_waveform_buffer(&audio_waveform.buffer)
            .map(|slice| quantize_to_i8(slice))
            .unwrap_or_default();


        let audio_waveform_ui = AudioWaveformUiForClip {
            preview_buffer,
            name: audio_waveform.name.clone(),
            sample_rate: audio_waveform.sample_rate,
            channels: audio_waveform.channels,
            duration: audio_waveform.duration,
        };

        Ok(audio_waveform_ui)
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
                parameters: plugin_instance.parameters.clone().into_iter().collect(),
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

// ============================ APIs ==================================

/// Get the current project metadata state from the backend
pub fn get_project_metadata() -> Result<UiProjectMetadata, String> {
    let app = get_app_read();

    let metadata = app.metadata.clone();
    Ok(UiProjectMetadata::from(metadata))
}

/// Get the transport state from the backend
pub fn get_transport_state() -> Result<UiTransportState, String> {
    let app = get_app_read();

    let ts = app.transport.clone();
    Ok(UiTransportState::from(ts))
}

/// Get all audio waveform source list from the backend
pub fn get_audio_source_list() -> Option<HashMap<u32, AudioWaveformUiForSourceList>> {
    // Read from app state
    let app = get_app_read();
    let map = app
        .asset_library
        .source_map
        .iter()
        .map(|(&id, arc_waveform)| {
            let ui = AudioWaveformUiForSourceList::from(arc_waveform.as_ref());
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
        .map(|(&id, generator_arc)| {
            let generator = generator_arc.deref();
            let ui_gen = UiGeneratorInstance::from(generator);
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
        match app.load_audio(file_path, None) {
            Ok(id) => {
                let Some(audio) = app.asset_library.source_map.get(&AudioSourceId::from(id)) else {
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
pub fn add_new_track(track_type: UiTrackType) -> Result<(), String> {
    {
        let mut app = get_app_write();
        app.add_new_track(track_type.into());
        log::info!("[add_new_track] successfully add new track");
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
