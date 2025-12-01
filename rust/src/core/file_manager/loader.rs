// src/core/file_manager/loader.rs

// Source code of file loader

use std::{fs::File, path::Path, sync::Arc};

use anyhow::{anyhow, Context, Result};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    errors::Error as SymphoniaError,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

use crate::core::track::audio_waveform::AudioWaveform;

pub fn load_audio_file(path_string: String) -> Result<AudioWaveform> {
    let path = Path::new(&path_string);
    let src =
        File::open(path).with_context(|| format!("Failed to open audio file: {}", path_string))?;

    // Create a media source
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    // Detect the media format
    let mut hint = Hint::new();
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            hint.with_extension(ext_str);
        }
    };

    // Use the default options for metadata and format readers.
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    // Probe the media source
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .context("unsupported format")?;

    // Select a track
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("no supported audio tracks")?;

    // use default options for the decoder
    let dec_opts: DecoderOptions = Default::default();

    // assign some parameters for the return result
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let channel_count = track.codec_params.channels.unwrap_or_default().count();

    // Create a decoder for the track
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .context("unsupported codec")?;

    let mut all_samples: Vec<f32> = Vec::new();
    let mut _total_frames = 0;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    // Normal end of file
                    break;
                } else {
                    return Err(anyhow!("IO error while reading audio: {}", e));
                }
            }
            Err(SymphoniaError::ResetRequired) => {
                continue;
            }
            Err(err) => return Err(anyhow!("Failed to read audio packet: {}", err)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let frame = decoded.frames();
                let mut sample_buf =
                    SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
                sample_buf.copy_interleaved_ref(decoded);

                // Append to main vector
                all_samples.extend_from_slice(sample_buf.samples());
                _total_frames += frame;
            }
            Err(SymphoniaError::DecodeError(_)) => continue, // Skip bad frames
            Err(err) => return Err(anyhow!("Decode packet error: {}", err)),
        }
    }

    // We divide by channel count because all_samples is interleaved (L, R, L, R)
    // If we have 100 floats and 2 channels, we have 50 "Audio Frames" (Samples).
    let total_frames = if channel_count > 0 {
        all_samples.len() as u64 / channel_count as u64
    } else {
        0
    };

    let duration_seconds = if sample_rate > 0 {
        total_frames as f64 / sample_rate as f64
    } else {
        0.0
    };

    Ok(AudioWaveform {
        buffer: Arc::new(all_samples),
        file_path: path_string,
        sample_rate,
        channel_count: channel_count as u8,
        duration: duration_seconds,
        trim_end: total_frames,
        ..Default::default()
    })
}
