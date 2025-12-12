// src/core/file_manager/loader.rs

// Source code of file loader

use std::{fs::File, io::BufReader, path::{Path, PathBuf}, sync::Arc};

use anyhow::{anyhow, Context, Result};
use rodio::Source;

use crate::core::{project::ApplicationState, track::audio_waveform::AudioWaveform};

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

/// Main entry point for loading audio. 
pub fn load_audio_file(path_string: String, name: Option<String>) -> Result<AudioWaveform> {
    let path = Path::new(&path_string);
    let file = File::open(path)
        .with_context(|| format!("Failed to open audio file: {}", path_string))?;
    let reader = BufReader::new(file);

    let decoder = rodio::Decoder::new(reader)
        .context("Failed to decode audio file (unsupported format)")?;
    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels();
    let all_samples: Vec<f32> = decoder.collect();
    let total_samples = all_samples.len() as u64;
    let total_frames = if channels > 0 { total_samples / channels as u64 } else { 0 };
    
    let duration_seconds = if sample_rate > 0 {
        total_frames as f64 / sample_rate as f64
    } else {
        0.0
    };

    let final_name = name.unwrap_or_else(|| path.file_name_string());

    Ok(AudioWaveform {
        buffer: Arc::new(all_samples),
        file_path: path_string,
        name: final_name,
        sample_rate,
        channels,
        duration: duration_seconds,
        trim_end: total_frames,
        ..Default::default()
    })
}

// Trait AudioLoader
pub trait AudioLoader {
    fn load_audio(&mut self, path: String, name: Option<String>) -> Result<u32>;
    fn get_audio_source(&self, id: u32) -> Option<Arc<AudioWaveform>>;
}

impl AudioLoader for ApplicationState {
    fn load_audio(&mut self, path: String, name: Option<String>) -> Result<u32> {
        // Load the actual audio data (Heavy I/O operation)
        // This parses the file into f32 samples
        let waveform = match load_audio_file(path.clone(), name) {
            Ok(waveform) => waveform,
            Err(e) => {
                println!("Cannot decode audio file: {}", e);
                return Err(anyhow!(format!("Cannot decode audio file: {}", e)));
            }
        };
        let id = self.asset_library.next_id;
        let mut asset_library = Arc::make_mut(&mut self.asset_library);
        asset_library.next_id += 1;

        asset_library.sample_paths.insert(
            id, 
            PathBuf::from(&path)
        );
        asset_library.source_map.insert(
            id, 
            Arc::new(waveform)
        );

        println!("Successfully loaded audio: {} (ID: {})", path, id);

        Ok(id)
    }
    
    fn get_audio_source(&self, id: u32) -> Option<Arc<AudioWaveform>> {
        self.asset_library.source_map.get(&id).cloned()
    }
}