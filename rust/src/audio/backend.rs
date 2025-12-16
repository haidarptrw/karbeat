use std::sync::Mutex;

use anyhow::{anyhow, Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    OutputCallbackInfo,
};
use log::debug;
use once_cell::sync::Lazy;
use rtrb::{Consumer, RingBuffer};
use triple_buffer::Output;

use crate::{
    audio::{engine::AudioEngine, event::PlaybackPosition, render_state::AudioRenderState},
    commands::AudioCommand,
};

static STREAM_GUARD: Lazy<Mutex<Option<cpal::Stream>>> = Lazy::new(|| Mutex::new(None));

pub static POSITION_CONSUMER: Mutex<Option<rtrb::Consumer<PlaybackPosition>>> = Mutex::new(None);

struct AudioContext {
    engine: AudioEngine,
    producer: rtrb::Producer<f32>,
    staging_buffer: Vec<f32>,
    channels: usize,
}

/// Macro to generate the stream building logic
/// $device: cpal device
/// $config: cpal config
/// $ctx: The AudioContext (moved into the closure)
/// $consumer: The RingBuffer consumer (moved into the closure)
/// $sample_type: The primitive type (f32, i16, etc)
/// $converter: A closure |f32_sample| -> $sample_type
macro_rules! run_stream {
    ($device:expr, $config:expr, $ctx:expr, $consumer:expr, $sample_type:ty, $converter:expr, $err_fn:expr) => {{
        let mut ctx = $ctx;
        let mut consumer = $consumer;
        // Internal buffer for reading from ringbuffer before conversion
        let mut read_buffer: Vec<f32> = Vec::new();

        $device.build_output_stream(
            &$config,
            move |data: &mut [$sample_type], _: &OutputCallbackInfo| {
                let samples_needed = data.len();

                // Ensure Ring Buffer has enough data
                // While readable samples < needed samples
                while consumer.slots() < samples_needed {
                    // Process fixed block
                    ctx.engine.process(&mut ctx.staging_buffer);

                    // Push to RingBuffer
                    // Note: process() fills staging_buffer completely
                    for sample in &ctx.staging_buffer {
                        if let Err(_) = ctx.producer.push(*sample) {
                            break;
                        }
                    }
                }

                if read_buffer.len() != samples_needed {
                    read_buffer.resize(samples_needed, 0.0);
                }

                for i in 0..samples_needed {
                    if let Ok(sample) = consumer.pop() {
                        read_buffer[i] = sample;
                    }
                }

                // Write to output with conversion
                for (out, &in_sample) in data.iter_mut().zip(read_buffer.iter()) {
                    *out = $converter(in_sample);
                }
            },
            $err_fn,
            None,
        )
    }};
}

/// Set host to use the optimized host. For now, it handles driver on Windows to use ASIO that is more optimized
///
/// **TODO: Handle drive on other OS**
fn set_host() -> cpal::Host {
    #[allow(unused_assignments)]
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
        log::info!("Connected to WASAPI Host");
    }
    host
}

pub fn start_audio_stream(
    mut state_consumer: Output<AudioRenderState>,
    command_consumer: Consumer<AudioCommand>,
    initial_state: AudioRenderState,
) -> Result<()> {

    {
        let mut guard = STREAM_GUARD.lock().unwrap();
        if guard.is_some() {
            log::info!("🛑 Stopping previous audio stream...");
            *guard = None; // This drops the stream, stopping the audio thread
        }
    }
    let host = set_host();

    let device = host
        .default_output_device()
        .context("no audio output device available")?;

    // debug!("Output dev");
    log::info!(
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
    let channels = config.channels as usize;

    log::info!("Stream Config: {:?} Hz, {} Channels", sample_rate, channels);
    log::info!("Sample format: {}", sample_format);

    {
        // We use crate::APP_STATE because it is public in lib.rs
        if let Ok(mut state) = crate::APP_STATE.write() {
            state.audio_config.sample_rate = sample_rate as u32;
            state.audio_config.selected_output_device = device.name().unwrap_or("Unknown".to_string());
            log::info!("✅ Global Audio Config updated: {} Hz", sample_rate);
        } else {
            log::error!("❌ Failed to lock APP_STATE to update audio config");
            panic!();
        }
    }

    state_consumer.update();

    // Read buffer size before moving state_consumer
    let mut buffer_size = state_consumer.read().buffer_size;

    if buffer_size == 0 {
        log::warn!("Warning: buffer_size is 0 — using fallback of 512 frames");
        buffer_size = 512; // or return Err
    }

    let (pos_producer, pos_consumer) = RingBuffer::<PlaybackPosition>::new(100);

    // 2. Store Consumer globally (or pass to your API stream handler)
    *POSITION_CONSUMER.lock().unwrap() = Some(pos_consumer);

    let engine = AudioEngine::new(state_consumer, command_consumer, pos_producer, sample_rate, initial_state);

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let ring_buffer_capacity = sample_rate as usize * channels as usize * 2;
    let (producer, consumer) = RingBuffer::<f32>::new(ring_buffer_capacity);

    let staging_buffer = vec![0.0; buffer_size * channels];

    let ctx = AudioContext {
        engine,
        producer,
        staging_buffer,
        channels,
    };

    let stream = match sample_format {
        cpal::SampleFormat::F32 => run_stream!(
            device,
            config,
            ctx,
            consumer,
            f32,
            |s| s, // Identity
            err_fn
        ),

        cpal::SampleFormat::I16 => run_stream!(
            device,
            config,
            ctx,
            consumer,
            i16,
            |s: f32| (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16,
            err_fn
        ),

        cpal::SampleFormat::U16 => run_stream!(
            device,
            config,
            ctx,
            consumer,
            u16,
            |s: f32| ((s + 1.0) * 0.5 * u16::MAX as f32).clamp(0.0, u16::MAX as f32) as u16,
            err_fn
        ),

        cpal::SampleFormat::U8 => run_stream!(
            device,
            config,
            ctx,
            consumer,
            u8,
            |s: f32| ((s + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0) as u8,
            err_fn
        ),

        other => {
            return Err(anyhow!("Unsupported sample format: {:?}", other));
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
