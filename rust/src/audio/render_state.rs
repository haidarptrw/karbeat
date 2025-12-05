use std::{collections::HashMap, sync::Arc};

use crate::{
    core::project::{ApplicationState, AssetLibrary, KarbeatTrack, MixerChannel, MixerState, Pattern},
    utils::math::is_power_of_two,
};

#[derive(Default, Clone)]
pub struct AudioRenderState {
    pub is_playing: bool,
    pub is_looping: bool,
    pub tempo: f32, // BPM
    pub sample_rate: u32,
    pub buffer_size: usize,

    // Flattened data for quick access
    pub tracks: Vec<Arc<KarbeatTrack>>,
    pub patterns: HashMap<u32, Arc<Pattern>>,
    pub mixer_state: MixerState,
    pub asset_library: Arc<AssetLibrary>,
}

impl From<&ApplicationState> for AudioRenderState {
    fn from(app: &ApplicationState) -> Self {
        let mut tracks: Vec<Arc<KarbeatTrack>> = app.tracks.values().cloned().collect();
        tracks.sort_by_key(|t| t.id);

        Self {
            is_playing: app.transport.is_playing,
            is_looping: app.transport.is_looping,
            tempo: app.transport.bpm,
            sample_rate: app.audio_config.sample_rate,
            tracks,
            patterns: app.pattern_pool.clone(),
            mixer_state: app.mixer.clone(),
            asset_library: app.asset_library.clone(),
            buffer_size: if is_power_of_two(app.audio_config.buffer_size.into()) {
                app.audio_config.buffer_size as usize
            } else {
                64
            },
        }
    }
}
