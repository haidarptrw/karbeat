use std::time::Duration;

use crate::{
    APP_STATE, COMMAND_SENDER, api::project::AudioWaveformUiForAudioProperties, audio::{backend::POSITION_CONSUMER, event::PlaybackPosition}, broadcast_state_change, commands::AudioCommand, core::project::AudioHardwareConfig, frb_generated::StreamSink
};

// 1. GETTER: Fetch details + Downsampled Buffer for UI
pub fn get_audio_properties(id: u32) -> Option<AudioWaveformUiForAudioProperties> {
    let app = APP_STATE.read().ok()?;
    let waveform = app.asset_library.source_map.get(&id)?;
    Some(AudioWaveformUiForAudioProperties::from(waveform.as_ref()))
}

// 2. ACTION: Play the sound via the Engine
pub fn play_source_preview(id: u32) {
    {
        let app = match APP_STATE.read() {
            Ok(a) => a,
            Err(_) => return,
        };

        if let Some(waveform_arc) = app.asset_library.source_map.get(&id) {
            let waveform_to_play = (**waveform_arc).clone();

            if let Ok(mut guard) = COMMAND_SENDER.lock() {
                if let Some(sender) = guard.as_mut() {
                    // This matches the logic you requested
                    let _ = sender.push(AudioCommand::PlayOneShot(waveform_to_play));
                    println!("Preview command sent for ID: {}", id);
                }
            }
        }
    }

    broadcast_state_change();
}

pub fn stop_all_previews() {
    if let Ok(mut guard) = COMMAND_SENDER.lock() {
        if let Some(sender) = guard.as_mut() {
            let _ = sender.push(AudioCommand::StopAllPreviews);
            println!("Stop all preview sounds");
        }
    }
}

pub fn get_audio_config() -> Result<AudioHardwareConfig, String>{
    let Ok(app_state) = APP_STATE.read() else {
        return Err("failed to acquire Lock State".to_string());
    };

    Ok(app_state.audio_config.clone())
}

pub fn create_position_stream(sink: StreamSink<PlaybackPosition>) -> Result<(), String> {
    // Spawn a thread to poll the ring buffer
    std::thread::spawn(move || {
        loop {
            // Get access to the consumer
            if let Ok(mut guard) = POSITION_CONSUMER.lock() {
                if let Some(consumer) = guard.as_mut() {
                    //Read everything currently in the buffer
                    while let Ok(pos_data) = consumer.pop() {
                        // We map the Rust struct to something Dart understands
                        if sink.add(pos_data).is_err() {
                            return; 
                        }
                    }
                }
            }
            
            // 4. Sleep to prevent high CPU usage on this polling thread
            // 16ms ~= 60fps
            std::thread::sleep(Duration::from_millis(16));
        }
    });
    Ok(())
}
