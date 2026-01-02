use crate::{APP_STATE, HISTORY};

pub enum LockMode {
    Read,
    Write,
}

// Acquires a Read lock. Panics if poisoned.
pub fn get_app_read() -> std::sync::RwLockReadGuard<'static, crate::core::project::ApplicationState>
{
    APP_STATE
        .read()
        .expect("CRITICAL: APP_STATE (Read) lock is poisoned. Application state is corrupt.")
}

/// Acquires a Write lock. Panics if poisoned.
pub fn get_app_write(
) -> std::sync::RwLockWriteGuard<'static, crate::core::project::ApplicationState> {
    APP_STATE
        .write()
        .expect("CRITICAL: APP_STATE (Write) lock is poisoned. Application state is corrupt.")
}

// --- History Lock ---

pub fn get_history_lock() -> std::sync::MutexGuard<'static, crate::core::history::HistoryManager> {
    HISTORY.lock().expect("CRITICAL: HISTORY lock is poisoned.")
}
