pub mod wav;
use anyhow::{anyhow, Result};
use std::path::Path;
use derive_builder::Builder;

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

#[derive(Clone, Copy, Debug, Builder)]
/// Standard definition for audio metadata required by all encoders
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_per_sample: BitPerSample,
}
/// The common trait implemented by all format-specific writers
pub trait AudioWriter: Send {
    /// Writes interleaved f32 samples (e.g., [L, R, L, R])
    fn write(&mut self, samples: &[f32]) -> Result<()>;
    
    /// Flushes buffers and writes trailing file headers. 
    /// Must be called before the writer is dropped.
    fn finalize(&mut self) -> Result<()>;
}

/// Factory function to create the appropriate writer based on file extension
pub fn create_writer(path: &Path, format: AudioFormat) -> Result<Box<dyn AudioWriter>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "wav" => Ok(Box::new(wav::WavAudioWriter::new(path, format)?)),
        "mp3" => todo!("Add MP3 Audio Writer here"),
        "flac" => todo!("Add FLAC Audio Writer here"),
        "ogg" => todo!("Add OGG Audio Writer here"),
        _ => Err(anyhow!("Unsupported file extension: .{}", ext)),
    }
}