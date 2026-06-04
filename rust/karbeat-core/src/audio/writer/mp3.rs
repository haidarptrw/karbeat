use std::{fs::File, io::Write, mem::MaybeUninit, slice};

use crate::audio::writer::{AudioWriter, BitDepth};
use anyhow::Context;
use mp3lame_encoder::{Bitrate, Builder, DualPcm, Encoder, FlushNoGap, Id3Tag, Quality};

pub struct Mp3AudioWriter {
    pub encoder: Encoder,
    pub mp3_out_buffer: Vec<u8>,
    pub output_path: String,
    pub channels: u8,
}

impl TryFrom<BitDepth> for mp3lame_encoder::Bitrate {
    type Error = anyhow::Error;

    fn try_from(value: BitDepth) -> Result<Self, Self::Error> {
        match value {
            BitDepth::BitPerSample(_) => {
                return Err(anyhow::anyhow!("MP3 used Bit/second, not Bit/sample"));
            }
            BitDepth::BitPerSecond(bit_per_second) => {
                let lame_encoder = match bit_per_second {
                    super::BitPerSecond::Kbps128 => mp3lame_encoder::Bitrate::Kbps128,
                    super::BitPerSecond::Kbps160 => mp3lame_encoder::Bitrate::Kbps160,
                    super::BitPerSecond::Kbps192 => mp3lame_encoder::Bitrate::Kbps192,
                    super::BitPerSecond::Kbps256 => mp3lame_encoder::Bitrate::Kbps256,
                    super::BitPerSecond::Kbps320 => mp3lame_encoder::Bitrate::Kbps320,
                };

                Ok(lame_encoder)
            }
        }
    }
}

pub struct Mp3AudioWriterConfig<'a> {
    pub id3_tag: Option<Id3Tag<'a>>,
    pub sample_rate: u32,
    pub num_channels: u8,
    pub bit_rate: Bitrate,
    pub quality: Quality,
}

impl<'a> Mp3AudioWriterConfig<'a> {
    pub fn try_new(
        sample_rate: u32,
        num_channels: u8,
        bit_depth: BitDepth,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            id3_tag: None,
            sample_rate,
            num_channels,
            bit_rate: bit_depth.try_into()?,
            quality: Quality::Best,
        })
    }
}

impl Mp3AudioWriter {
    pub fn try_new(
        output_path: impl Into<String>,
        config: Mp3AudioWriterConfig<'_>,
    ) -> anyhow::Result<Self> {
        let mut builder =
            Builder::new().ok_or_else(|| anyhow::anyhow!("Error creating Mp3 Writer Builder"))?;
        builder
            .set_num_channels(config.num_channels)
            .map_err(|e| anyhow::anyhow!("Set channels error: {:?}", e))?;
        builder
            .set_sample_rate(config.sample_rate)
            .map_err(|e| anyhow::anyhow!("Set sample rate error: {:?}", e))?;
        builder
            .set_brate(config.bit_rate)
            .map_err(|e| anyhow::anyhow!("Set bitrate error: {:?}", e))?;
        builder
            .set_quality(config.quality)
            .map_err(|e| anyhow::anyhow!("Set quality error: {:?}", e))?;

        if let Some(tag) = config.id3_tag {
            let _ = builder.set_id3_tag(tag);
        }

        let encoder = builder.build().map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(Self {
            encoder,
            mp3_out_buffer: Vec::new(),
            output_path: output_path.into(),
            channels: config.num_channels,
        })
    }
}

impl AudioWriter for Mp3AudioWriter {
    fn write(&mut self, samples: &[f32]) -> anyhow::Result<()> {
        if samples.is_empty() {
            return Ok(());
        }

        let mut left = Vec::with_capacity(samples.len() / (self.channels as usize));
        let mut right = Vec::with_capacity(samples.len() / (self.channels as usize));

        if self.channels == 2 {
            for chunk in samples.chunks_exact(2) {
                left.push(chunk[0]);
                right.push(chunk[1]);
            }
        } else {
            // If mono, feed the same signal to both channels safely
            left.extend_from_slice(samples);
            right.extend_from_slice(samples);
        }

        let input = DualPcm {
            left: &left,
            right: &right,
        };

        // Pre-allocate required space before encoding
        let req_size = mp3lame_encoder::max_required_buffer_size(left.len());
        self.mp3_out_buffer.reserve(req_size);

        // Encode and append to our master buffer
        let encoded_size = self
            .encoder
            .encode(input, self.mp3_out_buffer.spare_capacity_mut())
            .map_err(|e| anyhow::anyhow!("Encoding error: {:?}", e))?;

        unsafe {
            let new_len = self.mp3_out_buffer.len().wrapping_add(encoded_size);
            self.mp3_out_buffer.set_len(new_len);
        }

        Ok(())
    }

    fn finalize(&mut self) -> anyhow::Result<()> {
        // Flush any remaining data from the encoder without gaps
        self.mp3_out_buffer
            .reserve(mp3lame_encoder::max_required_buffer_size(0));
        let encoded_size = self
            .encoder
            .flush::<FlushNoGap>(self.mp3_out_buffer.spare_capacity_mut())
            .map_err(|e| anyhow::anyhow!("Flush error: {:?}", e))?;

        unsafe {
            let new_len = self.mp3_out_buffer.len().wrapping_add(encoded_size);
            self.mp3_out_buffer.set_len(new_len);
        }

        // Open output file for the final write
        let mut file = File::create(&self.output_path)
            .with_context(|| format!("Failed to create output file: {}", self.output_path))?;

        // Insert VBR/LAME tags and write the actual file
        if self.encoder.lame_tag_size() > 0 {
            let id3v2_tag_boundary = self.encoder.id3v2_tag_size();
            let mut lame_tag = [MaybeUninit::uninit(); 1024];

            let lame_tag_size = self
                .encoder
                .lame_tag_encode(&mut lame_tag)
                .ok_or_else(|| anyhow::anyhow!("Failed to encode LAME tag because it is empty"))?;

            let lame_tag_slice = unsafe {
                slice::from_raw_parts(lame_tag.as_ptr() as *const u8, lame_tag_size.get())
            };

            // Splice the file together: [ID3v2] -> [Lame Tag] -> [Audio Data]
            file.write_all(&self.mp3_out_buffer[..id3v2_tag_boundary])?;
            file.write_all(lame_tag_slice)?;
            file.write_all(&self.mp3_out_buffer[id3v2_tag_boundary..])?;
        } else {
            // No lame tags required, just dump the buffer
            file.write_all(&self.mp3_out_buffer)?;
        }

        Ok(())
    }
}
