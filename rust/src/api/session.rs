use crate::{APP_STATE, broadcast_state_change};

pub fn update_selected_clip(track_id: u32, clip_id: u32) -> Result<(), String> {
    {
        let mut app = APP_STATE.write().map_err(|e|format!("Poisoned error: NOTE: this should panic to prevent data corruption across threads"))?;

        app.session.selected_track_id = Some(track_id);
        app.session.selected_clip_id = Some(clip_id);
    }
    Ok(())
}

pub fn deselect_clip() -> Result<(), String> {
    {
        let mut app = APP_STATE.write().map_err(|e|format!("Poisoned error: NOTE: this should panic to prevent data corruption across threads"))?;
        app.session.selected_track_id = None;
        app.session.selected_clip_id = None;
    }
    Ok(())
}