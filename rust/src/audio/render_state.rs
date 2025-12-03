use std::collections::HashMap;

use crate::core::{project::{ApplicationState, KarbeatTrack, MixerChannel, Pattern}, track::audio_waveform::AudioWaveform};

#[derive(Default, Clone)]
pub struct AudioRenderState {
    pub is_playing: bool,
    pub tempo: f32, // BPM
    pub sample_rate: u32,

    // Flattened data for quick access
    pub tracks:  Vec<KarbeatTrack>,
    pub patterns: HashMap<u32, Pattern>,
    pub mixer_channels: HashMap<u32, MixerChannel>,
    pub assets: HashMap<u32, AudioWaveform>,
}

impl From<&ApplicationState> for AudioRenderState {
    fn from(app: &ApplicationState) -> Self {
        // 1. Flatten Tracks (Sort by ID to keep order deterministic)
        let mut tracks: Vec<KarbeatTrack> = app.tracks.values().cloned().collect();
        tracks.sort_by_key(|t| t.id);

        // 2. Map Assets
        // AssetLibrary holds Arc<AudioWaveform>, so cloning is cheap!
        let assets = app.asset_library.source_map.iter()
            .map(|(k, v)| (*k, (**v).clone()))
            .collect();

        Self {
            is_playing: app.transport.is_playing,
            tempo: app.metadata.bpm,
            sample_rate: app.audio_config.sample_rate,
            tracks,
            patterns: app.pattern_pool.clone(),
            mixer_channels: app.mixer.channels.clone(),
            assets,
        }
    }
}