use std::{collections::HashMap, hash::Hash};

use rodio::source;
use serde::Serialize;

use crate::{
    broadcast_state_change,
    core::{
        file_manager::loader::AudioLoader,
        project::{Clip, KarbeatSource, KarbeatTrack, ProjectMetadata, TrackType, TransportState},
        track::audio_waveform::AudioWaveform,
    },
    utils::audio_utils::downsample,
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
            clips: value.clips.iter().map(|c| UiClip::from(c)).collect()
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
    Audio(AudioWaveformUiForClip),
    None, // represent clip with empty source, this is placeholder, as this will be removed when I already implement MIDI Pattern and automation
}

impl From<&Clip> for UiClip {
    fn from(value: &Clip) -> Self {
        // Map source to either AudioWaveform, midi
        let source = match &value.source {
            KarbeatSource::Audio(audio_waveform) => UiClipSource::Audio(audio_waveform.as_ref().into()),
            _ => UiClipSource::None,
        };
        Self {
            name: value.name.clone(),
            id: value.id,
            start_time: value.start_time,
            source: source,
            offset_start: value.offset_start,
            loop_length: value.loop_length
        }
    }
}

// UI Data Structure for Audio Waveform window information (to change vol, pitch fine tune, normalization, panning, adsr envelope,
// play the audio when pressing the waveform etc)

#[derive(Clone, Debug, Serialize)]
pub struct AudioWaveformUiForSourceList {
    pub name: String,
    pub muted: bool,
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
}

impl From<&AudioWaveform> for AudioWaveformUiForSourceList {
    fn from(value: &AudioWaveform) -> Self {
        Self {
            name: value.name.clone(),
            muted: value.muted,
        }
    }
}

impl From<&AudioWaveform> for AudioWaveformUiForAudioProperties {
    fn from(value: &AudioWaveform) -> Self {
        Self {
            preview_buffer: downsample(value.buffer.as_ref()),
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
            preview_buffer: downsample(value.buffer.as_ref()),
            name: value.name.clone()
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
                clips: t.clips.iter().map(|e| UiClip::from(e)).collect(),
                id: t.id,
                name: t.name.clone(),
                track_type: t.track_type().clone(),
            })
            .collect(),
    };

    Some(project_state)
}

pub fn get_transport_state() -> Result<TransportState, String> {
    let Ok(app) = APP_STATE.read() else {
        return Err("Failed to acquire read lock on APP_STATE".to_string());
    };

    let ts = app.transport.clone();
    Ok(ts)
}

pub fn get_source_list() -> Option<HashMap<u32, AudioWaveformUiForSourceList>> {
    // Read from app state
    let Ok(app) = APP_STATE.read() else {
        return None; // Send empty
    };
    let map = app
        .asset_library
        .source_map
        .iter()
        .map(|(&id, arc_waveform)| {
            let ui = AudioWaveformUiForSourceList::from(arc_waveform.as_ref());
            (id, ui)
        })
        .collect();

    Some(map)
}

pub fn add_audio_source(file_path: &str) {
    {
        if let Ok(mut app) = APP_STATE.write() {
            // Add audio source
            match app.load_audio(file_path.to_string(), None) {
                Ok(id) => {
                    let Some(audio) = app.asset_library.source_map.get(&id) else {
                        println!("[error] can't get the audiowave");
                        return;
                    };

                    println!("Sucessfully add {}", audio.name);
                }
                Err(e) => {
                    println!("[error] failed to load the audio: {}", e);
                }
            }
        };
    }

    broadcast_state_change();
}

pub fn add_new_track(track_type: String) -> Result<(), String> {
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
        let track_type_concrete = track_type.parse::<TrackType>()?;
        app.add_new_track(track_type_concrete);
    }
    broadcast_state_change();
    Ok(())
}

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
