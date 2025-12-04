use std::sync::Mutex;

use anyhow::{anyhow, Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    OutputCallbackInfo,
};
use log::debug;
use once_cell::sync::Lazy;
use rtrb::Consumer;
use triple_buffer::Output;

use crate::{
    audio::{engine::AudioEngine, render_state::AudioRenderState},
    commands::AudioCommand,
};

static STREAM_GUARD: Lazy<Mutex<Option<cpal::Stream>>> = Lazy::new(|| Mutex::new(None));

/// Set host to use the optimized host. For now, it handles driver on Windows to use ASIO that is more optimized
///
/// **TODO: Handle drive on other OS**
fn set_host() -> cpal::Host {
    let mut host = cpal::default_host();
    #[cfg(target_os = "windows")]
    {
        // host = match cpal::host_from_id(cpal::HostId::Asio) {
        //     Ok(asio) => asio,
        //     Err(_) => cpal::host_from_id(cpal::HostId::Wasapi),
        // }
        let Ok(wasapi_host) = cpal::host_from_id(cpal::HostId::Wasapi) else {
            host = cpal::default_host();
            return host;
        };
        host = wasapi_host;
        println!("Connected to WASAPI Host");
    }
    host
}

pub fn start_audio_stream(
    state_consumer: Output<AudioRenderState>,
    command_consumer: Consumer<AudioCommand>,
) -> Result<()> {
    let host = set_host();

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
    let sample_rate: u64 = config.sample_rate.0.into();
    let channels = config.channels;

    println!("Stream Config: {:?} Hz, {} Channels", sample_rate, channels);
    println!("Sample format: {}", sample_format);

    let mut engine = AudioEngine::new(state_consumer, command_consumer, sample_rate);

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config,
            move |data: &mut [f32], _| {
                engine.process(data);
            },
            err_fn,
            None,
        ),

        cpal::SampleFormat::I16 => device.build_output_stream(
            &config,
            move |data: &mut [i16], _| {
                let mut buffer_f32 = vec![0.0; data.len()];
                engine.process(&mut buffer_f32);

                for (out, &v) in data.iter_mut().zip(buffer_f32.iter()) {
                    let v = (v * i16::MAX as f32).round();
                    *out = v.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                }
            },
            err_fn,
            None,
        ),

        cpal::SampleFormat::U16 => device.build_output_stream(
            &config,
            move |data: &mut [u16], _| {
                let mut buffer_f32 = vec![0.0; data.len()];
                engine.process(&mut buffer_f32);

                for (out, &v) in data.iter_mut().zip(buffer_f32.iter()) {
                    let v = ((v + 1.0) * 0.5 * u16::MAX as f32).round();
                    *out = v.clamp(0.0, u16::MAX as f32) as u16;
                }
            },
            err_fn,
            None,
        ),

        cpal::SampleFormat::U8 => {
            device.build_output_stream(
                &config,
                move |data: &mut [u8], _| {
                    let mut buffer_f32 = vec![0.0; data.len()];
                    engine.process(&mut buffer_f32);

                    for (out, &v) in data.iter_mut().zip(buffer_f32.iter()) {
                        let v = ((v + 1.0) * 0.5 * 255.0).round();
                        *out = v.clamp(0.0, 255.0) as u8;
                    }
                },
                err_fn,
                None,
            )
        }

        other => {
            return Err(anyhow!(
                "Unsupported sample format from device: {:?}",
                other
            ));
        }
    }?;

    // Play and store
    stream
        .play()
        .map_err(|e| anyhow!("Failed to play stream: {}", e))?;

    // store the stream globally so it does not get dropped
    let mut guard = STREAM_GUARD.lock().unwrap();
    *guard = Some(stream);

    Ok(())
}
