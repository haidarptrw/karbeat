// src/api/transport.rs
// collections of transport API

use crate::{APP_STATE, COMMAND_SENDER, broadcast_state_change, commands::AudioCommand, sync_transport};

pub fn set_playing(val: bool) -> Result<(), String> {
    {
        let Ok(mut app) = APP_STATE.write() else {
            return Err("Failed acquiring lock".to_string()); // send empty
        };

        app.transport.is_playing = val;
    }
    broadcast_state_change();
    Ok(())
}

pub fn set_playhead(val: u32) -> Result<(), String> {
    {
        if let Ok(mut guard) = COMMAND_SENDER.lock() {
            if let Some(cmd_producer) = guard.as_mut() {
                let _ = cmd_producer.push(AudioCommand::SetPlayhead(val));
            }
        }
    }
    broadcast_state_change();
    Ok(())
}

pub fn set_looping(val: bool) -> Result<(), String> {
    {
        let Ok(mut app) = APP_STATE.write() else {
            return Err("Failed acquiring write lock".to_string()); // send empty
        };
        app.transport.is_looping = val;
    }
    broadcast_state_change();
    // sync_transport();
    Ok(())
}

pub fn set_bpm(val: f32) -> Result<(), String> {
    {
        let mut app = APP_STATE.write().map_err(|e| format!("POISON ERROR: {}", e))?;
        app.transport.bpm = val;
    }

    broadcast_state_change();
    Ok(())
}