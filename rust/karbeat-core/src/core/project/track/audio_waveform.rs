use serde::{Deserialize, Serialize};

use crate::shared::id::AudioSourceId;

pub type AudioFrame = [f32; 2];

use memmap2::Mmap;
/// Audio Waveform data of an audio sample
use std::{path::PathBuf, sync::Arc};

// STATIC global variables for waveform mipmaps

/// ======================================
/// AudioSampleMode
/// Determines how an audio clip's timeline position is interpreted.
/// - Default: raw samples (BPM-independent, for standard audio playback)
/// - Stretch: ticks (BPM-dependent, for time-stretched audio — future feature)
/// ======================================
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum AudioSampleMode {
    /// raw samples (BPM-independent, for standard audio playback)
    #[default]
    Default,
    /// ticks (BPM-dependent, for time-stretched audio, preserved pitch)
    Stretch,
    /// ticks (BPM-dependent, for time-stretched audio, do not preserve pitch)
    Resampled,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AudioWaveform {
    pub id: Option<AudioSourceId>,
    /// Audio buffer of samples
    #[serde(skip)]
    pub buffer: Option<Arc<Mmap>>,
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
    /// The original BPM of the sample (needed for Stretch and Sampled modes)
    pub original_bpm: f32,
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
    /// How this audio source maps to the timeline (raw samples vs tempo-locked ticks)
    pub sample_mode: AudioSampleMode,
}

impl PartialEq for AudioWaveform {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.file_path == other.file_path
            && self.name == other.name
            && self.sample_rate == other.sample_rate
            && self.channels == other.channels
            && self.duration == other.duration
            && self.root_note == other.root_note
            && self.fine_tune == other.fine_tune
            && self.trim_start == other.trim_start
            && self.trim_end == other.trim_end
            && self.is_looping == other.is_looping
            && self.normalized == other.normalized
            && self.muted == other.muted
            && self.sample_mode == other.sample_mode
            && self.original_bpm == other.original_bpm
    }
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
            root_note: 60, // C4
            fine_tune: 0,
            trim_start: 0,
            trim_end: 0,
            is_looping: false,
            normalized: false,
            muted: false,
            sample_mode: AudioSampleMode::Default,
            original_bpm: 120.0,
        }
    }
}

/// A context struct providing the audio engine with everything it needs
/// to render the waveform correctly based on the current sample mode.
pub struct WaveformPlaybackContext<'a> {
    /// The trimmed, ready-to-read audio buffer slice
    pub buffer: &'a [f32],
    /// Multiplier for the read-pointer speed (e.g., 2.0 = play twice as fast)
    pub playback_rate: f64,
    /// Multiplier for the pitch (e.g., 2.0 = one octave up).
    /// If playback_rate != pitch_rate, the engine must use a time-stretching algorithm.
    pub pitch_ratio: f64,
    /// The mode, passed along just in case the engine needs algorithmic context
    pub mode: AudioSampleMode,
}

impl AudioWaveform {
    pub fn try_assign_id(&mut self, id: AudioSourceId) -> anyhow::Result<()> {
        if self.id.is_some() {
            return Err(anyhow::anyhow!("Audio waveform already has an ID"));
        }
        self.id = Some(id);
        Ok(())
    }

    /// Returns a strictly valid slice of the audio buffer, cropped exactly to
    /// the trim_start and trim_end boundaries.
    pub fn get_playable_buffer<'a>(&'a self) -> Option<&'a [f32]> {
        let raw_buffer = crate::utils::get_waveform_buffer(&self.buffer)?;
        let channels = self.channels as usize;

        if channels == 0 || raw_buffer.is_empty() {
            return None;
        }

        let total_frames = raw_buffer.len() / channels;

        let start_frame = (self.trim_start as usize).min(total_frames);
        let end_frame = if self.trim_end > 0 {
            (self.trim_end as usize).min(total_frames).max(start_frame)
        } else {
            total_frames
        };

        let start_idx = start_frame * channels;
        let end_idx = end_frame * channels;

        if start_idx >= raw_buffer.len() || end_idx > raw_buffer.len() || start_idx >= end_idx {
            return None;
        }

        Some(&raw_buffer[start_idx..end_idx])
    }

    /// Abstracts the logic of AudioSampleModes. The audio engine passes the project's
    /// current BPM, and this returns the exact parameters needed to render the audio.
    pub fn get_playback_context<'a>(
        &'a self,
        project_bpm: f32,
    ) -> Option<WaveformPlaybackContext<'a>> {
        let buffer = self.get_playable_buffer()?;

        let (playback_rate, pitch_ratio) = match self.sample_mode {
            // Default: Ignores BPM entirely. Plays 1:1 speed, original pitch.
            AudioSampleMode::Default => (1.0, 1.0),

            // Stretch: Speeds up/slows down to match project BPM, but pitch remains 1.0
            // (Engine must use phase vocoder/granular synthesis to accommodate this)
            AudioSampleMode::Stretch => {
                let ratio = (project_bpm / self.original_bpm.max(1.0)) as f64;
                (ratio, 1.0)
            }

            // Resampled: Speeds up/slows down to match project BPM, pitch bends with it
            // (Classic turntable/tape effect. Engine just reads faster/slower)
            AudioSampleMode::Resampled => {
                let ratio = (project_bpm / self.original_bpm.max(1.0)) as f64;
                (ratio, ratio)
            }
        };

        Some(WaveformPlaybackContext {
            buffer,
            playback_rate,
            pitch_ratio,
            mode: self.sample_mode.clone(),
        })
    }
}
