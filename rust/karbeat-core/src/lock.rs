use karbeat_plugins::registry::PluginRegistry;
use parking_lot::{MutexGuard, RwLockReadGuard, RwLockWriteGuard};

use crate::{commands::AudioFeedback, context::ctx};

pub enum LockMode {
    Read,
    Write,
}

// Acquires a Read lock for application state
/// # Example
/// ```ignore
/// let app = ctx().app_state.read();
/// ```
pub fn get_app_read() -> RwLockReadGuard<'static, crate::core::project::ApplicationState> {
    ctx().app_state.read()
}

/// Acquires a Write lock. Panics if poisoned.
pub fn get_app_write() -> RwLockWriteGuard<'static, crate::core::project::ApplicationState> {
    ctx().app_state.write()
}

// --- History Lock ---

pub fn get_history_lock() -> MutexGuard<'static, crate::core::history::HistoryManager> {
    ctx().history.lock()
}

/// Get plugin registry read lock
pub fn get_plugin_registry_read(
) -> parking_lot::lock_api::RwLockReadGuard<'static, parking_lot::RawRwLock, PluginRegistry> {
    ctx().plugin_registry.read()
}

/// Get plugin registry write lock
pub fn get_plugin_registry_write(
) -> parking_lot::lock_api::RwLockWriteGuard<'static, parking_lot::RawRwLock, PluginRegistry> {
    ctx().plugin_registry.write()
}

pub fn get_audio_feedback_lock() -> parking_lot::lock_api::MutexGuard<
    'static,
    parking_lot::RawMutex,
    std::option::Option<rtrb::Consumer<AudioFeedback>>,
> {
    ctx().feedback_consumer.lock()
}
