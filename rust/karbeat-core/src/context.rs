//! Centralized application context containing all shared state.
//!
//! This module replaces scattered lazy static globals with a single `KarbeatContext` struct
//! for improved testability and explicit dependencies.

use std::sync::{ Arc, Once };

use once_cell::sync::Lazy;
use parking_lot::{ Mutex, RwLock };
use rtrb::Producer;
use triple_buffer::Input;

use crate::{
    audio::{ event::TransportFeedback, render_state::AudioRenderState },
    commands::{ AudioCommand, AudioFeedback },
    core::{ history::HistoryManager, project::ApplicationState },
};
use karbeat_plugins::registry::PluginRegistry;

// MixerParamEvent: FFI-friendly event pushed to Flutter when a mixer param changes
#[derive(Clone, Debug)]
pub struct MixerParamEvent {
    /// Track ID. `u32::MAX` means master bus.
    pub track_id: u32,
    pub volume: Option<f32>,
    pub pan: Option<f32>,
    pub mute: Option<bool>,
    pub solo: Option<bool>,
}

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
    pub mixer_event_sink: Mutex<Option<Box<dyn Fn(MixerParamEvent) + Send + Sync + 'static>>>,
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

pub mod utils {
    use karbeat_plugin_api::traits::{KarbeatEffect, KarbeatGenerator};
    use smallvec::SmallVec;

    use crate::{ commands::AudioCommand, context::ctx };

    /// Helper function to send AudioCommand to context's command sender
    pub fn send_audio_command(command: AudioCommand) {
        if let Some(sender) = ctx().command_sender.lock().as_mut() {
            let _ = sender.push(command);
        }
    }

    pub fn try_send_audio_command_chain(commands: SmallVec<[AudioCommand; 4]>) -> anyhow::Result<()> {
        if let Some(sender) = ctx().command_sender.lock().as_mut() {
            commands.into_iter().try_for_each(|command| sender.push(command))?;
        }

        Ok(())
    }

    pub fn get_effect_plugin_box(registry_id: u32) -> Option<Box<dyn KarbeatEffect + Send + Sync>> {
        let registry = ctx().plugin_registry.read();
        let Some((plugin, _)) = registry.create_effect_by_id(registry_id) else {
            return None;
        };

        Some(plugin)
    }

    pub fn get_synth_plugin_box(registry_id: u32) -> std::option::Option<Box<dyn KarbeatGenerator + Send + Sync>> {
        let registry = ctx().plugin_registry.read();
        let Some((plugin, _)) = registry.create_generator_by_id(registry_id) else {
            return None;
        };

        Some(plugin)
    }
}
