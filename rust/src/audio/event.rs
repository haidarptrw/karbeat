use crate::core::project::{TrackId, mixer::{BusId, EffectId}};

/// Event struct for playback position that will be sent to Frontend side
#[derive(Clone, Copy, Debug)]
pub struct PlaybackPosition {
    // Song playback position
    pub samples: u32,
    pub beat: usize,
    pub bar: usize,
    pub tempo: f32, // Useful for Flutter to interpolate movement
    pub sample_rate: u32,
    pub is_playing: bool,

    // Pattern playback (independent from song)
    pub is_pattern_mode: bool,
    pub pattern_samples: u32,
    pub pattern_beat: usize,
    pub pattern_bar: usize,
}


// Automation event for event-driven automation system

// src/audio/event.rs (or wherever you define these)

pub enum GeneratorAutomationEvent {
    PluginParam { param_id: u32, value: f32 },
}

pub enum TrackAutomationEvent {
    Volume(f32),
    Pan(f32),
    PluginParam { effect_id: EffectId, param_id: u32, value: f32 },
}

pub enum BusAutomationEvent {
    Volume(f32),
    Pan(f32),
    PluginParam { effect_id: EffectId, param_id: u32, value: f32 },
}

pub enum MasterAutomationEvent {
    Volume(f32),
    Pan(f32),
    PluginParam { effect_id: EffectId, param_id: u32, value: f32 },
}

pub enum GlobalAutomationEvent {
    TempoBpm(f32),
}

pub enum AudioSourceAutomationEvent {
    Pitch(f32),
    Pan(f32),
    Volume(f32),
}