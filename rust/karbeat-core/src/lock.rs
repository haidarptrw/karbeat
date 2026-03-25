use parking_lot::{MutexGuard, RwLockReadGuard, RwLockWriteGuard};

use crate::context::ctx;

pub enum LockMode {
    Read,
    Write,
}

// Acquires a Read lock. Panics if poisoned.
/// # Example
/// ```ignore
/// let app = ctx().app_state.read().unwrap();
/// ```
pub fn get_app_read() -> RwLockReadGuard<'static, crate::core::project::ApplicationState>
{
    ctx()
        .app_state
        .read()
}

/// Acquires a Write lock. Panics if poisoned.
pub fn get_app_write(
) -> RwLockWriteGuard<'static, crate::core::project::ApplicationState> {
    ctx()
        .app_state
        .write()
}

// --- History Lock ---

pub fn get_history_lock() -> MutexGuard<'static, crate::core::history::HistoryManager> {
    ctx()
        .history
        .lock()
}
