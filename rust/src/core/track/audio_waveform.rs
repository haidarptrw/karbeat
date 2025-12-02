use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub type AudioFrame = [f32; 2];

#[derive(Clone, Serialize, Deserialize)]
pub struct AudioWaveform {
    #[serde(skip)]
    pub buffer: Arc<Vec<f32>>,
    pub file_path: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration: f64,
    pub root_note: u8,
    pub fine_tune: i16,
    pub trim_start: u64,
    pub trim_end: u64,
    pub is_looping: bool,
    pub normalized: bool,
}


impl Default for AudioWaveform {
    fn default() -> Self {
        Self { 
            buffer: Arc::new(Vec::new()),
            file_path: String::new(),
            sample_rate: 44100, 
            channels: 2, 
            duration: 0.0, 
            root_note: 60, // C5
            fine_tune: 0, 
            trim_start: 0, 
            trim_end: 0, 
            is_looping: false, 
            normalized: false,
        }
    }
}

// UI Data Structure for Audio Waveform window information (to change vol, pitch fine tune, normalization, panning, adsr envelope, 
// play the audio when pressing the waveform etc)

pub struct AudioWaveformUi {
    pub waveform: AudioWaveform
}