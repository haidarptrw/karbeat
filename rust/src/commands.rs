use crate::{
    audio::engine::PlaybackMode,
    core::project::{
        mixer::EffectId,
        plugin::{KarbeatEffect, KarbeatGenerator},
        track::audio_waveform::AudioWaveform,
        GeneratorId, TrackId,
    },
};

pub enum AudioCommand {
    PlayOneShot(AudioWaveform),
    StopAllPreviews,
    ResetPlayhead,
    SetPlayhead(u32),
    PlayPreviewNote {
        note_key: u8,
        generator_id: u32,
        velocity: u8,
        is_note_on: bool,
    },
    /// Set BPM to the field0 value
    SetBPM(f32),
    SetPlaybackMode(PlaybackMode),

    // =========================================================================
    // Generator Plugin Commands
    // =========================================================================
    /// Add a generator plugin to the audio thread
    AddGenerator {
        generator_id: GeneratorId,
        track_id: TrackId,
        plugin: Box<dyn KarbeatGenerator + Send>,
    },
    /// Remove a generator plugin from the audio thread
    RemoveGenerator {
        generator_id: GeneratorId,
    },
    /// Set a parameter on a generator plugin
    SetGeneratorParameter {
        generator_id: GeneratorId,
        param_id: u32,
        value: f32,
    },
    /// Update generator's associated track
    UpdateGeneratorTrack {
        generator_id: GeneratorId,
        track_id: TrackId,
    },
    /// Request parameter feedback for a generator (triggers ParameterUpdate responses)
    QueryGeneratorParameters {
        generator_id: GeneratorId,
    },

    // =========================================================================
    // Effect Plugin Commands
    // =========================================================================
    /// Add an effect to a track's effect chain
    AddTrackEffect {
        track_id: TrackId,
        effect_id: EffectId,
        effect: Box<dyn KarbeatEffect + Send>,
    },
    /// Remove an effect from a track's effect chain
    RemoveTrackEffect {
        track_id: TrackId,
        effect_idx: usize,
    },
    /// Set a parameter on a track effect
    SetTrackEffectParameter {
        track_id: TrackId,
        effect_idx: usize,
        param_id: u32,
        value: f32,
    },

    // ======================================================
    // Master Effect Command
    // ======================================================
    /// Add an effect to the master bus
    AddMasterEffect {
        effect_id: EffectId,
        effect: Box<dyn KarbeatEffect + Send>,
    },
    /// Remove an effect from the master bus
    RemoveMasterEffect {
        effect_idx: usize,
    },
    /// Set a parameter on a master effect
    SetMasterEffectParameter {
        effect_idx: usize,
        param_id: u32,
        value: f32,
    },
}

// ============================================================================
// Audio → UI Feedback Messages
// ============================================================================

/// Parameter value update from audio thread to UI
#[derive(Clone, Debug)]
pub struct ParameterUpdate {
    pub generator_id: GeneratorId,
    pub param_id: u32,
    pub value: f32,
}

/// Full parameter snapshot for a generator (response to QueryGeneratorParameters)
#[derive(Clone, Debug)]
pub struct GeneratorParameterSnapshot {
    pub generator_id: GeneratorId,
    pub parameters: Vec<(u32, f32)>, // (param_id, value) pairs
}

/// Messages from audio thread to UI thread
#[derive(Clone, Debug)]
pub enum AudioFeedback {
    /// Single parameter changed (e.g., automation moved it)
    ParameterChanged(ParameterUpdate),
    /// Full parameter snapshot in response to query
    ParameterSnapshot(GeneratorParameterSnapshot),
}
