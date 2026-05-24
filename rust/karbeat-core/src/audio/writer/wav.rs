
use super::{WavAudioWriterConfig, AudioWriter, BitDepth};
use anyhow::{Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{fs::File, io::BufWriter, path::Path};

pub struct WavAudioWriter {
    writer: Option<WavWriter<BufWriter<File>>>,
    spec: WavSpec,
}

impl WavAudioWriter {
    pub fn new(path: &Path, format: WavAudioWriterConfig) -> Result<Self> {
        let bits_per_sample = match format.bit_depth {
            BitDepth::BitPerSample(bps) => bps.as_u16(),
            BitDepth::BitPerSecond(_) => {
                anyhow::bail!("BitPerSecond is not supported for WAV format")
            }
        };

        let sample_format = if bits_per_sample == 32 {
            SampleFormat::Float
        } else {
            SampleFormat::Int
        };

        let spec = WavSpec {
            channels: format.channels,
            sample_rate: format.sample_rate,
            bits_per_sample,
            sample_format,
        };

        let writer = WavWriter::create(path, spec)
            .with_context(|| format!("Failed to create WAV file at {:?}", path))?;

        Ok(Self {
            writer: Some(writer),
            spec,
        })
    }
}

impl AudioWriter for WavAudioWriter {
    fn write(&mut self, samples: &[f32]) -> Result<()> {
        let writer = self.writer.as_mut().context("Writer already finalized")?;

        match self.spec.sample_format {
            SampleFormat::Float => {
                // For 32-bit float, we can write the f32 samples directly
                for &sample in samples {
                    writer.write_sample(sample.clamp(-1.0, 1.0))?;
                }
            }
            SampleFormat::Int => {
                // For Integer formats, we must scale the f32 [-1.0, 1.0] range to the integer bounds
                match self.spec.bits_per_sample {
                    16 => {
                        let multiplier = i16::MAX as f32; // 32767.0
                        for &sample in samples {
                            let clamped = sample.clamp(-1.0, 1.0);
                            let int_sample = (clamped * multiplier) as i16;
                            writer.write_sample(int_sample)?;
                        }
                    }
                    24 => {
                        // 24-bit is written using i32, and Hound truncates it to 3 bytes internally.
                        let multiplier = 8_388_607.0; // 2^23 - 1
                        for &sample in samples {
                            let clamped = sample.clamp(-1.0, 1.0);
                            let int_sample = (clamped * multiplier) as i32;
                            writer.write_sample(int_sample)?;
                        }
                    }
                    8 => {
                        let multiplier = i8::MAX as f32; // 127.0
                        for &sample in samples {
                            let clamped = sample.clamp(-1.0, 1.0);
                            let int_sample = (clamped * multiplier) as i8;
                            writer.write_sample(int_sample)?;
                        }
                    }
                    _ => anyhow::bail!(
                        "Unsupported bit depth for integer format: {}",
                        self.spec.bits_per_sample
                    ),
                }
            }
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
