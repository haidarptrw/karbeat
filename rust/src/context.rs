//! Centralized application context containing all shared state.
//!
//! This module replaces scattered lazy static globals with a single `KarbeatContext` struct
//! for improved testability and explicit dependencies.

use std::sync::{Arc, Mutex, Once, RwLock};

use once_cell::sync::Lazy;
use rtrb::Producer;
use triple_buffer::Input;

use crate::{
    api::mixer::MixerParamEvent,
    audio::{event::TransportFeedback, render_state::AudioRenderState},
    commands::{AudioCommand, AudioFeedback},
    core::{history::HistoryManager, project::ApplicationState},
    frb_generated::StreamSink,
    plugin::registry::PluginRegistry,
};

/// Centralized application context containing all shared state.
///
/// Access via the [`ctx()`] function to get a reference to the global instance.
pub struct KarbeatContext {
    /// Main application state (UI/editing source of truth)
    pub app_state: Arc<RwLock<ApplicationState>>,

    /// Undo/redo history manager
    pub history: Mutex<HistoryManager>,

    /// Audio command queue producer (UI → Audio)
    pub command_sender: Mutex<Option<Producer<AudioCommand>>>,

    /// Parameter feedback consumer (Audio → UI)
    pub feedback_consumer: Mutex<Option<rtrb::Consumer<AudioFeedback>>>,

    /// Triple buffer input for audio render state
    pub render_state_producer: Mutex<Option<Input<AudioRenderState>>>,

    /// Shadow state tracking last sent render state
    pub current_render_state: Mutex<AudioRenderState>,

    /// Audio stream handle
    pub stream_guard: Mutex<Option<cpal::Stream>>,

    /// Playback position ring buffer consumer
    pub position_consumer: Mutex<Option<rtrb::Consumer<TransportFeedback>>>,

    /// Plugin factory registry
    pub plugin_registry: RwLock<PluginRegistry>,

    /// Mixer parameter event stream sink (Rust → Flutter)
    pub mixer_event_sink: Mutex<Option<StreamSink<MixerParamEvent>>>,
}

impl KarbeatContext {
    fn new() -> Self {
        Self {
            app_state: Arc::new(RwLock::new(ApplicationState::default())),
            history: Mutex::new(HistoryManager::new()),
            command_sender: Mutex::new(None),
            feedback_consumer: Mutex::new(None),
            render_state_producer: Mutex::new(None),
            current_render_state: Mutex::new(AudioRenderState::default()),
            stream_guard: Mutex::new(None),
            position_consumer: Mutex::new(None),
            plugin_registry: RwLock::new(PluginRegistry::new_with_defaults()),
            mixer_event_sink: Mutex::new(None),
        }
    }
}

/// Logger initialization flag (kept separate as one-time init)
pub static INIT_LOGGER: Once = Once::new();

/// Global context instance
static CONTEXT: Lazy<KarbeatContext> = Lazy::new(KarbeatContext::new);

/// Access the global context.
///
/// # Example
/// ```ignore
/// let app = ctx().app_state.read().unwrap();
/// ```
#[inline]
pub fn ctx() -> &'static KarbeatContext {
    &CONTEXT
}
