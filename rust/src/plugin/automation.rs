// src/plugin/automation.rs
//
// Automation system for parameter modulation over time.
// Works with both generators (SynthWrapper) and effects (EffectWrapper).

// ============================================================================
// CURVE TYPES
// ============================================================================

/// Interpolation curve type between automation points
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

/// A single point on an automation lane
#[derive(Clone, Debug)]
pub struct AutomationPoint {
    /// Position in beats (relative to clip/project start)
    pub time_beats: f64,
    /// Parameter value (typically 0.0-1.0 normalized or raw value)
    pub value: f32,
    /// Interpolation curve to the NEXT point
    pub curve_type: CurveType,
}

impl AutomationPoint {
    pub fn new(time_beats: f64, value: f32) -> Self {
        Self {
            time_beats,
            value,
            curve_type: CurveType::Linear,
        }
    }

    pub fn with_curve(time_beats: f64, value: f32, curve_type: CurveType) -> Self {
        Self {
            time_beats,
            value,
            curve_type,
        }
    }
}

// ============================================================================
// AUTOMATION LANE
// ============================================================================

/// An automation lane that controls a single parameter
#[derive(Clone, Debug)]
pub struct AutomationLane {
    /// The parameter ID this lane controls
    pub target_param_id: u32,
    /// Automation points sorted by time
    pub points: Vec<AutomationPoint>,
    /// Whether this lane is active
    pub enabled: bool,
}

impl AutomationLane {
    /// Create a new empty automation lane for a parameter
    pub fn new(param_id: u32) -> Self {
        Self {
            target_param_id: param_id,
            points: Vec::new(),
            enabled: true,
        }
    }

    /// Add a point to the lane (maintains sorted order)
    pub fn add_point(&mut self, point: AutomationPoint) {
        let idx = self
            .points
            .binary_search_by(|p| {
                p.time_beats
                    .partial_cmp(&point.time_beats)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or_else(|i| i);
        self.points.insert(idx, point);
    }

    /// Remove a point at the given index
    pub fn remove_point(&mut self, index: usize) -> Option<AutomationPoint> {
        if index < self.points.len() {
            Some(self.points.remove(index))
        } else {
            None
        }
    }

    /// Get the interpolated value at a given time in beats
    /// Returns None if the lane is disabled or has no points
    pub fn value_at(&self, time_beats: f64) -> Option<f32> {
        if !self.enabled || self.points.is_empty() {
            return None;
        }

        // Before first point: return first point's value
        if time_beats <= self.points[0].time_beats {
            return Some(self.points[0].value);
        }

        // After last point: return last point's value
        let last = self.points.last().unwrap();
        if time_beats >= last.time_beats {
            return Some(last.value);
        }

        // Find surrounding points using binary search
        let idx = self
            .points
            .binary_search_by(|p| {
                p.time_beats
                    .partial_cmp(&time_beats)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or_else(|i| i);

        // idx is where we'd insert, so points[idx-1] <= time < points[idx]
        if idx == 0 {
            return Some(self.points[0].value);
        }

        let p1 = &self.points[idx - 1];
        let p2 = &self.points[idx];

        // Calculate interpolation factor (0.0 to 1.0)
        let duration = p2.time_beats - p1.time_beats;
        if duration <= 0.0 {
            return Some(p1.value);
        }

        let t = ((time_beats - p1.time_beats) / duration) as f32;

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

    /// Clear all points
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
// AUTOMATION MANAGER (optional convenience struct)
// ============================================================================

use std::collections::HashMap;

/// Manages multiple automation lanes for a plugin
#[derive(Clone, Debug, Default)]
pub struct AutomationManager {
    /// Map of parameter ID to automation lane
    pub lanes: HashMap<u32, AutomationLane>,
}

impl AutomationManager {
    pub fn new() -> Self {
        Self {
            lanes: HashMap::new(),
        }
    }

    /// Get or create a lane for a parameter
    pub fn get_or_create_lane(&mut self, param_id: u32) -> &mut AutomationLane {
        self.lanes
            .entry(param_id)
            .or_insert_with(|| AutomationLane::new(param_id))
    }

    /// Get a lane by parameter ID
    pub fn get_lane(&self, param_id: u32) -> Option<&AutomationLane> {
        self.lanes.get(&param_id)
    }

    /// Get mutable lane by parameter ID
    pub fn get_lane_mut(&mut self, param_id: u32) -> Option<&mut AutomationLane> {
        self.lanes.get_mut(&param_id)
    }

    /// Apply all automation values at the given time
    /// Returns a vec of (param_id, value) pairs that should be applied
    pub fn get_values_at(&self, time_beats: f64) -> Vec<(u32, f32)> {
        self.lanes
            .iter()
            .filter_map(|(id, lane)| lane.value_at(time_beats).map(|v| (*id, v)))
            .collect()
    }

    /// Check if any lanes have automation data
    pub fn has_automation(&self) -> bool {
        self.lanes.values().any(|lane| !lane.points.is_empty())
    }
}
