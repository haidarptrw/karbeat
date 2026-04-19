use std::time::Duration;

use karbeat_core::api::audio_api;
use karbeat_core:: audio::event::TransportFeedback ;
use karbeat_core::core::project::{AudioSourceId, GeneratorId, TrackId};
use crate::api::project::{ AudioWaveformUiForAudioProperties, UiAudioHardwareConfig };
use crate::frb_generated::StreamSink;

#[derive(Clone, Copy, Debug)]
pub struct UiTransportFeedback {
    pub samples: u32,
    pub beat: usize,
    pub bar: usize,
    pub tempo: f32,
    pub sample_rate: u32,
    pub is_playing: bool,
    pub is_looping: bool,
    pub is_recording: bool,
    pub is_pattern_playing: bool,
    pub is_pattern_mode: bool,
    pub pattern_samples: u32,
    pub pattern_beat: usize,
    pub pattern_bar: usize,
}

impl From<TransportFeedback> for UiTransportFeedback {
    fn from(f: TransportFeedback) -> Self {
        Self {
            samples: f.samples,
            beat: f.beat,
            bar: f.bar,
            tempo: f.tempo,
            sample_rate: f.sample_rate,
            is_playing: f.is_playing,
            is_looping: f.is_looping,
            is_recording: f.is_recording,
            is_pattern_playing: f.is_pattern_playing,
            is_pattern_mode: f.is_pattern_mode,
            pattern_samples: f.pattern_samples,
            pattern_beat: f.pattern_beat,
            pattern_bar: f.pattern_bar,
        }
    }
}

/// GETTER: Fetch details + Downsampled Buffer for UI
pub fn get_audio_properties(id: u32) -> Option<AudioWaveformUiForAudioProperties> {
    audio_api::get_audio_source(AudioSourceId::from(id), |waveform| {
        //TODO: Add dynamic downsampling to send this to the frontend side for smaller memory footprint
        // downsample(waveform.buffer)
        AudioWaveformUiForAudioProperties::from(waveform)
    })
}

/// ACTION: Play the sound via the Engine
pub fn play_source_preview(id: u32) {
    if let Err(e) = audio_api::play_source_preview(AudioSourceId::from(id)) {
        log::warn!("Preview failed: {}", e);
    } else {
        log::info!("Preview command sent for ID: {}", id);
    }
}

pub fn stop_all_previews() {
    audio_api::stop_all_previews();
    println!("Stop all preview sounds");
}

pub fn get_audio_config() -> Result<UiAudioHardwareConfig, String> {
    Ok(audio_api::get_audio_config(|config| {
        UiAudioHardwareConfig::from(config)
    }))
}

pub fn create_position_stream(sink: StreamSink<UiTransportFeedback>) -> Result<(), String> {
    // Spawn a thread to poll the ring buffer
   std::thread::spawn(move || {
        loop {
            // Drain all pending events from the Core
            let feedbacks = audio_api::drain_position_feedback(UiTransportFeedback::from);
            
            for fb in feedbacks {
                if sink.add(fb).is_err() {
                    println!("[Rust] PlaybackPosition Stream disconnected! Stopping thread.");
                    return;
                }
            }

            // Sleep to prevent high CPU usage on this polling thread
            // 16ms ~= 60fps
            std::thread::sleep(Duration::from_millis(16));
        }
    });
    Ok(())
}

/// play preview sound when drawing note or pressing the piano tile on the UI
pub fn play_preview_note(
    track_id: u32,
    note_key: i32,
    velocity: i32,
    is_on: bool
) -> Result<(), String> {
    if !(0..=127).contains(&note_key) {
        return Err("Note key must be between 0 and 127".to_string());
    }

    if !(0..=100).contains(&velocity) {
        return Err("Note velocity must be between 0 and 100".to_string());
    }

    audio_api::play_preview_note(
        TrackId::from(track_id),
        note_key as u8,
        velocity as u8,
        is_on,
    )
    .map_err(|e| e.to_string())
}

/// Play preview sound directly on a generator (without requiring a track).
/// Used in plugin editor screens to test synth sounds.
pub fn play_preview_note_generator(
    generator_id: u32,
    note_key: i32,
    velocity: i32,
    is_on: bool
) -> Result<(), String> {
    if !(0..=127).contains(&note_key) {
        return Err("Note key must be between 0 and 127".to_string());
    }

    if !(0..=100).contains(&velocity) {
        return Err("Note velocity must be between 0 and 100".to_string());
    }

    if is_on {
        log::info!("Playing note {}", note_key);
    }

    audio_api::play_preview_note_generator(
        GeneratorId::from(generator_id),
        note_key as u8,
        velocity as u8,
        is_on,
    )
    .map_err(|e| e.to_string())
}
