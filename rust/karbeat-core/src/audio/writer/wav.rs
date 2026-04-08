use super::{AudioFormat, AudioWriter};
use anyhow::{Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{fs::File, io::BufWriter, path::Path};

pub struct WavAudioWriter {
    // Wrapped in an Option so we can consume it in finalize()
    writer: Option<WavWriter<BufWriter<File>>>,
}

impl WavAudioWriter {
    pub fn new(path: &Path, format: AudioFormat) -> Result<Self> {
        let spec = WavSpec {
            channels: format.channels,
            sample_rate: format.sample_rate,
            bits_per_sample: format.bit_per_sample.as_u16(),
            sample_format: SampleFormat::Float,
        };

        let writer = WavWriter::create(path, spec)
            .with_context(|| format!("Failed to create WAV file at {:?}", path))?;

        Ok(Self {
            writer: Some(writer),
        })
    }
}

impl AudioWriter for WavAudioWriter {
    fn write(&mut self, samples: &[f32]) -> Result<()> {
        let writer = self.writer.as_mut().context("Writer already finalized")?;
        
        for &sample in samples {
            // Hard clamp to prevent WAV corruption on clipping
            writer.write_sample(sample.clamp(-1.0, 1.0))?;
        }
        Ok(())
    }

    fn finalize(&mut self) -> Result<()> {
        if let Some(writer) = self.writer.take() {
            writer.finalize().context("Failed to finalize WAV file")?;
        }
        Ok(())
    }
}