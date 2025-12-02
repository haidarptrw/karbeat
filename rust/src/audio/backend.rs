use std::sync::Mutex;

use anyhow::{anyhow, Context, Result};
use cpal::{OutputCallbackInfo, traits::{DeviceTrait, HostTrait, StreamTrait}};
use log::debug;
use once_cell::sync::Lazy;
use rtrb::Consumer;
use triple_buffer::Output;

use crate::{audio::{engine::AudioEngine, render_state::AudioRenderState}, commands::AudioCommand};

static STREAM_GUARD: Lazy<Mutex<Option<cpal::Stream>>> = Lazy::new(|| Mutex::new(None));

pub fn start_audio_stream(state_consumer: Output<AudioRenderState>, command_consumer: Consumer<AudioCommand>) -> Result<()> {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .context("no audio output device available")?;

    // debug!("Output dev");
    println!(
        "Output device: {}",
        device.name().unwrap_or("Unknown".into())
    );

    let supported_configs_range = device
        .supported_output_configs()
        .map_err(|e| anyhow!("error querying configs: {e}"))?;

    let supported_config = supported_configs_range
        .filter(|c| c.sample_format() == cpal::SampleFormat::F32)
        .next()
        .map(|c| c.with_max_sample_rate())
        .context("device does not support f32 samples")?;

    let sample_format = supported_config.sample_format();
    let config: cpal::StreamConfig = supported_config.into();
    let sample_rate = config.sample_rate.0;
    let channels = config.channels;

    println!("Stream Config: {:?} Hz, {} Channels", sample_rate, channels);

    let mut engine = AudioEngine::new(state_consumer, command_consumer, sample_rate);

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config, 
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                engine.process(data);
            }, 
            err_fn, 
            None),
        _ => return Err(anyhow!("Unsupported sample format (only f32 supported)".to_string()))
    }.map_err(|e| anyhow!("Failed to build stream: {}", e))?;

    // Play and store
    stream.play().map_err(|e| anyhow!("Failed to play stream: {}", e))?;

    // store the stream globally so it does not get dropped
    let mut guard  = STREAM_GUARD.lock().unwrap();
    *guard = Some(stream);
    
    Ok(())
}
