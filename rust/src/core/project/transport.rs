use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct TransportState {
    pub is_playing: bool,
    pub is_pattern_playing: bool,
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
            is_pattern_playing: Default::default(),
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

impl PartialEq for TransportState {
    fn eq(&self, other: &Self) -> bool {
        self.is_playing == other.is_playing
            && self.is_pattern_playing == other.is_pattern_playing
            && self.is_recording == other.is_recording
            && self.is_looping == other.is_looping
            && self.playhead_position_samples == other.playhead_position_samples
            && self.loop_start_samples == other.loop_start_samples
            && self.loop_end_samples == other.loop_end_samples
            && self.bpm == other.bpm
            && self.time_signature == other.time_signature
            && self.beat_tracker == other.beat_tracker
            && self.bar_tracker == other.bar_tracker
    }
}
