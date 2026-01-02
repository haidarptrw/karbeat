// src/api/transport.rs
// collections of transport API

use crate::{COMMAND_SENDER, broadcast_state_change, commands::AudioCommand, utils::lock::get_app_write};

pub fn set_playing(val: bool) -> Result<(), String> {
    {
        let mut app = get_app_write();

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
        let mut app = get_app_write();
        app.transport.is_looping = val;
    }
    broadcast_state_change();
    // sync_transport();
    Ok(())
}

pub fn set_bpm(val: f32) -> Result<(), String> {
    {
        let mut app = get_app_write();
        app.transport.bpm = val;
    }

    broadcast_state_change();
    Ok(())
}