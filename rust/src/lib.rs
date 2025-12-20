pub mod plugin;
pub mod test;
pub mod utils;
// src/lib.rs

use std::sync::{Arc, Mutex, Once, RwLock};

use once_cell::sync::Lazy;
use rtrb::{Producer, RingBuffer};
use triple_buffer::Input;

use crate::{
    audio::{backend::start_audio_stream, render_state::{AudioGraphState, AudioRenderState}},
    commands::AudioCommand,
    core::{project::ApplicationState, track::audio_waveform::AudioWaveform},
};

pub mod api;
pub mod audio;
pub mod commands;
pub mod core;
mod frb_generated;

// ==================================================================
// ================== Global States variables =======================
// ==================================================================

pub static COMMAND_SENDER: Lazy<Mutex<Option<Producer<AudioCommand>>>> =
    Lazy::new(|| Mutex::new(None));

// SOURCE OF TRUTH For UI/Editing
pub static APP_STATE: Lazy<Arc<RwLock<ApplicationState>>> =
    Lazy::new(|| Arc::new(RwLock::new(ApplicationState::default())));

// Audio Bridge
// This input sits behind a Mutex, waiting for us to push updates
pub static RENDER_STATE_PRODUCER: Lazy<Mutex<Option<Input<AudioRenderState>>>> =
    Lazy::new(|| Mutex::new(None));

// SHADOW STATE
// This holds the last version of the state sent to the audio thread.
// It allows us to update the Transport without re-generating the AudioGraph, and vice versa.
static CURRENT_RENDER_STATE: Lazy<Mutex<AudioRenderState>> =
    Lazy::new(|| Mutex::new(AudioRenderState::default()));

pub static INIT_LOGGER: Once = Once::new();

// ==================================================================
// ================== Functions =====================================
// ==================================================================

/// Broadcast changes in ApplicationState to AudioRenderState (things that
/// is used by the Audio Thread)
pub fn broadcast_state_change() {
    // if read failed, we do nothing
    let Ok(app) = APP_STATE.read() else {
        return;
    };
    let render_state = AudioRenderState::from(&*app);

    drop(app); // Drop the read lock immediately so we don't hold it while waiting for the producer

    // Publish to Audio Thread
    if let Ok(mut guard) = RENDER_STATE_PRODUCER.lock() {
        if let Some(producer) = guard.as_mut() {
            {
                let mut input = producer.input_buffer_publisher();
                *input = render_state;
            }
            // producer.publish();
        }
    } else {
        log::error!("Error when publishing");
    }
}

/// Broadcast Structural Changes (Tracks, Plugins, Samples).
/// This is "Heavy". Call this only when tracks/plugins are added/removed.
pub fn sync_audio_graph() {
    let Ok(app) = APP_STATE.read() else { return };
    
    // Expensive operation: Rebuilds the graph structure from AppState
    let new_graph = AudioGraphState::from(&*app);  // This is kinda cheap because all large properties inside Graph State are actually Arc's
    drop(app); // Drop lock early

    let mut shadow = CURRENT_RENDER_STATE.lock().unwrap();
    shadow.graph = new_graph; // Update only the graph part
    
    // Push the composite state to the audio thread
    publish_to_audio_thread(&shadow);
}

/// Broadcast Transport Changes (Play/Stop, BPM).
/// This is "Light". Call this frequently.
pub fn sync_transport() {
    let Ok(app) = APP_STATE.read() else { return };
    
    let new_transport = app.transport.clone();
    drop(app);

    let mut shadow = CURRENT_RENDER_STATE.lock().unwrap();
    
    // Don't write if nothing changed
    if shadow.transport == new_transport { return; }

    shadow.transport = new_transport;
    
    publish_to_audio_thread(&shadow);
}

/// Helper to push state to TripleBuffer
fn publish_to_audio_thread(state: &AudioRenderState) {
    if let Ok(mut guard) = RENDER_STATE_PRODUCER.lock() {
        if let Some(producer) = guard.as_mut() {
            // Write to the back buffer (TripleBuffer handles the swap)
            {
                let mut input = producer.input_buffer_publisher();
                *input = state.clone();
            }
        }
    } else {
        log::error!("Error when publishing audio state");
    }
}

fn generate_startup_beep() -> AudioWaveform {
    let sample_rate = 48000;
    let duration_secs = 0.5;
    let total_frames = (sample_rate as f32 * duration_secs) as usize;
    let frequency = 440.0; // A4 Note

    let mut buffer = Vec::with_capacity(total_frames * 2); // Stereo

    for i in 0..total_frames {
        let t = i as f32 / sample_rate as f32;
        let signal = (t * frequency * 2.0 * std::f32::consts::PI).sin();
        let envelope = 1.0 - (i as f32 / total_frames as f32);

        let final_sample = signal * envelope * 0.3;

        buffer.push(final_sample); // Left
        buffer.push(final_sample); // Right
    }

    AudioWaveform {
        buffer: Arc::new(buffer),
        file_path: "internal_beep".to_string(),
        sample_rate,
        channels: 2,
        duration: duration_secs as f64,
        trim_end: total_frames as u64,
        ..Default::default()
    }
}

pub fn init_engine() {
    let initial_state = {
        let app = APP_STATE.read().unwrap();
        AudioRenderState::from(&*app)
    };

    log::info!(
        "Init Engine with Buffer Size: {}",
        initial_state.graph.buffer_size
    );
    let (state_in, state_out) = triple_buffer::TripleBuffer::new(&initial_state).split();

    {
        let mut render_state_guard = RENDER_STATE_PRODUCER.lock().unwrap();
        *render_state_guard = Some(state_in);
    }
    // Capacity 128 is plenty for manual clicks
    let (cmd_prod, cmd_cons) = RingBuffer::new(128);

    // Store Producer globally
    let mut guard;
    {
        guard = COMMAND_SENDER.lock().unwrap();
        *guard = Some(cmd_prod);
    }

    match start_audio_stream(state_out, cmd_cons, initial_state) {
        Ok(_) => {
            log::info!("Audio Engine Successfully initialized");

            // SEND STARTUP BEEP
            if let Some(producer) = guard.as_mut() {
                let beep_waveform = generate_startup_beep();
                // Push the command directly to the ring buffer
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
    INIT_LOGGER.call_once(|| {
        #[cfg(debug_assertions)]
        {
            use env_logger::Env;

            let _ = env_logger::Builder::from_env(Env::default().default_filter_or("debug"))
                .format_timestamp_millis()
                .try_init();
        }
    });
}
