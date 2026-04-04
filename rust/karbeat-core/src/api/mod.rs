pub mod note;
pub mod clip;

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
