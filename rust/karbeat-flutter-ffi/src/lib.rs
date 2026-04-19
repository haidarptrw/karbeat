// src/lib.rs

use std::{path::PathBuf, sync::Arc};

use memmap2::MmapOptions;
use rtrb::RingBuffer;

pub(crate) use karbeat_core::{
    audio::{backend::start_audio_stream, render_state::AudioRenderState},
    commands::AudioCommand,
    context::{ctx, INIT_LOGGER},
    core::project::track::audio_waveform::AudioWaveform,
};

pub mod api;
mod frb_generated;

pub use karbeat_core::context::{ctx as get_ctx, INIT_LOGGER as get_init};

// ==================================================================
// ================== Functions =====================================
// ==================================================================


fn generate_startup_beep() -> AudioWaveform {
    let sample_rate = 48000;
    let duration_secs = 0.5;
    let total_frames = ((sample_rate as f32) * duration_secs) as usize;
    let frequency = 440.0; // A4 Note

    let mut buffer = Vec::with_capacity(total_frames * 2); // Stereo

    for i in 0..total_frames {
        let t = (i as f32) / (sample_rate as f32);
        let signal = (t * frequency * 2.0 * std::f32::consts::PI).sin();
        let envelope = 1.0 - (i as f32) / (total_frames as f32);

        let final_sample = signal * envelope * 0.3;

        buffer.push(final_sample); // Left
        buffer.push(final_sample); // Right
    }

    let byte_slice: &[u8] = bytemuck::cast_slice(&buffer);
    let mut mmap_mut = MmapOptions::new()
        .len(byte_slice.len())
        .map_anon()
        .expect("Failed to create anonymous mmap for beep");
    mmap_mut.copy_from_slice(byte_slice);
    let mmap = mmap_mut
        .make_read_only()
        .expect("Failed to make mmap read-only");

    AudioWaveform {
        buffer: Some(Arc::new(mmap)),
        file_path: PathBuf::from("internal_beep"),
        sample_rate,
        channels: 2,
        duration: duration_secs as f64,
        trim_end: total_frames as u32,
        ..Default::default()
    }
}

pub fn init_engine() {
    let initial_state = {
        let app = ctx().app_state.read();
        AudioRenderState::from(&*app)
    };

    log::info!(
        "Init Engine with Buffer Size: {}",
        initial_state.graph.buffer_size
    );
    let (state_in, state_out) = triple_buffer::TripleBuffer::new(&initial_state).split();

    {
        let mut render_state_guard = ctx().render_state_producer.lock();
        *render_state_guard = Some(state_in);
    }
    // Capacity 128 is plenty for manual clicks
    let (cmd_prod, cmd_cons) = RingBuffer::new(128);

    // Store Producer in context
    let mut guard = ctx().command_sender.lock();
    *guard = Some(cmd_prod);

    match start_audio_stream(state_out, cmd_cons, initial_state) {
        Ok(_) => {
            log::info!("Audio Engine Successfully initialized");

            // SEND STARTUP BEEP
            if let Some(producer) = guard.as_mut() {
                let beep_waveform = generate_startup_beep();
                let _ = producer.push(AudioCommand::PlayOneShot(beep_waveform));
                log::info!("Startup beep command sent");
            }
        }
        Err(e) => {
            log::error!("Failed to start audio engine: {}", e);
            panic!()
        }
    }
}

pub fn init_logger() {
    // if release, use info, else use debug
    INIT_LOGGER.call_once(|| {
        use env_logger::Env;

        let default_level = if cfg!(debug_assertions) {
            "debug"
        } else {
            "info"
        };

        let _ = env_logger::Builder::from_env(Env::default().default_filter_or(default_level))
            .format_timestamp_millis()
            .target(env_logger::Target::Stdout)
            .try_init();
    });
}
