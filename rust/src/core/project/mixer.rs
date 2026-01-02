use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::core::project::{PluginInstance, TrackId};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct MixerState {
    // Map Track ID -> Mixer Channel
    pub channels: HashMap<TrackId, Arc<MixerChannel>>,
    pub master_bus: Arc<MixerChannel>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MixerChannel {
    pub volume: f32, // 0.0 to 1.0 (or dB)
    pub pan: f32,    // -1.0 to 1.0
    pub mute: bool,
    pub solo: bool,
    pub inverted_phase: bool,

    // The effects chain (EQ, Compressor) comes AFTER the generator
    pub effects: Arc<[PluginInstance]>,
}

impl Default for MixerChannel {
    fn default() -> Self {
        Self {
            volume: 0.0,
            pan: 0.0,
            mute: Default::default(),
            solo: Default::default(),
            inverted_phase: Default::default(),
            effects: Arc::from(Vec::new()),
        }
    }
}

