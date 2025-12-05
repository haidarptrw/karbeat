// src/api/transport.rs
// collections of transport API

use crate::{broadcast_state_change, commands::AudioCommand, APP_STATE, COMMAND_SENDER};

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
    Ok(())
}
