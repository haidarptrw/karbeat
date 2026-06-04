// src/core/project/track/automation.rs
//
// Automation system for parameter modulation over time.
// Provides both project-level automation lane data (saved with the project)
// and a runtime AutomationManager used by plugin wrappers during audio processing.

use karbeat_dsp::interpolation::lerp;
use serde::{Deserialize, Serialize};

use crate::shared::{
    id::{AutomationId, BusId, EffectId, TrackId},
    types::FractionF32,
};

// ============================================================================
// AUTOMATION TARGET
// ============================================================================

/// Specifies what parameter an automation lane controls.
///
/// Each lane targets exactly one parameter on one thing (mixer channel,
/// generator, or effect slot).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AutomationTarget {
    Track {
        track_id: TrackId,
        track_target: TrackAutomationTarget,
    },

    Bus {
        bus_id: BusId,
        mix_target: MixerChannelParamTarget,
    },

    Master(MixerChannelParamTarget),

    // Global Targets
    TempoBpm,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TrackAutomationTarget {
    Generator { param_id: u32 },
    MixerChannel(MixerChannelParamTarget),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MixerChannelParamTarget {
    Volume,
    Pan,
    Plugin {
        effect_id: EffectId,
        target: EffectAutomationTarget,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EffectAutomationTarget {
    Mix, // NOTE: This is unused for a moment. might use it later
    PluginParam { param_id: u32 },
}

impl AutomationTarget {
    /// Returns true if this target references the given track ID.
    pub fn references_track(&self, id: TrackId) -> bool {
        match self {
            AutomationTarget::Track { track_id, .. } => *track_id == id,
            _ => false,
        }
    }

    /// Checks if this automation target references a specific Bus.
    pub fn references_bus(&self, target_bus_id: BusId) -> bool {
        match self {
            AutomationTarget::Bus { bus_id, .. } => *bus_id == target_bus_id,
            _ => false,
        }
    }
    /// Checks if two targets belong in the same UI accordion/drawer
    pub fn belongs_to_same_drawer_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Track { track_id: id1, .. }, Self::Track { track_id: id2, .. }) => id1 == id2,
            (Self::Bus { bus_id: id1, .. }, Self::Bus { bus_id: id2, .. }) => id1 == id2,
            (Self::Master(_), Self::Master(_)) => true,
            (Self::TempoBpm, Self::TempoBpm) => true,
            _ => false,
        }
    }
}
// ============================================================================
// CURVE TYPES
// ============================================================================

/// Interpolation curve type between automation points
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum AutomationCurveType {
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct AutomationPoint {
    /// Position in ticks (relative to project start)
    pub time_ticks: u32,
    /// Normalized parameter value (0.0–1.0)
    pub value: f32,
    /// Interpolation curve to the NEXT point
    pub curve_type: AutomationCurveType,

    pub tension: FractionF32,
}

impl AutomationPoint {
    pub fn new(time_ticks: u32, value: f32) -> Self {
        Self {
            time_ticks,
            value: value.clamp(0.0, 1.0),
            curve_type: AutomationCurveType::Linear,
            tension: 0.0.into(),
        }
    }

    pub fn with_curve(
        time_ticks: u32,
        value: f32,
        curve_type: AutomationCurveType,
        min: f32,
        max: f32,
    ) -> Self {
        Self {
            time_ticks,
            value: value.clamp(min, max),
            curve_type,
            tension: 0.0.into(),
        }
    }
}

// ============================================================================
// AUTOMATION LANE
// ============================================================================

/// An automation lane that controls a single parameter.
///
/// Lives in `ApplicationState::automation_pool` and is serialized with the project.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct AutomationLane {
    pub id: AutomationId,
    /// Human-readable label (e.g. "Volume", "Filter Cutoff")w
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
        label: impl Into<String>,
        min: f32,
        max: f32,
        default_value: f32,
    ) -> Self {
        Self {
            id,
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
    pub fn update_point(
        &mut self,
        index: usize,
        time_ticks: u32,
        value: f32,
        tension: f32,
    ) -> Option<usize> {
        if index >= self.points.len() {
            return None;
        }

        let mut point = self.points.remove(index);

        point.time_ticks = time_ticks;
        point.value = value;
        point.tension = tension.into();

        let new_index = match self
            .points
            .binary_search_by(|p| p.time_ticks.cmp(&point.time_ticks))
        {
            Ok(pos) => pos,
            Err(pos) => pos,
        };

        self.points.insert(new_index, point);
        Some(new_index)
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
        let tension = p1.tension.get();

        // Interpolate based on curve type of the FIRST point
        let value = match p1.curve_type {
            AutomationCurveType::Linear => {
                // Apply a cubic ease via tension-weighted Hermite tangents.
                // tension = 0  → linear (t)
                // tension < 0  → ease-in  (slow start, fast end)
                // tension > 0  → ease-out (fast start, slow end)
                let t_shaped = apply_tension_to_t(t, tension);
                lerp(t_shaped, p1.value, p2.value)
            }
            AutomationCurveType::Exponential => {
                // Bias the exponent so tension shifts the curve's inflection.
                // tension = 0  → standard (v1 * (v2/v1)^t)
                // tension < 0  → exponent biased toward a steeper early rise
                // tension > 0  → exponent biased toward a steeper late rise
                let v1 = p1.value.max(1e-4);
                let v2 = p2.value.max(1e-4);
                let t_biased = apply_tension_to_t(t, -tension); // inverted: feels natural
                v1 * (v2 / v1).powf(t_biased)
            }
            AutomationCurveType::Step => {
                let jump_at = 0.5 + tension * 0.5; // maps [-1,1] → [0,1]
                if t < jump_at {
                    p1.value
                } else {
                    p2.value
                }
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

// Maps a linear t ∈ [0,1] through a tension-controlled cubic ease.
///
/// Uses a smoothstep-family blend:
///   tension = 0  → identity (t)
///   tension < 0  → ease-in  (t² weighted)
///   tension > 0  → ease-out (√t weighted)
///
/// The blend is continuous and always passes through (0,0) and (1,1).
fn apply_tension_to_t(t: f32, tension: f32) -> f32 {
    if tension == 0.0 {
        return t;
    }
    // smoothstep gives ease-in-out; t^2 gives ease-in; sqrt(t) gives ease-out.
    // Blend from identity toward the appropriate end based on sign.
    if tension < 0.0 {
        // ease-in: blend toward t²
        let alpha = -tension; // 0..1
        lerp(alpha, t, t * t)
    } else {
        // ease-out: blend toward √t
        let alpha = tension; // 0..1
        lerp(alpha, t, t.sqrt())
    }
}
