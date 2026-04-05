use indexmap::IndexMap;

use crate::{
    audio::engine::PlaybackMode,
    core::project::{
        GeneratorId, TrackId, mixer::{BusId, EffectId, RoutingConnection}, plugin::{KarbeatEffect, KarbeatGenerator}, track::audio_waveform::AudioWaveform
    },
};

pub enum AudioCommand {
    PlayOneShot(AudioWaveform),
    StopAllPreviews,
    /// Set playback state (play/pause)
    SetPlaying(bool),
    /// Set loop mode
    SetLooping(bool),
    /// Stop playback and reset playhead to 0
    StopAndReset,
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
    /// Request parameter feedback for a generator (triggers ParameterSnapshot response)
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
        effect: Box<dyn KarbeatEffect + Send + Sync>,
    },
    /// Remove an effect from a track's effect chain
    RemoveTrackEffect {
        track_id: TrackId,
        effect_id: EffectId,
    },
    /// Set a parameter on a track effect
    SetTrackEffectParameter {
        track_id: TrackId,
        effect_id: EffectId,
        param_id: u32,
        value: f32,
    },
    /// Request parameter feedback for a track effect (triggers EffectParameterSnapshot response)
    QueryTrackEffectParameters {
        track_id: TrackId,
        effect_id: EffectId,
    },

    // ======================================================
    // Master Effect Command
    // ======================================================
    /// Add an effect to the master bus
    AddMasterEffect {
        effect_id: EffectId,
        effect: Box<dyn KarbeatEffect + Send + Sync>,
    },
    /// Remove an effect from the master bus
    RemoveMasterEffect {
        effect_id: EffectId,
    },
    /// Set a parameter on a master effect
    SetMasterEffectParameter {
        effect_id: EffectId,
        param_id: u32,
        value: f32,
    },
    /// Request parameter feedback for a master effect (triggers EffectParameterSnapshot response)
    QueryMasterEffectParameters {
        effect_id: EffectId,
    },

    // =========================================================================
    // Bus Commands
    // =========================================================================
    /// Create a new mixer bus on the audio thread
    AddBus {
        bus_id: BusId,
        name: String,
    },
    /// Remove a mixer bus from the audio thread
    RemoveBus {
        bus_id: BusId,
    },
    /// Set bus channel parameters (volume, pan, mute)
    SetBusParams {
        bus_id: BusId,
        volume: Option<f32>,
        pan: Option<f32>,
        mute: Option<bool>,
    },
    /// Add effect to a bus
    AddBusEffect {
        bus_id: BusId,
        effect_id: EffectId,
        effect: Box<dyn KarbeatEffect + Send + Sync>,
    },
    /// Remove effect from a bus
    RemoveBusEffect {
        bus_id: BusId,
        effect_id: EffectId,
    },
    /// Set a bus effect parameter
    SetBusEffectParameter {
        bus_id: BusId,
        effect_id: EffectId,
        param_id: u32,
        value: f32,
    },
    /// Request parameter feedback for a bus effect (triggers EffectParameterSnapshot response)
    QueryBusEffectParameters {
        bus_id: BusId,
        effect_id: EffectId,
    },
    /// Update the routing matrix (sync from main thread)
    UpdateRouting {
        routing: Vec<RoutingConnection>,
    },
    /// Prepare all of plugins from ApplicationState to AudioEngine (upon loading project)
    PreparePlugin {
        track_effects: IndexMap<TrackId, IndexMap<EffectId, Box<dyn KarbeatEffect + Send + Sync>>>,
        master_effects: IndexMap<EffectId, Box<dyn KarbeatEffect + Send + Sync>>,
        bus_effects: IndexMap<BusId, IndexMap<EffectId, Box<dyn KarbeatEffect + Send + Sync>>>,
        generators: IndexMap<GeneratorId, Box<dyn KarbeatGenerator + Send + Sync>>,
    }
}

// ============================================================================
// Audio → UI Feedback Messages
// ============================================================================

/// Specifies the location of an effect to ensure precise UI syncing
#[derive(Clone, Debug)]
pub enum EffectTarget {
    Track(TrackId),
    Master,
    Bus(BusId),
}

/// Parameter value update from a generator to the UI
#[derive(Clone, Debug)]
pub struct GeneratorParameterUpdate {
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

/// Parameter value update from an effect to the UI
#[derive(Clone, Debug)]
pub struct EffectParameterUpdate {
    pub target: EffectTarget,
    pub effect_id: EffectId,
    pub param_id: u32,
    pub value: f32,
}

/// Full parameter snapshot for an effect (response to Query...EffectParameters)
#[derive(Clone, Debug)]
pub struct EffectParameterSnapshot {
    pub target: EffectTarget,
    pub effect_id: EffectId,
    pub parameters: Vec<(u32, f32)>, // (param_id, value) pairs
}

/// Messages from audio thread to UI thread
#[derive(Clone, Debug)]
pub enum AudioFeedback {
    // --- Generator Feedback ---
    /// Single parameter changed (e.g., automation moved it)
    GeneratorParameterChanged(GeneratorParameterUpdate),
    /// Full parameter snapshot in response to query
    GeneratorParameterSnapshot(GeneratorParameterSnapshot),

    // --- Effect Feedback ---
    /// Single parameter changed on an effect (e.g., automation moved it)
    EffectParameterChanged(EffectParameterUpdate),
    /// Full parameter snapshot for an effect in response to query
    EffectParameterSnapshot(EffectParameterSnapshot),
}
