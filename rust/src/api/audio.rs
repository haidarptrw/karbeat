use std::time::Duration;

use crate::{
    api::project::AudioWaveformUiForAudioProperties, audio::event::PlaybackPosition,
    commands::AudioCommand, core::project::AudioHardwareConfig, ctx, frb_generated::StreamSink,
    utils::lock::get_app_read,
};

/// GETTER: Fetch details + Downsampled Buffer for UI
pub fn get_audio_properties(id: u32) -> Option<AudioWaveformUiForAudioProperties> {
    let app = get_app_read();
    let waveform = app.asset_library.source_map.get(&id.into())?;
    Some(AudioWaveformUiForAudioProperties::from(waveform.as_ref()))
}

/// ACTION: Play the sound via the Engine
pub fn play_source_preview(id: u32) {
    let app = get_app_read();

    if let Some(waveform_arc) = app.asset_library.source_map.get(&id.into()) {
        let waveform_to_play = (**waveform_arc).clone();

        if let Ok(mut guard) = ctx().command_sender.lock() {
            if let Some(sender) = guard.as_mut() {
                let _ = sender.push(AudioCommand::PlayOneShot(waveform_to_play));
                log::info!("Preview command sent for ID: {}", id);
            }
        }
    }
}

pub fn stop_all_previews() {
    if let Ok(mut guard) = ctx().command_sender.lock() {
        if let Some(sender) = guard.as_mut() {
            let _ = sender.push(AudioCommand::StopAllPreviews);
            println!("Stop all preview sounds");
        }
    }
}

pub fn get_audio_config() -> Result<AudioHardwareConfig, String> {
    let app_state = get_app_read();
    Ok(app_state.audio_config.clone())
}

pub fn create_position_stream(sink: StreamSink<PlaybackPosition>) -> Result<(), String> {
    // Spawn a thread to poll the ring buffer
    std::thread::spawn(move || {
        loop {
            // Get access to the consumer
            if let Ok(mut guard) = ctx().position_consumer.lock() {
                if let Some(consumer) = guard.as_mut() {
                    //Read everything currently in the buffer
                    while let Ok(pos_data) = consumer.pop() {
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
        let app = get_app_read();
        let track = app
            .tracks
            .get(&track_id.into())
            .ok_or("Can't find requested track")?;
        track.generator.as_ref().ok_or("Track has no generator")?.id
    };

    if let Ok(mut command_guard) = ctx().command_sender.lock() {
        if let Some(sender) = command_guard.as_mut() {
            let _ = sender.push(AudioCommand::PlayPreviewNote {
                note_key: note_key,
                generator_id: generator_id.into(),
                velocity,
                is_note_on: is_on,
            });
        }
    }

    Ok(())
}

/// Play preview sound directly on a generator (without requiring a track).
/// Used in plugin editor screens to test synth sounds.
pub fn play_preview_note_generator(
    generator_id: u32,
    note_key: i32,
    velocity: i32,
    is_on: bool,
) -> Result<(), String> {
    // validate input
    if note_key < 0 || note_key > 127 {
        return Err("Note key must be between 0 and 127".to_string());
    }

    if velocity < 0 || velocity > 127 {
        return Err("Note velocity must be between 0 and 127".to_string());
    }

    let note_key: u8 = note_key as u8;
    let velocity: u8 = velocity as u8;

    if let Ok(mut command_guard) = ctx().command_sender.lock() {
        if let Some(sender) = command_guard.as_mut() {
            let _ = sender.push(AudioCommand::PlayPreviewNote {
                note_key,
                generator_id: generator_id.into(),
                velocity,
                is_note_on: is_on,
            });
        }
    }

    Ok(())
}
