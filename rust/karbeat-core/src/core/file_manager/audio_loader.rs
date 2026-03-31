// src/core/file_manager/loader.rs

// Source code of file loader

use hashbrown::HashMap;
use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::Path,
    sync::Arc,
};
use tempfile::tempfile;

use anyhow::{anyhow, Context, Result};
use memmap2::MmapOptions;
use rodio::Source;

use crate::core::project::{
    track::audio_waveform::AudioWaveform,
    ApplicationState, AudioSourceId,
};

trait FileNameExt {
    fn file_name_string(&self) -> String;
}

impl FileNameExt for Path {
    fn file_name_string(&self) -> String {
        self.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string()
    }
}

mod err {
    use thiserror::Error;

    #[derive(Error, Debug)]
    #[error("Failed to load the audio: {message}")]
    #[allow(dead_code)]
    // TODO: Use this in Audio File I/O
    pub struct LoadAudioError<'a> {
        pub message: &'a str,
    }
}

/// Main entry point for loading audio.
pub fn load_audio_file(path_str: &str, name: Option<&str>) -> Result<AudioWaveform> {
    let path = Path::new(path_str);
    let file =
        File::open(path).with_context(|| format!("Failed to open audio file: {}", path_str))?;
    let reader = BufReader::new(file);

    let decoder =
        rodio::Decoder::new(reader).context("Failed to decode audio file (unsupported format)")?;
    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels();

    // Cache the loaded audio file
    let mut cache_file = tempfile().context("Failed to create temporary cache file")?;

    // let all_samples: Vec<f32> = decoder.collect();
    // let total_samples = all_samples.len() as u32;

    // Use a BufWriter. It handles disk I/O incredibly efficiently behind the scenes.
    let total_samples = {
        let mut writer = BufWriter::new(&mut cache_file);

        let mut total_samples: u32 = 0;

        // Create a tiny buffer to hold 8192 samples (~32 KB of RAM)
        let chunk_size = 8192;
        let mut chunk: Vec<f32> = Vec::with_capacity(chunk_size);

        // Stream directly from the decoder to the disk
        for sample in decoder {
            chunk.push(sample);
            total_samples += 1;

            // When our tiny RAM buffer is full, dump it to the disk and clear it
            if chunk.len() == chunk_size {
                let byte_slice: &[u8] = bytemuck::cast_slice(&chunk);
                writer.write_all(byte_slice)?;
                chunk.clear(); // Clears the vector but keeps the allocated 32KB capacity
            }
        }

        // Write any leftover samples that didn't perfectly fill the last chunk
        if !chunk.is_empty() {
            let byte_slice: &[u8] = bytemuck::cast_slice(&chunk);
            writer.write_all(byte_slice)?;
        }

        // Flush the writer to guarantee all bytes are written to the disk
        writer.flush()?;
        total_samples
    };

    let total_frames = if channels.get() > 0 {
        total_samples / (channels.get() as u32)
    } else {
        0
    };

    // Write the raw f32 bytes directly to the disk cache
    // bytemuck safely casts &[f32] into &[u8] for writing
    // let byte_slice: &[u8] = bytemuck::cast_slice(&all_samples);
    // cache_file.write_all(byte_slice)?;

    let mmap = unsafe { MmapOptions::new().map(&cache_file)? };

    let duration_seconds = if sample_rate.get() > 0 {
        (total_frames as f64) / (sample_rate.get() as f64)
    } else {
        0.0
    };

    let final_name = if let Some(name_str) = name {
        name_str.to_string()
    } else {
        path.file_name_string()
    };

    let buffer = Some(Arc::new(mmap));

    Ok(AudioWaveform {
        buffer,
        file_path: path.to_path_buf(),
        name: final_name,
        sample_rate: sample_rate.get(),
        channels: channels.get(),
        duration: duration_seconds,
        trim_end: total_frames,
        ..Default::default()
    })
}

// Trait AudioLoader
pub trait AudioLoader {
    fn load_audio(&mut self, path: &str, name: Option<&str>) -> Result<u32>;
    fn get_audio_source(&self, id: u32) -> Option<Arc<AudioWaveform>>;
    fn get_audio_sources(&self) -> HashMap<AudioSourceId, Arc<AudioWaveform>>;
}

impl AudioLoader for ApplicationState {
    fn load_audio(&mut self, path: &str, name: Option<&str>) -> Result<u32> {
        // Load the actual audio data (Heavy I/O operation)
        // This parses the file into f32 samples
        let waveform = match load_audio_file(path, name) {
            Ok(waveform) => waveform,
            Err(e) => {
                let error_msg = format!("Cannot decode audio file: {}", e);
                log::error!("{}", error_msg);
                return Err(anyhow!("{}", error_msg));
            }
        };
        let id = self.asset_library.next_id;
        let asset_library = Arc::make_mut(&mut self.asset_library);
        asset_library.next_id += 1;

        // set mipmap

        asset_library
            .source_map
            .insert(id.into(), Arc::new(waveform));

        log::info!("Successfully loaded audio: {} (ID: {})", path, id);

        Ok(id)
    }

    fn get_audio_source(&self, id: u32) -> Option<Arc<AudioWaveform>> {
        self.asset_library
            .source_map
            .get(&AudioSourceId::from(id))
            .cloned()
    }

    fn get_audio_sources(&self) -> HashMap<AudioSourceId, Arc<AudioWaveform>> {
        self.asset_library.source_map.clone()
    }
}
