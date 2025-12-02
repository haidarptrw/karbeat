use std::collections::HashMap;

use crate::core::{project::{KarbeatTrack, MixerChannel, Pattern}, track::audio_waveform::AudioWaveform};

#[derive(Default, Clone)]
pub struct AudioRenderState {
    pub is_playing: bool,
    pub tempo: f32, // BPM

    // Flattened data for quick access
    pub tracks:  Vec<KarbeatTrack>,
    pub patterns: HashMap<u32, Pattern>,
    pub mixer_channels: HashMap<u32, MixerChannel>,
    pub assets: HashMap<u32, AudioWaveform>,
}