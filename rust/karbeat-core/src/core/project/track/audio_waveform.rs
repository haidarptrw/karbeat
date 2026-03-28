use karbeat_utils::define_id;

use serde::{Deserialize, Serialize};

use crate::core::project::PluginInstance;

pub type AudioFrame = [f32; 2];

define_id!(AudioSourceId);

use memmap2::Mmap;
/// Audio Waveform data of an audio sample
use std::{path::PathBuf, sync::Arc};

// STATIC global variables for waveform mipmaps

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AudioWaveform {
    pub id: Option<AudioSourceId>,
    /// Audio buffer of samples
    #[serde(skip)]
    pub buffer: Option<Arc<Mmap>>, // future update: replace this with Arc<[f32]> for better performance
    /// path to the audio source file
    pub file_path: PathBuf,
    /// name of the audio waveform
    pub name: String,
    /// Sample rate of the audio waveform
    pub sample_rate: u32,
    /// Number of channels of the audio waveform
    pub channels: u16,
    /// duration of the entire audio waveform in seconds
    pub duration: f64,
    /// Root note of the audio waveform
    pub root_note: u8,
    /// Fine tune of the audio waveform
    pub fine_tune: i16,
    /// Start of the audio waveform in samples
    pub trim_start: u32,
    /// End of the audio waveform in samples
    pub trim_end: u32,
    /// Whether the audio waveform is looping
    pub is_looping: bool,
    /// Whether the audio waveform is normalized
    pub normalized: bool,
    /// Whether the audio waveform is muted
    pub muted: bool,

    /// Effects applied to the audio waveform
    pub effects: Arc<Vec<PluginInstance>>,
}

impl Default for AudioWaveform {
    fn default() -> Self {
        Self {
            id: None,
            buffer: None,
            file_path: PathBuf::new(),
            name: "Sample".to_string(),
            sample_rate: 44100,
            channels: 2,
            duration: 0.0,
            root_note: 60, // C5
            fine_tune: 0,
            trim_start: 0,
            trim_end: 0,
            is_looping: false,
            normalized: false,
            muted: false,
            effects: Default::default(),
        }
    }
}

impl AudioWaveform {
    pub fn try_assign_id(&mut self, id: AudioSourceId) -> anyhow::Result<()> {
        if self.id.is_some() {
            return Err(anyhow::anyhow!("Audio waveform already has an ID"));
        }
        self.id = Some(id);
        Ok(())
    }
}
