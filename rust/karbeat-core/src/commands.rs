use indexmap::IndexMap;
use karbeat_plugin_api::types::ZeroCopyBuffer;

use crate::{
    audio::{engine::PlaybackMode, event::PluginTarget},
    core::project::{
        mixer::RoutingConnection, plugin::AudioPlugin, track::audio_waveform::AudioWaveform,
        GeneratorId,
    },
    shared::{
        id::{BusId, EffectId, TrackId},
        PatternId,
    },
};

pub enum AudioCommand {
    // =============================
    // Transport Command
    // =============================
    PlayOneShot(AudioWaveform),
    StopAllPreviews,
    /// Set playback state (play/pause)
    SetPlaying(bool),
    TogglePlayingWithPlaybackMode(PlaybackMode),
    TogglePatternPlayback {
        pattern_id: PatternId,
        generator_id: GeneratorId,
    },
    /// Set loop mode
    SetLooping(bool),
    /// Stop playback and reset playhead to 0
    StopAndReset,
    SetPlayhead(u32),
    PlayPreviewNote {
        note_key: u8,
        generator_id: GeneratorId,
        velocity: u8,
        is_note_on: bool,
    },
    /// Set BPM to the field0 value
    SetBPM(f32),
    SetPlaybackMode(PlaybackMode),

    /// Hot-swaps the active generator while pattern playback is running
    SwitchPatternGenerator(crate::shared::id::GeneratorId),

    // =========================================================================
    // Generator Plugin Commands
    // =========================================================================
    /// Add a generator plugin to the audio thread
    AddGenerator {
        generator_id: GeneratorId,
        track_id: TrackId,
        plugin: Box<dyn AudioPlugin + Send>,
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
    /// Add an effect to the target's effect chain
    AddEffect {
        target: EffectTarget,
        effect_id: EffectId,
        effect: Box<dyn AudioPlugin + Send + Sync>,
    },
    /// Remove an effect from the target's effect chain
    RemoveEffect {
        target: EffectTarget,
        effect_id: EffectId,
    },
    /// Set a parameter on a target effect
    SetEffectParameter {
        target: EffectTarget,
        effect_id: EffectId,
        param_id: u32,
        value: f32,
    },
    /// Request parameter feedback for a target effect (triggers EffectParameterSnapshot response)
    QueryEffectParameters {
        target: EffectTarget,
        effect_id: EffectId,
    },



    /// Request a ZeroCopyBuffer from a specific plugin
    QueryZeroCopyBuffer {
        target: PluginTarget,
        name: String,
        request_id: u32,
    },
    
    // =====================================================
    // Bus Commands
    // =====================================================
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

    /// Update the routing matrix (sync from main thread)
    UpdateRouting {
        routing: Vec<RoutingConnection>,
    },
    /// Prepare all of plugins from ApplicationState to AudioEngine (upon loading project)
    PreparePlugin {
        track_effects: IndexMap<TrackId, IndexMap<EffectId, Box<dyn AudioPlugin + Send + Sync>>>,
        master_effects: IndexMap<EffectId, Box<dyn AudioPlugin + Send + Sync>>,
        bus_effects: IndexMap<BusId, IndexMap<EffectId, Box<dyn AudioPlugin + Send + Sync>>>,
        generators: IndexMap<GeneratorId, Box<dyn AudioPlugin + Send + Sync>>,
    },
    SetMetronomeActive(bool),

    /// Send command to execute command from real-time plugin state. to use this
    /// effectively, use a message passing channel for real-time data streaming
    ExecutePluginCommand {
        target: PluginTarget,
        command: String,
        payload: serde_json::Value,
        request_id: u32,
    },

    //////////////////////////////////////////
    // Load and Save related commands
    /////////////////////////////////////////
    /// Tell a specific plugin to overwrite its internal state
    SetPluginState {
        target: PluginTarget,
        state: Vec<u8>,
    },

    /// Ask the engine to provide the latest state of a specific plugin
    QueryPluginState {
        target: PluginTarget,
        request_id: u32, // To track the response in the UI
    },
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
    // Generator Feedback
    /// Single parameter changed (e.g., automation moved it)
    GeneratorParameterChanged(GeneratorParameterUpdate),
    /// Full parameter snapshot in response to query
    GeneratorParameterSnapshot(GeneratorParameterSnapshot),

    // Effect Feedback
    /// Single parameter changed on an effect (e.g., automation moved it)
    EffectParameterChanged(EffectParameterUpdate),
    /// Full parameter snapshot for an effect in response to query
    EffectParameterSnapshot(EffectParameterSnapshot),

    // Command Response Feedback
    PluginCommandResponse {
        request_id: u32,
        response: serde_json::Value,
    },

    /// The engine returning the requested state blob
    PluginStateSnapshot {
        target: PluginTarget,
        state: Vec<u8>,
        request_id: u32,
    },

    ZeroCopyBufferResponse {
        request_id: u32,
        buffer: Option<ZeroCopyBuffer>, // Wrap it in your opaque type
    },
}
