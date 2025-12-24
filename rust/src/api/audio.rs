use std::time::Duration;

use crate::{
    api::project::AudioWaveformUiForAudioProperties,
    audio::{backend::POSITION_CONSUMER, event::PlaybackPosition},
    commands::AudioCommand,
    core::project::AudioHardwareConfig,
    frb_generated::StreamSink,
    APP_STATE, COMMAND_SENDER,
};

/// GETTER: Fetch details + Downsampled Buffer for UI
pub fn get_audio_properties(id: u32) -> Option<AudioWaveformUiForAudioProperties> {
    let app = APP_STATE.read().ok()?;
    let waveform = app.asset_library.source_map.get(&id)?;
    Some(AudioWaveformUiForAudioProperties::from(waveform.as_ref()))
}

/// ACTION: Play the sound via the Engine
pub fn play_source_preview(id: u32) {
    let app = match APP_STATE.read() {
        Ok(a) => a,
        Err(_) => return,
    };

    if let Some(waveform_arc) = app.asset_library.source_map.get(&id) {
        let waveform_to_play = (**waveform_arc).clone();

        if let Ok(mut guard) = COMMAND_SENDER.lock() {
            if let Some(sender) = guard.as_mut() {
                let _ = sender.push(AudioCommand::PlayOneShot(waveform_to_play));
                log::info!("Preview command sent for ID: {}", id);
            }
        }
    }
}

pub fn stop_all_previews() {
    if let Ok(mut guard) = COMMAND_SENDER.lock() {
        if let Some(sender) = guard.as_mut() {
            let _ = sender.push(AudioCommand::StopAllPreviews);
            println!("Stop all preview sounds");
        }
    }
}

pub fn get_audio_config() -> Result<AudioHardwareConfig, String> {
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
                            println!(
                                "[Rust] PlaybackPosition Stream disconnected! Stopping thread."
                            );
                            return;
                        }
                    }
                }
            }

            // Sleep to prevent high CPU usage on this polling thread
            // 16ms ~= 60fps
            std::thread::sleep(Duration::from_millis(16));
        }
    });
    Ok(())
}

/// play preview sound when drawing note or pressing the piano tile on the UI
pub fn play_preview_note(
    track_id: u32,
    note_key: i32,
    velocity: i32,
    is_on: bool,
) -> Result<(), String> {
    // validate input
    if note_key < 0 || note_key > 127 {
        return Err("Note key must be between 0 and 127".to_string());
    }

    if velocity < 0 || note_key > 100 {
        return Err("Note velocity must be between 0 and 100".to_string());
    }

    let note_key: u8 = note_key as u8;

    let velocity: u8 = velocity as u8;

    let generator_id = {
        let app = APP_STATE.read().map_err(|e| format!("{}", e))?;
        let track = app
            .tracks
            .get(&track_id)
            .ok_or("Can't find requested track")?;
        track.generator.as_ref().ok_or("Track has no generator")?.id
    };

    if let Ok(mut command_guard) = COMMAND_SENDER.lock() {
        if let Some(sender) = command_guard.as_mut() {
            let _ = sender.push(AudioCommand::PlayPreviewNote {
                note_key: note_key,
                generator_id,
                velocity,
                is_note_on: is_on,
            });
        }
    }

    Ok(())
}
