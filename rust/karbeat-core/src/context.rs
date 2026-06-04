//! Centralized application context containing all shared state.
//!
//! This module replaces scattered lazy static globals with a single `KarbeatContext` struct
//! for improved testability and explicit dependencies.

use std::sync::{Arc, Once};

use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use rtrb::Producer;

use crate::{
    audio::event::TransportFeedback,
    commands::{AudioCommand, AudioFeedback},
    core::{history::HistoryManager, project::ApplicationState},
};
use karbeat_plugins::registry::PluginRegistry;

/// Centralized application context containing all shared state.
///
/// Access via the [`ctx()`] function to get a reference to the global instance.
pub struct DawContext {
    /// Main application state (UI/editing source of truth)
    pub app_state: Arc<RwLock<ApplicationState>>,

    /// Undo/redo history manager
    pub history: Mutex<HistoryManager>,

    /// Audio command queue producer (UI → Audio)
    pub command_sender: Mutex<Option<Producer<AudioCommand>>>,

    /// Parameter feedback consumer (Audio → UI)
    pub feedback_consumer: Mutex<Option<rtrb::Consumer<AudioFeedback>>>,

    /// Audio stream handle
    pub stream_guard: Mutex<Option<cpal::Stream>>,

    /// Playback position ring buffer consumer
    pub position_consumer: Mutex<Option<rtrb::Consumer<TransportFeedback>>>,

    /// Plugin factory registry
    pub plugin_registry: RwLock<PluginRegistry>,

    /// Shared pending feedback buffer for all modules to poll (Rust → Flutter)
    pub pending_feedback: Mutex<Vec<AudioFeedback>>,
}

impl DawContext {
    fn new() -> Self {
        Self {
            app_state: Arc::new(RwLock::new(ApplicationState::default())),
            history: Mutex::new(HistoryManager::new()),
            command_sender: Mutex::new(None),
            feedback_consumer: Mutex::new(None),
            stream_guard: Mutex::new(None),
            position_consumer: Mutex::new(None),
            plugin_registry: RwLock::new(PluginRegistry::new_with_defaults()),
            pending_feedback: Mutex::new(Vec::new()),
        }
    }
}

/// Logger initialization flag (kept separate as one-time init)
pub static INIT_LOGGER: Once = Once::new();

/// Global context instance
static CONTEXT: Lazy<DawContext> = Lazy::new(DawContext::new);

/// Access the global context.
///
/// # Example
/// ```ignore
/// let app = ctx().app_state.read();
/// ```
#[inline]
pub fn ctx() -> &'static DawContext {
    &CONTEXT
}

pub mod utils {
    use std::sync::Arc;

    use karbeat_plugin_api::traits::AudioPlugin;

    use crate::{
        audio::render_state::{AudioAutomationLane, AudioGraphState},
        commands::AudioCommand,
        context::ctx,
        lock::get_app_read,
        shared::id::*,
    };

    /// Helper function to send AudioCommand to context's command sender.
    pub fn send_audio_command(command: AudioCommand) {
        if let Some(sender) = ctx().command_sender.lock().as_mut() {
            let _ = sender.push(command);
        }
    }

    pub fn try_send_audio_command_chain(commands: Vec<AudioCommand>) -> anyhow::Result<()> {
        if let Some(sender) = ctx().command_sender.lock().as_mut() {
            commands.into_iter().for_each(|command| {
                let _ = sender.push(command);
            });
        }

        Ok(())
    }

    pub fn get_effect_plugin_box(registry_id: u32) -> Option<Box<dyn AudioPlugin + Send + Sync>> {
        let registry = ctx().plugin_registry.read();
        let Some((plugin, _)) = registry.create_effect_by_id(registry_id) else {
            return None;
        };

        Some(plugin)
    }

    pub fn get_synth_plugin_box(
        registry_id: u32,
    ) -> std::option::Option<Box<dyn AudioPlugin + Send + Sync>> {
        let registry = ctx().plugin_registry.read();
        let Some((plugin, _)) = registry.create_generator_by_id(registry_id) else {
            return None;
        };

        Some(plugin)
    }

    // ======================================
    // Granular graph-state broadcast helpers
    // ======================================

    /// Push an UpdateTrackGraph command to the audio thread from the current AppState.
    /// Call this whenever tracks, clips, patterns, or max_sample_index change.
    pub fn broadcast_track_graph() {
        let app = get_app_read();
        let mut tracks_vec: Vec<Arc<crate::core::project::track::AudioTrack>> =
            app.tracks.values().cloned().collect();
        tracks_vec.sort_by_key(|t| t.id);

        send_audio_command(AudioCommand::UpdateTrackGraph {
            tracks: Arc::from(tracks_vec),
            patterns: app.pattern_pool.clone(),
            max_sample_index: app.max_sample_index,
        });
    }

    /// Push a ReplaceFullGraph command built from the current AppState.
    /// Only used for undo/redo and full project loads.
    pub fn broadcast_full_graph() {
        let app = get_app_read();
        let graph = AudioGraphState::from(&*app);
        drop(app);
        send_audio_command(AudioCommand::ReplaceFullGraph { graph });
    }

    /// Push an UpdateAutomationLane command for the given automation ID.
    pub fn broadcast_automation_lane(id: AutomationId) {
        let app = get_app_read();
        let Some(lane_arc) = app.automation_pool.get(&id) else {
            return;
        };
        let lane = AudioAutomationLane {
            points: lane_arc.points.clone(),
            enabled: lane_arc.enabled,
            min: lane_arc.min,
            max: lane_arc.max,
            default_value: lane_arc.default_value,
        };
        drop(app);
        send_audio_command(AudioCommand::UpdateAutomationLane { id, lane });
    }
}
