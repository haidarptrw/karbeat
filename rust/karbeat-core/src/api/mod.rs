pub mod project_api;
pub mod audio_waveform_api;
pub mod mixer_api;
pub mod pattern_api;
pub mod note_api;
pub mod clip_api;
pub mod track_api;

use crate::lock::{get_app_write, get_history_lock};

pub fn undo() -> Result<(), String> {
    let mut history = get_history_lock();
    let mut app = get_app_write();
    history.undo(&mut app)
}

pub fn redo() -> Result<(), String> {
    let mut history = get_history_lock();
    let mut app = get_app_write();
    history.redo(&mut app)
}
