use hashbrown::HashMap;
use indexmap::IndexMap;
use karbeat_plugin_api::types::ZeroCopyBuffer;
use std::sync::Arc;

use crate::{
    audio::{
        engine::PlaybackMode,
        event::PluginTarget,
        render_state::{AudioAutomationLane, AudioGraphState},
    },
    core::project::{
        mixer::{MixerChannelParams, RoutingConnection},
        plugin::AudioPlugin,
        track::{audio_waveform::AudioWaveform, midi::Pattern, AudioTrack},
        GeneratorId, ModulationLink, ModulationSource,
    },
    shared::{id::*, AutomationId, ModulationId, PatternId},
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
    // Mixer Channel Parameter Commands
    // =====================================================
    /// Set a single DSP parameter on a mixer channel (track, bus, or master).
    /// The audio thread is the sole owner of these values; AppState is only
    /// written back during save_project.
    SetMixerChannelParameter {
        target: MixerChannelTarget,
        param: MixerChannelParams,
    },

    /// Request a full snapshot of a mixer channel's current DSP state.
    /// The audio thread responds with AudioFeedback::MixerChannelSnapshot.
    QueryMixerChannel {
        target: MixerChannelTarget,
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

    /// Update the routing matrix directly on the audio thread.
    /// This bypasses the triple-buffer; routing is owned by the ring-buffer path.
    UpdateRouting {
        routing: Vec<RoutingConnection>,
    },
    /// Prepare all plugins and seed the audio thread's mixer channel state.
    /// Called on project load / new project to fully hydrate the audio thread.
    HydratePlugin {
        track_effects: IndexMap<TrackId, IndexMap<EffectId, Box<dyn AudioPlugin + Send + Sync>>>,
        master_effects: IndexMap<EffectId, Box<dyn AudioPlugin + Send + Sync>>,
        bus_effects: IndexMap<BusId, IndexMap<EffectId, Box<dyn AudioPlugin + Send + Sync>>>,
        generators: IndexMap<GeneratorId, Box<dyn AudioPlugin + Send + Sync>>,
        /// Initial DSP values for every track channel (volume, pan, mute, solo, inverted_phase)
        track_channels: IndexMap<TrackId, MixerChannelSeed>,
        /// Initial DSP values for every bus channel
        bus_channels: IndexMap<BusId, MixerChannelSeed>,
        /// Initial DSP values for the master channel
        master_channel: MixerChannelSeed,
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

    /// Ask the engine to get the copied version of the latest engine snapshot.
    /// This is only used when exporting a project into a sound file.
    /// The export engine is cloned from the live engine's own internal state
    /// (no triple-buffer involved).
    QueryAudioEngine {
        command_consumer: rtrb::Consumer<AudioCommand>,
        position_producer: rtrb::Producer<crate::audio::event::TransportFeedback>,
        feedback_producer: rtrb::Producer<crate::commands::AudioFeedback>,
        response_tx: std::sync::mpsc::Sender<Box<crate::audio::engine::AudioEngine>>,
    },

    // =========================================================================
    // Granular Graph-State Commands (replace the old triple-buffer path)
    // =========================================================================
    /// Update the track + pattern snapshot on the audio thread.
    /// Sent whenever tracks are added/removed, clips are edited, patterns
    /// are modified, or BPM changes the max_sample_index.
    UpdateTrackGraph {
        tracks: Arc<[Arc<AudioTrack>]>,
        patterns: HashMap<PatternId, Arc<Pattern>>,
        max_sample_index: u32,
    },

    /// Add or replace a single automation lane on the audio thread.
    /// Sent on add/remove automation point, lane enable toggle, or lane metadata update.
    UpdateAutomationLane {
        id: AutomationId,
        lane: AudioAutomationLane,
    },

    /// Remove an automation lane from the audio thread's local graph.
    RemoveAutomationLane {
        id: AutomationId,
    },

    /// Update audio engine config (sample_rate, buffer_size).
    /// Sent when the audio device is reconfigured at runtime.
    UpdateAudioConfig {
        sample_rate: u32,
        buffer_size: usize,
    },

    /// Atomically replace the full audio graph state on the engine.
    /// Used exclusively for undo/redo where an arbitrary subset of sub-graphs
    /// may have changed and enumerating diffs would be impractical.
    ReplaceFullGraph {
        graph: AudioGraphState,
    },

    /// Spawn a new generator in the DSP thread (e.g., an LFO)
    AddModulationSource {
        id: ModulationId,
        source: ModulationSource,
    },
    RemoveModulationSource(ModulationId),

    /// Plug a cable from a Generator to a Target
    AddModulationLink {
        id: ModulationLinkId,
        link: ModulationLink,
    },
    UpdateModulationLinkDepth {
        id: ModulationLinkId,
        depth: f32,
    },
    RemoveModulationLink(ModulationLinkId),
}

// ============================================================================
// Audio → UI Feedback Messages
// ============================================================================

// ======================================
// MixerChannelTarget
// ======================================

/// Identifies which mixer channel a command or snapshot applies to.
#[derive(Clone, Debug, PartialEq)]
pub enum MixerChannelTarget {
    Track(TrackId),
    Bus(BusId),
    Master,
}

/// Lightweight plain-value snapshot used to seed the audio thread's mixer
/// state on project load, and written back to AppState during save_project.
#[derive(Clone, Debug, Default)]
pub struct MixerChannelSeed {
    pub volume: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub inverted_phase: bool,
}

// ======================================
// EffectTarget
// ======================================

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

/// Full DSP state snapshot of a single mixer channel.
/// Sent by the audio thread in response to QueryMixerChannel.
#[derive(Clone, Debug)]
pub struct MixerChannelSnapshot {
    pub target: MixerChannelTarget,
    pub volume: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub inverted_phase: bool,
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

    // ======================================
    // Mixer Channel Feedback
    // ======================================
    /// Full DSP state snapshot for a mixer channel (response to QueryMixerChannel)
    MixerChannelSnapshot(MixerChannelSnapshot),

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
