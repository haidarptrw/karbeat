// src/api/transport.rs
// collections of transport API

use crate::{APP_STATE, broadcast_state_change};


pub fn set_playing(val: bool) -> Result<(), String> {
    {
        let Ok(mut app )= APP_STATE.write() else {
            return Err("Failed acquiring lock".to_string()); // send empty
        };

        app.transport.is_playing = val;
    }
    broadcast_state_change();
    Ok(())
}

pub fn set_playhead(val: u32) -> Result<(), String>  {
    {
        let Ok(mut app )= APP_STATE.write() else {
            return Err("Failed acquiring write lock".to_string()); // send empty
        };
        app.transport.playhead_position_samples = val as u64;
    }
    broadcast_state_change();
    Ok(())
}

pub fn set_looping(val: bool) -> Result<(), String> {
    {
        let Ok(mut app )= APP_STATE.write() else {
            return Err("Failed acquiring write lock".to_string()); // send empty
        };
        app.transport.is_looping = val;
    }
    broadcast_state_change();
    Ok(())
}

