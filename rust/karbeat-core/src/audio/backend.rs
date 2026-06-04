use anyhow::{anyhow, Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    OutputCallbackInfo,
};
use rtrb::{Consumer, RingBuffer};
use serde::{Deserialize, Serialize};

use crate::{
    audio::{engine::AudioEngine, event::TransportFeedback},
    commands::AudioCommand,
    context::ctx,
};

#[allow(unused)]
fn host_has_output_device(host: &cpal::Host) -> bool {
    host.output_devices()
        .map(|mut devices| devices.next().is_some())
        .unwrap_or(false)
}

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
    ) => {{
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
    }};
}

/// Set host to use the optimized host for each platform.
/// - Windows: ASIO, then fallback to WASAPI (low latency)
/// - Android: AAudio (low latency, requires API 26+)
/// - Linux: JACK
/// - Other platforms: default host
fn set_host() -> cpal::Host {
    #[allow(unused_assignments)]
    #[allow(unused_mut)]
    let mut host = cpal::default_host();

    #[cfg(target_os = "windows")]
    {
        if let Ok(asio_host) = cpal::host_from_id(cpal::HostId::Asio) {
            if host_has_output_device(&asio_host) {
                log::info!("Connected to ASIO Host");
                return asio_host;
            } else {
                log::warn!(
                    "ASIO host found but no output devices were available; falling back to WASAPI"
                );
            }
        } else {
            log::warn!("ASIO host not available; falling back to WASAPI");
        }

        if let Ok(wasapi_host) = cpal::host_from_id(cpal::HostId::Wasapi) {
            log::info!("Connected to WASAPI Host");
            return wasapi_host;
        }

        log::warn!("WASAPI not available; falling back to default host");
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceConfig {
    /// The string name of the CPAL host (e.g., "ASIO", "WASAPI", "CoreAudio")
    pub host_name: Option<String>,
    /// The specific device name chosen by the user
    pub device_name: Option<String>,
    pub device_id: Option<String>,
    /// The user's desired sample rate (e.g., 44100, 48000, 96000)
    pub sample_rate: Option<u32>,
    /// The desired block/buffer size (e.g., 256, 512, 1024)
    pub buffer_size: Option<u32>,
}

/// A clean struct to pass back to Flutter for the UI dropdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
}

impl Default for AudioDeviceConfig {
    fn default() -> Self {
        Self {
            host_name: None,
            device_name: None,
            device_id: None,
            sample_rate: None,
            buffer_size: None, // Falls back to 1024
        }
    }
}

/// Resolve the host based on user preference or OS defaults
fn resolve_host(preferred_host: Option<&str>) -> cpal::Host {
    if let Some(host_name) = preferred_host {
        if let Some(host_id) = cpal::available_hosts()
            .into_iter()
            .find(|h| h.name() == host_name)
        {
            if let Ok(host) = cpal::host_from_id(host_id) {
                log::info!("Connected to user-selected Host: {}", host_name);
                return host;
            }
        }
        log::warn!(
            "Requested host '{}' not found or failed to load. Falling back...",
            host_name
        );
    }

    set_host()
}

/// Returns a list of available host names on the current OS
pub fn get_available_hosts() -> Vec<String> {
    cpal::available_hosts()
        .into_iter()
        .map(|h| h.name().to_string())
        .collect()
}

/// Returns the available output devices for a specific host
pub fn get_output_devices(host_name: String) -> Result<Vec<AudioDeviceInfo>, anyhow::Error> {
    let host_id = cpal::available_hosts()
        .into_iter()
        .find(|h| h.name() == host_name)
        .unwrap_or_else(|| cpal::default_host().id());

    let host = cpal::host_from_id(host_id)?;
    let mut devices = vec![];

    for dev in host.output_devices()? {
        if let (Ok(id), Ok(desc)) = (dev.id(), dev.description()) {
            devices.push(AudioDeviceInfo {
                id: id.to_string(),
                name: desc.to_string(),
            });
        }
    }
    Ok(devices)
}

/// Returns the supported sample rates for a specific device on a specific host
pub fn get_device_sample_rates(
    host_name: String,
    device_id: String,
) -> Result<Vec<u32>, anyhow::Error> {
    let host_id = cpal::available_hosts()
        .into_iter()
        .find(|h| h.name() == host_name)
        .unwrap_or_else(|| cpal::default_host().id());

    let host = cpal::host_from_id(host_id)?;
    let device = host
        .output_devices()?
        .find(|d| {
            let Ok(id) = d.id() else {
                return false;
            };

            id.to_string() == device_id
        })
        .ok_or_else(|| anyhow::anyhow!("Device not found"))?;

    let mut rates = vec![];
    for config in device.supported_output_configs()? {
        rates.push(config.min_sample_rate());
        rates.push(config.max_sample_rate());
    }

    rates.sort();
    rates.dedup();
    Ok(rates)
}

/// Start the audio stream by initializing the Command Queue and Audio Engine
/// and then building the audio stream.
pub fn start_audio_stream(
    command_consumer: Consumer<AudioCommand>,
    config_pref: AudioDeviceConfig,
) -> Result<()> {
    {
        let mut guard = ctx().stream_guard.lock();
        if guard.is_some() {
            log::info!("Stopping previous audio stream...");
            *guard = None; // This drops the stream, stopping the audio thread
        }
    }
    let host = resolve_host(config_pref.host_name.as_deref());

    let device = if let Some(dev_id) = &config_pref.device_id {
        host.output_devices()?
            .find(|d| match d.id() {
                Ok(id) => id.to_string() == *dev_id,
                Err(_) => false,
            })
            .ok_or_else(|| anyhow!("Requested device ID '{}' not found", dev_id))?
    } else {
        host.default_output_device()
            .context("no audio output device available")?
    };
    // debug!("Output dev");
    let device_name = match device.description() {
        Ok(desc) => desc.name().to_owned(),
        Err(_) => "Unknown".into(),
    };
    log::info!("Output device: {}", device_name);

    // Ask the OS for its preferred/native audio configuration
    let default_config = device
        .default_output_config()
        .context("no default output config available")?;
    let native_sample_rate = default_config.sample_rate();

    let target_sample_rate = config_pref.sample_rate.unwrap_or(native_sample_rate);

    let supported_configs_range = device
        .supported_output_configs()
        .map_err(|e| anyhow!("error querying configs: {e}"))?;

    // Find a config that supports F32, 2 Channels, AND the OS's native sample rate
    let supported_config = supported_configs_range
        .filter(|c| c.sample_format() == cpal::SampleFormat::F32 && c.channels() == 2)
        .find(|c| {
            c.min_sample_rate() <= target_sample_rate && c.max_sample_rate() >= target_sample_rate
        })
        .map(|c| c.with_sample_rate(target_sample_rate))
        .or_else(|| {
            // Fallback: If requested rate fails, force fallback to default native
            log::warn!("Requested sample rate unsupported by device, falling back");
            device
                .supported_output_configs()
                .ok()?
                .find(|c| c.sample_format() == cpal::SampleFormat::F32 && c.channels() == 2)
                .map(|c| {
                    let clamped =
                        native_sample_rate.clamp(c.min_sample_rate(), c.max_sample_rate());
                    c.with_sample_rate(clamped)
                })
        })
        .context("device does not support f32 samples with 2 channels")?;

    let buffer_size = match supported_config.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => {
            let desired = config_pref.buffer_size.unwrap_or(1024);
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

    {
        let mut state = ctx().app_state.write();
        state.audio_config.sample_rate = sample_rate;
        state.audio_config.selected_output_device = match device.description() {
            Ok(desc) => desc.to_string(),
            Err(_) => "Unknown".into(),
        };
    }

    let (pos_producer, pos_consumer) = RingBuffer::<TransportFeedback>::new(100);

    // Store Consumer in context
    *ctx().position_consumer.lock() = Some(pos_consumer);

    // Create feedback ring buffer (Audio → UI for parameter updates)
    let (feedback_producer, feedback_consumer) =
        RingBuffer::<crate::commands::AudioFeedback>::new(512);
    *ctx().feedback_consumer.lock() = Some(feedback_consumer);

    // Read initial BPM from app state for the audio engine
    let initial_bpm = {
        let app = ctx().app_state.read();
        app.transport.bpm
    };

    // Resolved before engine creation so the engine can seed its internal
    // graph snapshot with the real buffer_size (avoids graph.buffer_size = 0).
    let engine_block_size = config_pref.buffer_size.unwrap_or(1024) as usize;

    let engine = AudioEngine::new(
        command_consumer,
        pos_producer,
        feedback_producer,
        sample_rate,
        channels as u16,
        initial_bpm,
        engine_block_size,
    );

    let ring_buffer_capacity = 8192;
    let (producer, consumer) = RingBuffer::<f32>::new(ring_buffer_capacity);

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

        cpal::SampleFormat::I16 => run_stream!(
            device,
            config,
            audio_ctx,
            consumer,
            i16,
            |s: f32| (s * (i16::MAX as f32)).clamp(i16::MIN as f32, i16::MAX as f32) as i16,
            err_fn
        ),

        cpal::SampleFormat::U16 => run_stream!(
            device,
            config,
            audio_ctx,
            consumer,
            u16,
            |s: f32| ((s + 1.0) * 0.5 * (u16::MAX as f32)).clamp(0.0, u16::MAX as f32) as u16,
            err_fn
        ),

        cpal::SampleFormat::U8 => run_stream!(
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
    let mut guard = ctx().stream_guard.lock();
    *guard = Some(stream);

    log::info!("Successfully initialize Audio backend");
    Ok(())
}
