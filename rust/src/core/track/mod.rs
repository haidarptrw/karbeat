// src/core/track/mod.rs

use crate::core::track::audio_waveform::AudioWaveform;

pub mod audio_waveform;

pub enum TrackType {
    Waveform(AudioWaveform),
    PianoRoll,
    Automation
}

