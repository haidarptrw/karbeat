use std::{collections::HashMap, sync::Arc};

use flutter_rust_bridge::frb;
use karbeat_core::{
    core::{
        file_manager::audio_loader::AudioLoader,
        project::{DawSource, TrackType},
    },
    lock::get_app_read,
    shared::{AudioSourceId, TrackId},
    utils::get_waveform_buffer,
};

pub use karbeat_core::core::project::AudioWaveform;
use serde::Serialize;

/// ======================================
/// WaveformHandle
/// An opaque handle that keeps a reference-counted pointer to an AudioWaveform
/// living entirely in Rust memory. Dart obtains a raw pointer and length via
/// sync methods to build a zero-copy Float32List view — no FFI serialization
/// of the audio buffer is performed.
/// ======================================
#[derive(Debug, Serialize, Clone)]
#[frb(opaque)]
pub struct WaveformHandle(Arc<AudioWaveform>);

impl WaveformHandle {
    /// Returns the raw memory address of the f32 interleaved sample buffer.
    /// Returns 0 if the waveform has no loaded buffer (e.g. not yet mmap-ed).
    #[frb(sync)]
    pub fn get_pointer(&self) -> usize {
        get_waveform_buffer(&self.0.buffer)
            .map(|s| s.as_ptr() as usize)
            .unwrap_or(0)
    }

    /// Returns the total number of f32 samples in the buffer (frames × channels).
    #[frb(sync)]
    pub fn get_len(&self) -> usize {
        get_waveform_buffer(&self.0.buffer)
            .map(|s| s.len())
            .unwrap_or(0)
    }

    /// Returns the sample rate of the audio waveform (e.g. 44100, 48000).
    #[frb(sync)]
    pub fn get_sample_rate(&self) -> u32 {
        self.0.sample_rate
    }

    /// Returns the number of channels in the audio waveform (1 = mono, 2 = stereo).
    #[frb(sync)]
    pub fn get_channels(&self) -> u16 {
        self.0.channels
    }
}

/// Get a WaveformHandle for a single audio source by its source ID.
/// Returns None if the source does not exist in the asset library.
#[frb(sync)]
pub fn get_waveform_handle(source_id: u32) -> Option<WaveformHandle> {
    let app = get_app_read();
    let wf = app.get_audio_source(&AudioSourceId::from(source_id))?;
    Some(WaveformHandle(wf.clone()))
}

/// Returns a map of source_id → WaveformHandle for every audio clip in the
/// given track. Returns an empty map if the track does not exist, is not an
/// audio track, or contains no audio clips.
///
/// This is a sync call — it only reads Arc pointers from the app state, so
/// there is no blocking I/O and no buffer copying.
#[frb(sync)]
pub fn get_waveform_handles_for_track(track_id: u32) -> HashMap<u32, WaveformHandle> {
    let app = get_app_read();

    let track = match app.tracks.get(&TrackId::from(track_id)) {
        Some(t) => t,
        None => return HashMap::new(),
    };

    // Only audio tracks have waveform data
    if !matches!(track.track_type, TrackType::Audio) {
        return HashMap::new();
    }

    let mut map = HashMap::new();

    for clip in track.clips() {
        if let Some(DawSource::Audio(source_id)) = clip.source {
            // Skip duplicates — a source may appear in multiple clips
            if map.contains_key(&source_id.to_u32()) {
                continue;
            }

            if let Some(wf) = app.get_audio_source(&source_id) {
                map.insert(source_id.to_u32(), WaveformHandle(wf.clone()));
            }
        }
    }

    map
}
