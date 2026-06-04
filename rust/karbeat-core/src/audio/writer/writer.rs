use anyhow::{anyhow, Result};
use derive_builder::Builder;
use std::{error::Error, path::Path};

use crate::audio::writer::{
    mp3::{Mp3AudioWriter, Mp3AudioWriterConfig},
    wav,
};

#[derive(Clone, Copy, Debug)]
#[repr(u16)]
pub enum BitPerSample {
    B8 = 8,
    B16 = 16,
    B24 = 24,
    B32 = 32,
}

impl BitPerSample {
    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Debug)]
pub struct BitPerSampleError;

impl std::fmt::Display for BitPerSampleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Not a valid BitPerSample value")
    }
}

impl std::error::Error for BitPerSampleError {}

impl TryFrom<u16> for BitPerSample {
    type Error = BitPerSampleError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            8 => Ok(BitPerSample::B8),
            16 => Ok(BitPerSample::B16),
            24 => Ok(BitPerSample::B24),
            32 => Ok(BitPerSample::B32),
            _ => Err(BitPerSampleError),
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum BitPerSecond {
    Kbps128 = 128,
    Kbps160 = 160,
    Kbps192 = 192,
    Kbps256 = 256,
    Kbps320 = 320,
}

impl BitPerSecond {
    pub fn as_kbps(self) -> u32 {
        self as u32
    }
}

/// Unified configuration payload for any supported audio exporter
pub enum AudioExportConfig {
    Wav(WavAudioWriterConfig),
    Mp3 {
        sample_rate: u32,
        channels: u8,
        bit_depth: BitDepth,
    },
    // Flac(FlacAudioWriterConfig),
    // Ogg(OggAudioWriterConfig),
}

impl AudioExportConfig {
    /// Extracts the target sample rate for the audio engine
    pub fn sample_rate(&self) -> u32 {
        match self {
            Self::Wav(config) => config.sample_rate,
            Self::Mp3 { sample_rate, .. } => *sample_rate,
        }
    }

    /// Extracts the target channel count for the audio engine
    pub fn channels(&self) -> u16 {
        match self {
            Self::Wav(config) => config.channels,
            Self::Mp3 { channels, .. } => *channels as u16,
        }
    }
}

#[derive(Debug)]
pub struct BitPerSecondError;

impl std::fmt::Display for BitPerSecondError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Not a valid BitPerSecond value")
    }
}

impl std::error::Error for BitPerSecondError {}

impl TryFrom<u16> for BitPerSecond {
    type Error = BitPerSecondError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            128 => Ok(BitPerSecond::Kbps128),
            160 => Ok(BitPerSecond::Kbps160),
            192 => Ok(BitPerSecond::Kbps192),
            256 => Ok(BitPerSecond::Kbps256),
            320 => Ok(BitPerSecond::Kbps320),
            _ => Err(BitPerSecondError),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BitDepth {
    BitPerSample(BitPerSample),
    BitPerSecond(BitPerSecond),
}

impl BitDepth {
    pub fn try_new_bits_per_sample(bps: u16) -> Result<BitDepth, Box<dyn Error>> {
        let bits_per_sample: BitPerSample = bps.try_into()?;
        Ok(BitDepth::BitPerSample(bits_per_sample))
    }

    pub fn try_new_bits_per_sec(bps: u16) -> Result<BitDepth, Box<dyn Error>> {
        let bits_per_sec: BitPerSecond = bps.try_into()?;
        Ok(BitDepth::BitPerSecond(bits_per_sec))
    }
}

#[derive(Clone, Copy, Debug, Builder)]
/// Standard definition for audio metadata required by all encoders
pub struct WavAudioWriterConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_depth: BitDepth,
}
/// The common trait implemented by all format-specific writers
pub trait AudioWriter: Send {
    /// Writes interleaved f32 samples (e.g., [L, R, L, R])
    fn write(&mut self, samples: &[f32]) -> Result<()>;

    /// Flushes buffers and writes trailing file headers.
    /// Must be called before the writer is dropped.
    fn finalize(&mut self) -> Result<()>;
}

impl AudioWriter for Box<dyn AudioWriter> {
    fn write(&mut self, buffer: &[f32]) -> anyhow::Result<()> {
        (**self).write(buffer)
    }

    fn finalize(&mut self) -> anyhow::Result<()> {
        (**self).finalize()
    }
}

/// Factory function to create the appropriate writer based on file extension
pub fn create_writer(path: &Path, config: AudioExportConfig) -> Result<Box<dyn AudioWriter>> {
    match config {
        AudioExportConfig::Wav(format) => Ok(Box::new(wav::WavAudioWriter::new(path, format)?)),
        AudioExportConfig::Mp3 {
            sample_rate,
            channels,
            bit_depth,
        } => {
            let path_str = path
                .to_str()
                .ok_or_else(|| anyhow!("Invalid path for MP3 export"))?;
            let config = Mp3AudioWriterConfig::try_new(sample_rate, channels, bit_depth)?;
            Ok(Box::new(Mp3AudioWriter::try_new(path_str, config)?))
        }
    }
}

pub trait AudioExporter {
    fn config(&self) -> AudioExportConfig;
    fn writer(&self) -> Box<dyn AudioExporter>;
}
