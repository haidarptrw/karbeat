use std::{collections::HashMap, ops::Deref};

use chrono::{DateTime, Utc};
use flutter_rust_bridge::frb;
use karbeat_core::api::{audio_waveform_api, project_api, track_api};
use karbeat_core::core::project::{ApplicationState, PluginInstance};
use karbeat_utils::audio_utils::quantize_to_i8;
use serde::Serialize;
use karbeat_core::{
    core::
        project::{
            clip::Clip,
            generator::{GeneratorInstance, GeneratorInstanceType},
            track::{audio_waveform::AudioWaveform, KarbeatTrack, TrackType},
            transport::TransportState,
            AudioHardwareConfig, KarbeatSource, ProjectMetadata,
        }
    ,
    utils::get_waveform_buffer,
};

pub enum UiTrackType {
    Audio,
    Midi,
    Automation,
}

pub struct UiApplicationState {
    pub metadata: UiProjectMetadata,
    pub transport: UiTransportState,
    pub hardware_config: UiAudioHardwareConfig,
    pub tracks: HashMap<u32, UiTrack>,
    pub generators: HashMap<u32, UiGeneratorInstance>,
    pub patterns: HashMap<u32, crate::api::pattern::UiPattern>,
    pub mixer: crate::api::mixer::UiMixerState,
    pub max_sample_index: u32,
    pub audio_sources: HashMap<u32, AudioWaveformUiForSourceList>,
}

impl From<ApplicationState> for UiApplicationState {
    fn from(value: ApplicationState) -> Self {
        let tracks: HashMap<u32, UiTrack> = value
            .tracks
            .iter()
            .map(|(id, track)| (id.to_u32(), UiTrack::from(track.as_ref())))
            .collect();

        let generators: HashMap<u32, UiGeneratorInstance> = value
            .generator_pool
            .iter()
            .map(|(id, gen)| (id.to_u32(), UiGeneratorInstance::from(gen.as_ref())))
            .collect();

        let patterns: HashMap<u32, crate::api::pattern::UiPattern> = value
            .pattern_pool
            .iter()
            .map(|(id, pat)| {
                (
                    id.to_u32(),
                    crate::api::pattern::UiPattern::from(pat.as_ref()),
                )
            })
            .collect();

        let audio_sources: HashMap<u32, AudioWaveformUiForSourceList> = value
            .asset_library
            .source_map
            .iter()
            .map(|(id, source)| {
                (
                    id.to_u32(),
                    AudioWaveformUiForSourceList::from(source.as_ref()),
                )
            })
            .collect();

        Self {
            metadata: UiProjectMetadata::from(value.metadata),
            transport: UiTransportState::from(value.transport),
            hardware_config: UiAudioHardwareConfig::from(&value.audio_config),
            tracks,
            generators,
            patterns,
            mixer: crate::api::mixer::UiMixerState::from(&value.mixer),
            max_sample_index: value.max_sample_index,
            audio_sources,
        }
    }
}

impl From<&UiTrackType> for TrackType {
    fn from(value: &UiTrackType) -> Self {
        match value {
            UiTrackType::Audio => TrackType::Audio,
            UiTrackType::Midi => TrackType::Midi,
            UiTrackType::Automation => TrackType::Automation,
        }
    }
}

impl From<TrackType> for UiTrackType {
    fn from(value: TrackType) -> Self {
        match value {
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

impl From<&AudioHardwareConfig> for UiAudioHardwareConfig {
    fn from(c: &AudioHardwareConfig) -> Self {
        Self {
            selected_input_device: c.selected_input_device.clone(),
            selected_output_device: c.selected_output_device.clone(),
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
        let generator_id = value.generator.as_ref().map(|gen_instance| gen_instance.id.to_u32());
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
    UiAudioHardwareConfig::from(&AudioHardwareConfig::default())
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
            source,
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
        audio_waveform_api::get_audio_waveform(source_id, |waveform| {
            let preview_buffer = get_waveform_buffer(&waveform.buffer)
                .map(|slice| quantize_to_i8(slice))
                .unwrap_or_default();

            Self {
                preview_buffer,
                name: waveform.name.clone(),
                sample_rate: waveform.sample_rate,
                channels: waveform.channels,
                duration: waveform.duration,
            }
        }).map_err(|e| e.to_string())
    }
}
// ============================================================
// =================== GENERATOR INSTANCE =====================
// ============================================================
pub struct UiGeneratorInstance {
    pub id: u32,
    pub instance_type: UiGeneratorInstanceType,
}

pub struct UiPluginInstance {
    /// Registry ID for plugin lookup (stable identifier)
    pub registry_id: u32,
    /// Name of the plugin (for display purposes)
    pub name: String,
    /// Whether this plugin is bypassed
    pub bypass: bool,
    /// Plugin parameters for persistence (Param ID -> Value)
    pub parameters: HashMap<u32, f32>,
}

impl From<PluginInstance> for UiPluginInstance {
    fn from(value: PluginInstance) -> Self {
        Self {
            registry_id: value.registry_id,
            name: value.name,
            bypass: value.bypass,
            parameters: value.parameters.into_iter().collect(),
        }
    }
}

pub enum UiGeneratorInstanceType {
    Plugin(UiPluginInstance),
    Sampler { asset_id: u32, root_note: u8 },
}

impl From<GeneratorInstanceType> for UiGeneratorInstanceType {
    fn from(value: GeneratorInstanceType) -> Self {
        match value {
            GeneratorInstanceType::Plugin(plugin_instance) => Self::Plugin(UiPluginInstance::from(plugin_instance)),
            GeneratorInstanceType::Sampler { asset_id, root_note } => Self::Sampler { asset_id, root_note },
        }
    }
}

impl From<&GeneratorInstance> for UiGeneratorInstance {
    fn from(generator_instance: &GeneratorInstance) -> Self {
        match &generator_instance.instance_type {
            GeneratorInstanceType::Plugin(plugin_instance) => Self {
                id: generator_instance.id.to_u32(),
                instance_type: UiGeneratorInstanceType::Plugin(UiPluginInstance::from(plugin_instance.to_owned())),
            },
            GeneratorInstanceType::Sampler { asset_id, root_note } => {
                Self {
                    id: generator_instance.id.to_u32(),
                    instance_type: UiGeneratorInstanceType::Sampler { asset_id: *asset_id, root_note: *root_note },
                }
            }
        }
    }
}

// ============================ APIs ==================================

/// Get the current project metadata state from the backend
pub fn get_project_metadata() -> Result<UiProjectMetadata, String> {
    project_api::get_project_metadata(|m| UiProjectMetadata::from(m.clone()))
        .map_err(|e| e.to_string())
}

/// Get the transport state from the backend
pub fn get_transport_state() -> Result<UiTransportState, String> {
    project_api::get_transport_state(|t| UiTransportState::from(t.clone()))
        .map_err(|e| e.to_string())
}

/// Get all audio waveform source list from the backend
pub fn get_audio_source_list() -> Option<HashMap<u32, AudioWaveformUiForSourceList>> {
    audio_waveform_api::get_audio_source_list(|id, wf| {
        (id, AudioWaveformUiForSourceList::from(wf))
    }).ok()
}

/// Get generator list used in the project
pub fn get_generator_list() -> Result<HashMap<u32, UiGeneratorInstance>, String> {
    project_api::get_generator_list(|id, gen| {
        (id, UiGeneratorInstance::from(gen))
    }).map_err(|e| e.to_string())
}

/// Add a new audio source to the project
///
/// ## Parameters:
/// - file_path: Path to the audio file to be added
pub fn add_audio_source(file_path: &str) -> Result<u32, String> {
    let source_id = audio_waveform_api::add_audio_source(file_path).map_err(|e| e.to_string())?;
    Ok(source_id.to_u32())
}

/// Add new track to the track list. Throws an error, so it must handled gracefully
pub fn add_new_track(track_type: UiTrackType) -> UiTrack {
    let arc_track = {
        track_api::add_new_track((&track_type).into())
    };
    log::info!("[add_new_track] successfully added new track");
    UiTrack::from(arc_track.as_ref())
}

/// Get all tracks on the session/project.
///
/// Returns Map<u32, UiTrack> upon success, and Error when it fails
pub fn get_tracks() -> Result<HashMap<u32, UiTrack>, String> {
    track_api::get_tracks(|id, track| {
        (id, UiTrack::from(track))
    }).map_err(|e| e.to_string())
}

/// Get the newest max sample index of the project
pub fn get_max_sample_index() -> Result<u32, String> {
    project_api::get_max_sample_index().map_err(|e| e.to_string())
}
