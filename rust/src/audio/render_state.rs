use std::{collections::HashMap, sync::Arc};

use crate::{
    core::project::{ApplicationState, AssetLibrary, KarbeatTrack, MixerChannel, MixerState, Pattern, TransportState},
    utils::math::is_power_of_two,
};

/// Structural State: Tracks, Patterns, Mixer, Assets (Heavy, changes rarely)
#[derive(Default, Clone)]
pub struct AudioGraphState {
    pub tracks: Vec<Arc<KarbeatTrack>>,
    pub patterns: HashMap<u32, Arc<Pattern>>,
    pub mixer_state: MixerState,
    pub asset_library: Arc<AssetLibrary>,
    pub max_sample_index: u64,
    pub sample_rate: u32,
    pub buffer_size: usize,
}

impl From<&ApplicationState> for AudioGraphState {
    fn from(app: &ApplicationState) -> Self {
        let mut tracks: Vec<Arc<KarbeatTrack>> = app.tracks.values().cloned().collect();
        tracks.sort_by_key(|t| t.id);

        Self {
            tracks,
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