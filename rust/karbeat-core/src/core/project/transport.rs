use serde::{Deserialize, Serialize};

/// Serializable project transport settings.
/// Runtime transport state (is_playing, playhead, etc.) lives in the AudioEngine.
#[derive(Serialize, Deserialize, Clone)]
pub struct TransportState {
    // general state
    pub bpm: f32,
    pub time_signature: (u8, u8),
}

impl Default for TransportState {
    fn default() -> Self {
        Self {
            bpm: 67.0,
            time_signature: (4, 4),
        }
    }
}

impl PartialEq for TransportState {
    fn eq(&self, other: &Self) -> bool {
        self.bpm == other.bpm && self.time_signature == other.time_signature
    }
}
