use anyhow::{ anyhow, Context, Result };
use cpal::{ traits::{ DeviceTrait, HostTrait, StreamTrait }, OutputCallbackInfo };
use rtrb::{ Consumer, RingBuffer };
use triple_buffer::Output;

use crate::{
    audio::{ engine::AudioEngine, event::TransportFeedback, render_state::AudioRenderState },
    commands::AudioCommand,
    ctx,
};

struct AudioContext {
    engine: AudioEngine,
    producer: rtrb::Producer<f32>,
    staging_buffer: Vec<f32>,
}

/// Macro to generate the stream building logic
/// $device: cpal device
/// $config: cpal config
/// $ctx: The AudioContext (moved into the closure)
/// $consumer: The RingBuffer consumer (moved into the closure)
/// $sample_type: The primitive type (f32, i16, etc)
/// $converter: A closure |f32_sample| -> $sample_type
macro_rules! run_stream {
    (
        $device:expr,
        $config:expr,
        $audio_ctx:expr,
        $consumer:expr,
        $sample_type:ty,
        $converter:expr,
        $err_fn:expr
    ) => {
        {
        let mut audio_ctx = $audio_ctx;
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
                    audio_ctx.engine.process(&mut audio_ctx.staging_buffer);

                    // Push to RingBuffer
                    // Note: process() fills staging_buffer completely
                    for sample in &audio_ctx.staging_buffer {
                        if let Err(_) = audio_ctx.producer.push(*sample) {
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
        }
    };
}

/// Set host to use the optimized host for each platform.
/// - Windows: ASIO, then fallback to WASAPI (low latency)
/// - Android: AAudio (low latency, requires API 26+)
/// - Linux: JACK
/// - Other platforms: default host
fn set_host() -> cpal::Host {
    #[allow(unused_assignments)]
    let mut host = cpal::default_host();

    #[cfg(target_os = "windows")]
    {   
        if let Ok(asio_host) = cpal::host_from_id(cpal::HostId::Asio) {
            host = asio_host;
            log::info!("Connected to ASIO Host");
        } else if let Ok(wasapi_host) = cpal::host_from_id(cpal::HostId::Wasapi) {
            host = wasapi_host;
            log::info!("Connected to WASAPI Host");
        } else {
            log::warn!("Neither ASIO nor WASAPI available, falling back to default host");
        }
    }

    #[cfg(target_os = "android")]
    {
        match cpal::host_from_id(cpal::HostId::AAudio) {
            Ok(aaudio_host) => {
                host = aaudio_host;
                log::info!("Connected to AAudio Host");
            }
            Err(e) => {
                log::warn!("AAudio not available, falling back to default host: {}", e);
                host = cpal::default_host();
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        match cpal::host_from_id(cpal::HostId::Jack) {
            Ok(jack_host) => {
                host = jack_host;
                log::info!("Connected to JACK Host");
            }
            Err(e) => {
                log::warn!("JACK not available, falling back to default host: {}", e);
                host = cpal::default_host();
            }
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        if let Ok(host) = cpal::host_from_id(cpal::HostId::CoreAudio) {
            log::info!("Connected to CoreAudio Host");
            return host;
        }
    }

    host
}

/// Start the audio strem by initializing the Command Queue and Audio Engine
/// and then building the audio stream
pub fn start_audio_stream(
    mut state_consumer: Output<AudioRenderState>,
    command_consumer: Consumer<AudioCommand>,
    initial_state: AudioRenderState
) -> Result<()> {
    {
        let mut guard = ctx().stream_guard.lock().unwrap();
        if guard.is_some() {
            log::info!("Stopping previous audio stream...");
            *guard = None; // This drops the stream, stopping the audio thread
        }
    }
    let host = set_host();

    let device = host.default_output_device().context("no audio output device available")?;

    // debug!("Output dev");
    let device_name = match device.description() {
        Ok(desc) => desc.to_string(),
        Err(_) => "Unknown".into(),
    };
    log::info!("Output device: {}", device_name);

    let supported_configs_range = device
        .supported_output_configs()
        .map_err(|e| anyhow!("error querying configs: {e}"))?;

    let supported_config = supported_configs_range
        .filter(|c| c.sample_format() == cpal::SampleFormat::F32 && c.channels() == 2)
        .next()
        .map(|c| c.with_max_sample_rate())
        .context("device does not support f32 samples")?;

    // This prevents PipeWire/JACK from resizing the buffer dynamically.
    let buffer_size = match supported_config.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => {
            // Request 512 frames (good balance of latency vs stability)
            // Clamp it to ensure we don't request something invalid
            let desired = 512;
            cpal::BufferSize::Fixed(desired.clamp(*min, *max))
        }
        cpal::SupportedBufferSize::Unknown => cpal::BufferSize::Default,
    };

    // Construct the concrete config manually
    let config = cpal::StreamConfig {
        channels: supported_config.channels(),
        sample_rate: supported_config.sample_rate(),
        buffer_size,
    };

    let sample_format = supported_config.sample_format();
    let sample_rate: u32 = config.sample_rate;
    let channels = config.channels as usize;

    log::info!("Stream Config: {:?} Hz, {} Channels", sample_rate, channels);
    log::info!("Sample format: {}", sample_format);

    if let Ok(mut state) = ctx().app_state.write() {
        state.audio_config.sample_rate = sample_rate;
        state.audio_config.selected_output_device = match device.description()
        {
            Ok(desc) => desc.to_string(),
            Err(_) => "Unknown".into(),
        }
    }

    state_consumer.update();

    let (pos_producer, pos_consumer) = RingBuffer::<TransportFeedback>::new(100);

    // 2. Store Consumer in context
    *ctx().position_consumer.lock().unwrap() = Some(pos_consumer);

    // Create feedback ring buffer (Audio → UI for parameter updates)
    let (feedback_producer, feedback_consumer) = RingBuffer::<crate::commands::AudioFeedback>::new(
        256
    );
    *ctx().feedback_consumer.lock().unwrap() = Some(feedback_consumer);

    // Read initial BPM from app state for the audio engine
    let initial_bpm = {
        let app = ctx().app_state.read().unwrap();
        app.transport.bpm
    };

    let engine = AudioEngine::new(
        state_consumer,
        command_consumer,
        pos_producer,
        feedback_producer,
        sample_rate,
        initial_bpm,
        initial_state
    );

    
    let ring_buffer_capacity = 4096;
    let (producer, consumer) = RingBuffer::<f32>::new(ring_buffer_capacity);
    
    let engine_block_size = 512;
    let staging_buffer = vec![0.0; engine_block_size * channels];

    let audio_ctx = AudioContext {
        engine,
        producer,
        staging_buffer,
    };

    let err_fn = |err| log::error!("Audio stream error: {}", err);

    let stream = (match sample_format {
        cpal::SampleFormat::F32 => {
            run_stream!(device, config, audio_ctx, consumer, f32, |s| s, err_fn)
        }

        cpal::SampleFormat::I16 =>
            run_stream!(
                device,
                config,
                audio_ctx,
                consumer,
                i16,
                |s: f32| (s * (i16::MAX as f32)).clamp(i16::MIN as f32, i16::MAX as f32) as i16,
                err_fn
            ),

        cpal::SampleFormat::U16 =>
            run_stream!(
                device,
                config,
                audio_ctx,
                consumer,
                u16,
                |s: f32| ((s + 1.0) * 0.5 * (u16::MAX as f32)).clamp(0.0, u16::MAX as f32) as u16,
                err_fn
            ),

        cpal::SampleFormat::U8 =>
            run_stream!(
                device,
                config,
                audio_ctx,
                consumer,
                u8,
                |s: f32| ((s + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0) as u8,
                err_fn
            ),

        other => {
            return Err(anyhow!("Unsupported sample format: {:?}", other));
        }
    })?;

    // Play and store
    stream.play().context("Failed to play stream")?;

    // store the stream in context so it does not get dropped
    let mut guard = ctx().stream_guard.lock().unwrap();
    *guard = Some(stream);

    Ok(())
}
