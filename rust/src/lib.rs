// src/lib.rs

use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;
use rtrb::{Producer, RingBuffer};

use crate::{
    audio::{backend::start_audio_stream, render_state::AudioRenderState},
    commands::AudioCommand, core::track::audio_waveform::AudioWaveform,
};

pub mod api;
pub mod audio;
pub mod commands;
pub mod core;
mod frb_generated;

pub static COMMAND_SENDER: Lazy<Mutex<Option<Producer<AudioCommand>>>> =
    Lazy::new(|| Mutex::new(None));

fn generate_startup_beep() -> AudioWaveform {
    let sample_rate = 44100;
    let duration_secs = 0.5;
    let total_frames = (sample_rate as f32 * duration_secs) as usize;
    let frequency = 440.0; // A4 Note

    let mut buffer = Vec::with_capacity(total_frames * 2); // Stereo

    for i in 0..total_frames {
        // 1. Time
        let t = i as f32 / sample_rate as f32;
        
        // 2. Waveform (Sine)
        let signal = (t * frequency * 2.0 * std::f32::consts::PI).sin();
        
        // 3. Envelope (Linear Fade Out)
        // This makes it sound like a "Ping" instead of a harsh "BEEP"
        let envelope = 1.0 - (i as f32 / total_frames as f32);

        let final_sample = signal * envelope * 0.3; // 0.3 Volume

        // 4. Write Stereo
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
    let (_state_in, state_out) =
        triple_buffer::TripleBuffer::new(&AudioRenderState::default()).split();
    // Capacity 128 is plenty for manual clicks
    let (cmd_prod, cmd_cons) = RingBuffer::new(128);

    // Store Producer globally
    let mut guard = COMMAND_SENDER.lock().unwrap();
    *guard = Some(cmd_prod);

    // 3. Start Audio
    // Pass both 'state_out' and 'cmd_cons' to the engine
    match start_audio_stream(state_out, cmd_cons) {
        Ok(_) => {
            println!("Audio Engine Successfully initialized");

            // 3. SEND STARTUP BEEP
            // We use the guard we just locked to ensure it's ready
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
