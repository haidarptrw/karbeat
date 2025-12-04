pub mod utils;
// src/lib.rs

use std::sync::{Arc, Mutex, RwLock};

use once_cell::sync::Lazy;
use rtrb::{Producer, RingBuffer};
use triple_buffer::Input;

use crate::{
    audio::{backend::start_audio_stream, render_state::AudioRenderState},
    commands::AudioCommand, core::{project::ApplicationState, track::audio_waveform::AudioWaveform},
};

pub mod api;
pub mod audio;
pub mod commands;
pub mod core;
mod frb_generated;

pub static COMMAND_SENDER: Lazy<Mutex<Option<Producer<AudioCommand>>>> =
    Lazy::new(|| Mutex::new(None));

// SOURCE OF TRUTH For UI/Editing
pub static APP_STATE: Lazy<Arc<RwLock<ApplicationState>>> = Lazy::new(|| {
    Arc::new(RwLock::new(ApplicationState::default()))
});

// Audio Bridge
// This input sits behind a Mutex, waiting for us to push updates
pub static RENDER_STATE_PRODUCER: Lazy<Mutex<Option<Input<AudioRenderState>>>> = 
    Lazy::new(|| Mutex::new(None));

/// Broadcast changes in ApplicationState to AudioRenderState (things that
/// is used by the Audio Thread)
pub fn broadcast_state_change() {
    // if read failed, we do nothing
    let Ok(app )= APP_STATE.read() else {return;};
    let render_state = AudioRenderState::from(&*app);
    drop(app); // Drop the read lock immediately so we don't hold it while waiting for the producer
    
    // Publish to Audio Thread
    if let Ok(mut guard) = RENDER_STATE_PRODUCER.lock() {
        if let Some(producer) = guard.as_mut() {
            producer.write(render_state);
            producer.publish(); // Instant swap
        }
    }
}

fn generate_startup_beep() -> AudioWaveform {
    let sample_rate = 44100;
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
    let (state_in, state_out) =
        triple_buffer::TripleBuffer::new(&AudioRenderState::default()).split();

    *RENDER_STATE_PRODUCER.lock().unwrap() = Some(state_in);

    // Capacity 128 is plenty for manual clicks
    let (cmd_prod, cmd_cons) = RingBuffer::new(128);

    // Store Producer globally
    let mut guard = COMMAND_SENDER.lock().unwrap();
    *guard = Some(cmd_prod);

    match start_audio_stream(state_out, cmd_cons) {
        Ok(_) => {
            println!("Audio Engine Successfully initialized");

            // SEND STARTUP BEEP
            if let Some(producer) = guard.as_mut() {
                let beep_waveform = generate_startup_beep();
                // Push the command directly to the ring buffer
                let _ = producer.push(AudioCommand::PlayOneShot(beep_waveform));
                println!("Startup beep command sent");
            }
        },
        Err(e) => eprintln!("Failed to start audio engine: {}", e),
    }
}
