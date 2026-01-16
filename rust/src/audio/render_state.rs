use std::{collections::HashMap, sync::Arc};

use crate::{
    core::project::{
        mixer::{BusId, EffectId, MixerState},
        plugin::{KarbeatEffect, KarbeatGenerator},
        track::{
            midi::{Pattern, PatternId},
            KarbeatTrack,
        },
        transport::TransportState,
        ApplicationState, AssetLibrary, GeneratorId, TrackId,
    },
    utils::math::is_power_of_two,
};

// =============================================================================
// Audio Thread Owned Plugin State
// =============================================================================

/// A generator plugin instance owned by the audio thread
pub struct AudioGeneratorInstance {
    pub id: GeneratorId,
    pub track_id: TrackId,
    pub plugin: Box<dyn KarbeatGenerator + Send>,
}

pub struct AudioEffectInstance {
    pub id: EffectId,
    pub plugin: Box<dyn KarbeatEffect + Send>,
}

/// Audio thread's owned plugin instances - NO locks required for access
/// This is managed via AudioCommand, NOT cloned from ApplicationState
#[derive(Default)]
pub struct AudioPluginState {
    /// Generator plugins keyed by GeneratorId
    pub generators: HashMap<GeneratorId, AudioGeneratorInstance>,
    /// Effect chain per track (owned by audio thread)
    pub track_effects: HashMap<TrackId, Vec<AudioEffectInstance>>,
    /// Master effect chain (owned by audio thread)
    pub master_effects: Vec<AudioEffectInstance>,
    /// Bus effect chains (owned by audio thread)
    pub bus_effects: HashMap<BusId, Vec<AudioEffectInstance>>,
}

// =============================================================================
// Cloneable Graph State (metadata only, no plugin instances)
// =============================================================================

/// Structural State: Tracks, Patterns, Mixer, Assets (Heavy, changes rarely)
#[derive(Default, Clone)]
pub struct AudioGraphState {
    pub tracks: Arc<[Arc<KarbeatTrack>]>,
    pub patterns: HashMap<PatternId, Arc<Pattern>>,
    pub mixer_state: MixerState,
    pub asset_library: Arc<AssetLibrary>,
    pub max_sample_index: u32,
    pub sample_rate: u32,
    pub buffer_size: usize,
}

impl From<&ApplicationState> for AudioGraphState {
    fn from(app: &ApplicationState) -> Self {
        let mut tracks_vec: Vec<Arc<KarbeatTrack>> = app.tracks.values().cloned().collect();
        tracks_vec.sort_by_key(|t| t.id);

        Self {
            tracks: Arc::from(tracks_vec),
            patterns: app.pattern_pool.clone(),
            mixer_state: app.mixer.clone(),
            asset_library: app.asset_library.clone(),
            max_sample_index: app.max_sample_index,
            sample_rate: app.audio_config.sample_rate,
            buffer_size: if is_power_of_two(app.audio_config.buffer_size.into()) {
                app.audio_config.buffer_size as usize
            } else {
                64
            },
        }
    }
}

/// Consolidated State wrapper for the Audio Thread
#[derive(Clone)]
pub struct AudioRenderState {
    pub graph: AudioGraphState,
    // Transport is now separate to allow fast updates without full graph clone
    // However, for backward compatibility with your TripleBuffer setup,
    // we can keep a unified struct if your architecture requires a single atomic update.
    // If you implemented the split buffers (graph_in, transport_in), this struct is not needed as a monolith.
    // Assuming we stick to the monolithic struct for `state_consumer` in `AudioEngine`:
    pub transport: TransportState,
}

impl Default for AudioRenderState {
    fn default() -> Self {
        Self {
            graph: AudioGraphState::default(),
            transport: TransportState::default(),
        }
    }
}

impl From<&ApplicationState> for AudioRenderState {
    fn from(app: &ApplicationState) -> Self {
        Self {
            graph: AudioGraphState::from(app),
            transport: app.transport.clone(),
        }
    }
}
