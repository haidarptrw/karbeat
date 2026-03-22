use crate::core::project::mixer::EffectId;

/// Transport feedback struct sent from the audio thread to Flutter.
/// This is the single source of truth for all runtime transport state.
#[derive(Clone, Copy, Debug)]
pub struct TransportFeedback {
    // Song playback position
    pub samples: u32,
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
    pub pattern_beat: usize,
    pub pattern_bar: usize,
}

// Automation event for event-driven automation system

pub enum GeneratorAutomationEvent {
    PluginParam { param_id: u32, value: f32 },
}

pub enum TrackAutomationEvent {
    Volume(f32),
    Pan(f32),
    PluginParam {
        effect_id: EffectId,
        param_id: u32,
        value: f32,
    },
}

pub enum BusAutomationEvent {
    Volume(f32),
    Pan(f32),
    PluginParam {
        effect_id: EffectId,
        param_id: u32,
        value: f32,
    },
}

pub enum MasterAutomationEvent {
    Volume(f32),
    Pan(f32),
    PluginParam {
        effect_id: EffectId,
        param_id: u32,
        value: f32,
    },
}

pub enum GlobalAutomationEvent {
    TempoBpm(f32),
}

pub enum AudioSourceAutomationEvent {
    Pitch(f32),
    Pan(f32),
    Volume(f32),
}
