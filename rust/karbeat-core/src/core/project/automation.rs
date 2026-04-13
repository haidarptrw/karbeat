use karbeat_utils::define_id;
// src/core/project/track/automation.rs
//
// Automation system for parameter modulation over time.
// Provides both project-level automation lane data (saved with the project)
// and a runtime AutomationManager used by plugin wrappers during audio processing.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    core::project::{
        mixer::{BusId, EffectId},
        track::TrackId,
    },
    
};

// ============================================================================
// IDs
// ============================================================================

define_id!(AutomationId);

// ============================================================================
// AUTOMATION TARGET
// ============================================================================

/// Specifies what parameter an automation lane controls.
///
/// Each lane targets exactly one parameter on one thing (mixer channel,
/// generator, or effect slot).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AutomationTarget {
    /// A parameter on the track's generator plugin
    TrackGeneratorPluginParam {
        track_id: TrackId,
        param_id: u32,
    },

    // Track Targets
    TrackVolume(TrackId),
    TrackPan(TrackId),
    TrackPluginParam {
        track_id: TrackId,
        effect_id: EffectId,
        param_id: u32,
    },

    // Bus Targets
    BusVolume(BusId),
    BusPan(BusId),
    BusPluginParam {
        bus_id: BusId,
        effect_id: EffectId,
        param_id: u32,
    },

    // Master Targets
    MasterVolume,
    MasterPan,
    MasterPluginParam {
        effect_id: EffectId,
        param_id: u32,
    },

    // Global Targets
    TempoBpm,
}

impl AutomationTarget {
    /// Returns true if this target references the given track ID.
    pub fn references_track(&self, id: TrackId) -> bool {
        match self {
            AutomationTarget::TrackGeneratorPluginParam { track_id, .. }
            | AutomationTarget::TrackVolume(track_id)
            | AutomationTarget::TrackPan(track_id)
            | AutomationTarget::TrackPluginParam { track_id, .. } => *track_id == id,
            _ => false,
        }
    }
}
// ============================================================================
// CURVE TYPES
// ============================================================================

/// Interpolation curve type between automation points
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum CurveType {
    /// Linear interpolation between points
    #[default]
    Linear,
    /// Exponential curve (good for frequency, volume)
    Exponential,
    /// Instant step (no interpolation)
    Step,
}

// ============================================================================
// AUTOMATION POINT
// ============================================================================

/// A single point on an automation lane.
///
/// Values are stored in normalized form (0.0–1.0). The lane's `min`/`max`
/// fields define the mapping to the actual parameter range.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AutomationPoint {
    /// Position in ticks (relative to project start)
    pub time_ticks: u32,
    /// Normalized parameter value (0.0–1.0)
    pub value: f32,
    /// Interpolation curve to the NEXT point
    pub curve_type: CurveType,
}

impl AutomationPoint {
    pub fn new(time_ticks: u32, value: f32) -> Self {
        Self {
            time_ticks,
            value: value.clamp(0.0, 1.0),
            curve_type: CurveType::Linear,
        }
    }

    pub fn with_curve(time_ticks: u32, value: f32, curve_type: CurveType) -> Self {
        Self {
            time_ticks,
            value: value.clamp(0.0, 1.0),
            curve_type,
        }
    }
}

// ============================================================================
// AUTOMATION LANE
// ============================================================================

/// An automation lane that controls a single parameter.
///
/// Lives in `ApplicationState::automation_pool` and is serialized with the project.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AutomationLane {
    pub id: AutomationId,
    /// What this lane controls
    pub target: AutomationTarget,
    /// Human-readable label (e.g. "Volume", "Filter Cutoff")
    pub label: String,
    /// Automation points sorted by time
    pub points: Vec<AutomationPoint>,
    /// Whether this lane is active
    pub enabled: bool,
    /// Minimum value of the target parameter (for display/denormalization)
    pub min: f32,
    /// Maximum value of the target parameter (for display/denormalization)
    pub max: f32,
    /// Default value of the target parameter (normalized 0.0–1.0)
    pub default_value: f32,
}

impl AutomationLane {
    /// Create a new empty automation lane for the given target on the given track.
    pub fn new(
        id: AutomationId,
        target: AutomationTarget,
        label: impl Into<String>,
        min: f32,
        max: f32,
        default_value: f32,
    ) -> Self {
        Self {
            id,
            target,
            label: label.into(),
            points: Vec::new(),
            enabled: true,
            min,
            max,
            default_value,
        }
    }

    /// Add a point to the lane (maintains sorted order by time).
    pub fn add_point(&mut self, point: AutomationPoint) {
        let idx = self
            .points
            .binary_search_by(|p| p.time_ticks.cmp(&point.time_ticks))
            .unwrap_or_else(|i| i);
        self.points.insert(idx, point);
    }

    /// Remove a point at the given index.
    pub fn remove_point(&mut self, index: usize) -> Option<AutomationPoint> {
        if index < self.points.len() {
            Some(self.points.remove(index))
        } else {
            None
        }
    }

    /// Update a point at the given index.
    pub fn update_point(&mut self, index: usize, time_ticks: u32, value: f32) -> bool {
        if let Some(point) = self.points.get_mut(index) {
            point.time_ticks = time_ticks;
            point.value = value.clamp(0.0, 1.0);

            // Re-sort after update (point may have moved in time)
            self.points.sort_by(|a, b| a.time_ticks.cmp(&b.time_ticks));
            true
        } else {
            false
        }
    }

    /// Get the interpolated normalized value (0.0–1.0) at a given time in ticks.
    /// Returns `None` if the lane is disabled or has no points.
    pub fn value_at(&self, time_ticks: u32) -> Option<f32> {
        if !self.enabled || self.points.is_empty() {
            return None;
        }

        // Before first point: return first point's value
        if time_ticks <= self.points[0].time_ticks {
            return Some(self.points[0].value);
        }

        // After last point: return last point's value
        let last = self.points.last()?;
        if time_ticks >= last.time_ticks {
            return Some(last.value);
        }

        // Find surrounding points using binary search
        let idx = self
            .points
            .binary_search_by(|p| p.time_ticks.cmp(&time_ticks))
            .unwrap_or_else(|i| i);

        // idx is where we'd insert, so points[idx-1] <= time < points[idx]
        if idx == 0 {
            return Some(self.points[0].value);
        }

        let p1 = &self.points[idx - 1];
        let p2 = &self.points[idx];

        // Calculate interpolation factor (0.0 to 1.0)
        let duration = p2.time_ticks.saturating_sub(p1.time_ticks);
        if duration == 0 {
            return Some(p1.value);
        }

        let t = ((time_ticks - p1.time_ticks) as f32) / (duration as f32);

        // Interpolate based on curve type of the FIRST point
        let value = match p1.curve_type {
            CurveType::Linear => lerp(p1.value, p2.value, t),
            CurveType::Exponential => {
                // Exponential interpolation (good for frequency/volume)
                // Avoid log(0) by clamping
                let v1 = p1.value.max(0.0001);
                let v2 = p2.value.max(0.0001);
                v1 * (v2 / v1).powf(t)
            }
            CurveType::Step => {
                // Step: use p1's value until we reach p2
                p1.value
            }
        };

        Some(value)
    }

    /// Convert a normalized value (0.0–1.0) to the actual parameter value.
    pub fn denormalize(&self, normalized: f32) -> f32 {
        self.min + normalized * (self.max - self.min)
    }

    /// Convert an actual parameter value to normalized (0.0–1.0).
    pub fn normalize(&self, value: f32) -> f32 {
        if (self.max - self.min).abs() < f32::EPSILON {
            return 0.0;
        }
        ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    /// Get the denormalized value at a given time in ticks.
    pub fn denormalized_value_at(&self, time_ticks: u32) -> Option<f32> {
        self.value_at(time_ticks).map(|v| self.denormalize(v))
    }

    /// Clear all points.
    pub fn clear(&mut self) {
        self.points.clear();
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Linear interpolation
#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// ============================================================================
// AUTOMATION MANAGER (runtime — used by plugin wrappers during audio processing)
// ============================================================================

/// Runtime automation state for a single plugin instance.
///
/// This is used by `SynthWrapper` and `EffectWrapper` during audio rendering
/// to apply per-buffer automation values. It is NOT serialized with the project;
/// the authoritative data lives in `KarbeatTrack::automation_lanes`.
#[derive(Clone, Debug, Default)]
pub struct AutomationManager {
    /// Map of parameter ID → runtime automation lane
    pub lanes: HashMap<u32, AutomationLane>,
}

impl AutomationManager {
    pub fn new() -> Self {
        Self {
            lanes: HashMap::new(),
        }
    }

    /// Get or create a lane for a parameter.
    pub fn get_or_create_lane(&mut self, param_id: u32) -> &mut AutomationLane {
        self.lanes.entry(param_id).or_insert_with(|| {
            AutomationLane::new(
                AutomationId::from(0),
                AutomationTarget::TrackGeneratorPluginParam {
                    track_id: TrackId::from(0),
                    param_id,
                },
                format!("Param {}", param_id),
                0.0,
                1.0,
                0.5,
            )
        })
    }

    /// Get a lane by parameter ID.
    pub fn get_lane(&self, param_id: u32) -> Option<&AutomationLane> {
        self.lanes.get(&param_id)
    }

    /// Get mutable lane by parameter ID.
    pub fn get_lane_mut(&mut self, param_id: u32) -> Option<&mut AutomationLane> {
        self.lanes.get_mut(&param_id)
    }

    /// Apply all automation values at the given time.
    /// Returns a vec of (param_id, denormalized_value) pairs that should be applied.
    pub fn get_values_at(&self, time_ticks: u32) -> Vec<(u32, f32)> {
        self.lanes
            .iter()
            .filter_map(|(id, lane)| lane.denormalized_value_at(time_ticks).map(|v| (*id, v)))
            .collect()
    }

    /// Check if any lanes have automation data.
    pub fn has_automation(&self) -> bool {
        self.lanes.values().any(|lane| !lane.points.is_empty())
    }
}
