use crate::{
    api::project::AudioWaveformUiForAudioProperties, broadcast_state_change,
    commands::AudioCommand, APP_STATE, COMMAND_SENDER,
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

