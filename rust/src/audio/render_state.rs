use std::{collections::HashMap, sync::Arc};

use crate::core::{project::{ApplicationState, AssetLibrary, KarbeatTrack, MixerChannel, Pattern}, track::audio_waveform::AudioWaveform};

#[derive(Default, Clone)]
pub struct AudioRenderState {
    pub is_playing: bool,
    pub tempo: f32, // BPM
    pub sample_rate: u32,

    // Flattened data for quick access
    pub tracks:  Vec<Arc<KarbeatTrack>>,
    pub patterns: HashMap<u32, Arc<Pattern>>,
    pub mixer_channels: HashMap<u32, Arc<MixerChannel>>,
    pub master_bus: Arc<MixerChannel>,
    pub asset_library: Arc<AssetLibrary>,
}

impl From<&ApplicationState> for AudioRenderState {
fn from(app: &ApplicationState) -> Self {
        // 1. Flatten Tracks (Sort by ID to keep order deterministic)
        // FIX: Changed type from Vec<KarbeatTrack> to Vec<Arc<KarbeatTrack>>
        let mut tracks: Vec<Arc<KarbeatTrack>> = app.tracks.values().cloned().collect();
        tracks.sort_by_key(|t| t.id);

        Self {
            is_playing: app.transport.is_playing,
            tempo: app.metadata.bpm,
            sample_rate: app.audio_config.sample_rate,
            tracks,
            patterns: app.pattern_pool.clone(),
            mixer_channels: app.mixer.channels.clone(),
            master_bus: app.mixer.master_bus.clone(),
            asset_library: app.asset_library.clone(),
        }
    }
}