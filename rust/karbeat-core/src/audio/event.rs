use serde::{Deserialize, Serialize};

use crate::shared::id::*;

/// Transport feedback struct sent from the audio thread to Flutter.
/// This is the single source of truth for all runtime transport state.
#[derive(Clone, Copy, Debug)]
pub struct TransportFeedback {
    // Song playback position
    pub samples: u32,
    pub ticks: u32,
    pub beat: usize,
    pub bar: usize,
    pub tempo: f32, // Useful for Flutter to interpolate movement
    pub sample_rate: u32,

    // Transport state
    pub is_playing: bool,
    pub is_looping: bool,
    pub is_recording: bool,
    pub is_pattern_playing: bool,

    // Pattern playback (independent from song)
    pub is_pattern_mode: bool,
    pub pattern_samples: u32,
    pub pattern_ticks: u32,
    pub pattern_beat: usize,
    pub pattern_bar: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginTarget {
    Generator(GeneratorId),
    TrackEffect(TrackId, EffectId),
    BusEffect(BusId, EffectId),
    MasterEffect(EffectId),
}
