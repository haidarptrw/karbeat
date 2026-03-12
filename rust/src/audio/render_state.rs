use std::{collections::HashMap, sync::Arc};

use crate::{
    core::project::{
        automation::{AutomationId, AutomationPoint, AutomationTarget, CurveType},
        mixer::{BusId, EffectId, MixerState},
        plugin::{KarbeatEffect, KarbeatGenerator},
        track::{
            midi::{Pattern, PatternId},
            KarbeatTrack,
        },
        transport::TransportState,
        ApplicationState, AssetLibrary, GeneratorId, TrackId,
    },
    utils::math::is_power_of_two,
};

// =============================================================================
// Audio Thread Owned Plugin State
// =============================================================================

/// A generator plugin instance owned by the audio thread
pub struct AudioGeneratorInstance {
    pub id: GeneratorId,
    pub track_id: TrackId,
    pub plugin: Box<dyn KarbeatGenerator + Send>,
}

pub struct AudioEffectInstance {
    pub id: EffectId,
    pub plugin: Box<dyn KarbeatEffect + Send>,
}

/// Audio thread's owned plugin instances - NO locks required for access
/// This is managed via AudioCommand, NOT cloned from ApplicationState
#[derive(Default)]
pub struct AudioPluginState {
    /// Generator plugins keyed by GeneratorId
    pub generators: HashMap<GeneratorId, AudioGeneratorInstance>,
    /// Effect chain per track (owned by audio thread)
    pub track_effects: HashMap<TrackId, Vec<AudioEffectInstance>>,
    /// Master effect chain (owned by audio thread)
    pub master_effects: Vec<AudioEffectInstance>,
    /// Bus effect chains (owned by audio thread)
    pub bus_effects: HashMap<BusId, Vec<AudioEffectInstance>>,
}

// =============================================================================
// Cloneable Graph State (metadata only, no plugin instances)
// =============================================================================

/// Lightweight automation lane snapshot for the audio thread.
/// Contains only the data needed for real-time interpolation.
#[derive(Clone, Debug)]
pub struct AudioAutomationLane {
    pub target: AutomationTarget,
    pub points: Vec<AutomationPoint>,
    pub enabled: bool,
    pub min: f32,
    pub max: f32,
    pub default_value: f32,
}

impl AudioAutomationLane {
    /// Get the denormalized value at a given time in ticks.
    /// Returns `default_value` (denormalized) if disabled or no points.
    #[inline]
    pub fn value_at_ticks(&self, time_ticks: u32) -> f32 {
        if !self.enabled || self.points.is_empty() {
            return self.denormalize(self.default_value);
        }
        let normalized = interpolate_points(&self.points, time_ticks);
        self.denormalize(normalized)
    }

    #[inline]
    fn denormalize(&self, normalized: f32) -> f32 {
        self.min + normalized * (self.max - self.min)
    }
}

/// Interpolate sorted automation points at the given time in ticks.
/// Returns a normalized value (0.0–1.0).
#[inline]
fn interpolate_points(points: &[AutomationPoint], time_ticks: u32) -> f32 {
    // Before first point
    if time_ticks <= points[0].time_ticks {
        return points[0].value;
    }

    // After last point
    let last = &points[points.len() - 1];
    if time_ticks >= last.time_ticks {
        return last.value;
    }

    // Binary search for the surrounding pair
    let idx = points
        .binary_search_by(|p| p.time_ticks.cmp(&time_ticks))
        .unwrap_or_else(|i| i);

    if idx == 0 {
        return points[0].value;
    }

    let p1 = &points[idx - 1];
    let p2 = &points[idx];
    let duration = p2.time_ticks.saturating_sub(p1.time_ticks);
    if duration == 0 {
        return p1.value;
    }

    let t = ((time_ticks - p1.time_ticks) as f32) / (duration as f32);

    match p1.curve_type {
        CurveType::Linear => p1.value + (p2.value - p1.value) * t,
        CurveType::Exponential => {
            let v1 = p1.value.max(0.0001);
            let v2 = p2.value.max(0.0001);
            v1 * (v2 / v1).powf(t)
        }
        CurveType::Step => p1.value,
    }
}

/// Structural State: Tracks, Patterns, Mixer, Assets (Heavy, changes rarely)
#[derive(Default, Clone)]
pub struct AudioGraphState {
    pub tracks: Arc<[Arc<KarbeatTrack>]>,
    pub patterns: HashMap<PatternId, Arc<Pattern>>,
    pub mixer_state: MixerState,
    pub asset_library: Arc<AssetLibrary>,
    /// Automation lanes for real-time parameter modulation
    pub automation_lanes: HashMap<AutomationId, AudioAutomationLane>,
    pub max_sample_index: u32,
    pub sample_rate: u32,
    pub buffer_size: usize,
}

impl From<&ApplicationState> for AudioGraphState {
    fn from(app: &ApplicationState) -> Self {
        let mut tracks_vec: Vec<Arc<KarbeatTrack>> = app.tracks.values().cloned().collect();
        tracks_vec.sort_by_key(|t| t.id);

        // Convert automation pool to lightweight audio-thread snapshots
        let automation_lanes: HashMap<AutomationId, AudioAutomationLane> = app
            .automation_pool
            .iter()
            .filter(|(_, lane)| lane.enabled && !lane.points.is_empty())
            .map(|(&id, lane)| {
                (
                    id,
                    AudioAutomationLane {
                        target: lane.target.clone(),
                        points: lane.points.clone(),
                        enabled: lane.enabled,
                        min: lane.min,
                        max: lane.max,
                        default_value: lane.default_value,
                    },
                )
            })
            .collect();

        Self {
            tracks: Arc::from(tracks_vec),
            patterns: app.pattern_pool.clone(),
            mixer_state: app.mixer.clone(),
            asset_library: app.asset_library.clone(),
            automation_lanes,
            max_sample_index: app.max_sample_index,
            sample_rate: app.audio_config.sample_rate,
            buffer_size: if is_power_of_two(app.audio_config.buffer_size.into()) {
                app.audio_config.buffer_size as usize
            } else {
                64
            },
        }
    }
}

/// Consolidated State wrapper for the Audio Thread
#[derive(Clone)]
pub struct AudioRenderState {
    pub graph: AudioGraphState,
    // Transport is now separate to allow fast updates without full graph clone
    // However, for backward compatibility with your TripleBuffer setup,
    // we can keep a unified struct if your architecture requires a single atomic update.
    // If you implemented the split buffers (graph_in, transport_in), this struct is not needed as a monolith.
    // Assuming we stick to the monolithic struct for `state_consumer` in `AudioEngine`:
    pub transport: TransportState,
}

impl Default for AudioRenderState {
    fn default() -> Self {
        Self {
            graph: AudioGraphState::default(),
            transport: TransportState::default(),
        }
    }
}

impl From<&ApplicationState> for AudioRenderState {
    fn from(app: &ApplicationState) -> Self {
        Self {
            graph: AudioGraphState::from(app),
            transport: app.transport.clone(),
        }
    }
}
