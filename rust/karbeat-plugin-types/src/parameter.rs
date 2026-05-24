use std::fmt::Debug;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ParameterValueType {
    Float,
    Int,
    Bool,
    Choice,
}

/// Generic description of a parameter spec for UI generation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParameterSpec {
    pub id: u32,
    pub path: String,
    pub name: String,
    pub group: String, // e.g., "Oscillator 1", "Master"
    pub value: f64,    // Current value upgraded to f64
    pub min: f64,
    pub max: f64,
    pub default_value: f64,
    pub step: f64, // 0.0 for continuous
    pub value_type: ParameterValueType,
    pub choices: Vec<String>, // Labels for Choice type (index = value)
}

impl ParameterSpec {
    /// Create a new float parameter (now uses f64 internally but retains f32 backward compatibility in name if desired)
    pub fn new_float(
        id: u32,
        name: &str,
        group: &str,
        val: f64,
        min: f64,
        max: f64,
        default: f64,
        step: f64,
    ) -> Self {
        Self {
            id,
            path: String::new(),
            name: name.to_string(),
            group: group.to_string(),
            value: val,
            min,
            max,
            default_value: default,
            step,
            value_type: ParameterValueType::Float,
            choices: Vec::new(),
        }
    }

    /// Create a new boolean parameter
    pub fn new_bool(id: u32, name: &str, group: &str, val: bool, default: bool) -> Self {
        Self {
            id,
            path: String::new(),
            name: name.to_string(),
            group: group.to_string(),
            value: if val { 1.0 } else { 0.0 },
            min: 0.0,
            max: 1.0,
            default_value: if default { 1.0 } else { 0.0 },
            step: 1.0,
            value_type: ParameterValueType::Bool,
            choices: Vec::new(),
        }
    }

    /// Create a new choice parameter
    pub fn new_choice(
        id: u32,
        name: &str,
        group: &str,
        val: u32,
        choices: Vec<String>,
        default: u32,
    ) -> Self {
        Self {
            id,
            path: String::new(),
            name: name.to_string(),
            group: group.to_string(),
            value: val as f64,
            min: 0.0,
            max: choices.len().saturating_sub(1) as f64,
            default_value: default as f64,
            step: 1.0,
            value_type: ParameterValueType::Choice,
            choices,
        }
    }
}

pub trait ParamType: Copy + Clone + Debug + PartialEq + std::any::Any {
    // Upgraded internal clamped value to f64 for precision.
    fn from_f64_clamped(val: f64, bounds: &ParamBounds<Self>) -> Self;
    fn from_f64(val: f64) -> Self;
    fn to_f64(self) -> f64;
    fn clamp_value(self, bounds: &ParamBounds<Self>) -> Self;
}

impl ParamType for f32 {
    fn from_f64_clamped(val: f64, bounds: &ParamBounds<Self>) -> Self {
        match bounds {
            // Note: Since min/max are now type T (f32 in this impl), we cast val to f32.
            ParamBounds::Continuous { min, max, .. } => (val as f32).clamp(*min, *max),
            _ => val as f32,
        }
    }
    fn from_f64(val: f64) -> Self {
        val as f32
    }
    fn to_f64(self) -> f64 {
        self as f64
    }
    fn clamp_value(self, bounds: &ParamBounds<Self>) -> Self {
        match bounds {
            ParamBounds::Continuous { min, max, .. } => self.clamp(*min, *max),
            _ => self,
        }
    }
}

impl ParamType for f64 {
    fn from_f64_clamped(val: f64, bounds: &ParamBounds<Self>) -> Self {
        match bounds {
            ParamBounds::Continuous { min, max, .. } => val.clamp(*min, *max),
            _ => val,
        }
    }
    fn from_f64(val: f64) -> Self {
        val
    }
    fn to_f64(self) -> f64 {
        self
    }
    fn clamp_value(self, bounds: &ParamBounds<Self>) -> Self {
        match bounds {
            ParamBounds::Continuous { min, max, .. } => self.clamp(*min, *max),
            _ => self,
        }
    }
}

impl ParamType for i32 {
    fn from_f64_clamped(val: f64, bounds: &ParamBounds<Self>) -> Self {
        match bounds {
            ParamBounds::Discrete { min, max, .. } => {
                val.round().clamp(*min as f64, *max as f64) as i32
            }
            _ => val.round() as i32,
        }
    }
    fn from_f64(val: f64) -> Self {
        val.round() as i32
    }
    fn to_f64(self) -> f64 {
        self as f64
    }
    fn clamp_value(self, bounds: &ParamBounds<Self>) -> Self {
        match bounds {
            ParamBounds::Discrete { min, max, .. } => self.clamp(*min, *max),
            _ => self,
        }
    }
}

impl ParamType for bool {
    fn from_f64_clamped(val: f64, _bounds: &ParamBounds<Self>) -> Self {
        val >= 0.5
    }
    fn from_f64(val: f64) -> Self {
        val >= 0.5
    }
    fn to_f64(self) -> f64 {
        if self { 1.0 } else { 0.0 }
    }
    fn clamp_value(self, _bounds: &ParamBounds<Self>) -> Self {
        self
    }
}

// Enum/Choice Implementation (using usize)
impl ParamType for usize {
    fn from_f64_clamped(val: f64, bounds: &ParamBounds<Self>) -> Self {
        match bounds {
            ParamBounds::Choice { count, .. } => {
                let max_idx = count.saturating_sub(1) as f64;
                val.round().clamp(0.0, max_idx) as usize
            }
            _ => val.round() as usize,
        }
    }
    fn from_f64(val: f64) -> Self {
        val.round() as usize
    }
    fn to_f64(self) -> f64 {
        self as f64
    }
    fn clamp_value(self, bounds: &ParamBounds<Self>) -> Self {
        match bounds {
            ParamBounds::Choice { count, .. } => {
                let max_idx = count.saturating_sub(1);
                self.clamp(0, max_idx)
            }
            _ => self,
        }
    }
}

/// A trait that allows an enum to be used safely as an automated parameter.
pub trait EnumParam: Copy + Clone + std::fmt::Debug + PartialEq + std::any::Any {
    /// Convert the enum to a raw usize index
    fn to_index(self) -> usize;
    /// Safely convert a usize index back to the enum (falling back to a default if out of bounds)
    fn from_index(index: usize) -> Self;
    /// Provide string labels for the UI
    fn variants() -> &'static [&'static str];
}

impl<T: EnumParam> ParamType for T {
    fn from_f64_clamped(val: f64, _bounds: &ParamBounds<Self>) -> Self {
        // Clamp the float to the exact number of enum variants
        let max_idx = T::variants().len().saturating_sub(1) as f64;
        let idx = val.round().clamp(0.0, max_idx) as usize;
        T::from_index(idx)
    }

    fn from_f64(val: f64) -> Self {
        T::from_index(val.round() as usize)
    }

    fn to_f64(self) -> f64 {
        self.to_index() as f64
    }

    fn clamp_value(self, _bounds: &ParamBounds<Self>) -> Self {
        let max_idx = T::variants().len().saturating_sub(1);
        let idx = self.to_index().clamp(0, max_idx);
        T::from_index(idx)
    }
}

/// Defines the constraints and behavior of a parameter.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ParamBounds<T> {
    Continuous { min: T, max: T, step: T },
    Discrete { min: T, max: T, step: T },
    Toggle,
    Choice { count: usize, labels: Vec<String> },
}

/// A strictly typed, thread-safe parameter wrapper for DSP plugins.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Param<T: ParamType> {
    pub id: u32,
    pub name: String,
    pub group: String,

    default_value: T,

    /// The value set by the user via the UI (no automation applied)
    base_value: T,

    /// The actual value used by the DSP thread (base_value + automation)
    current_value: T,

    is_automated: bool,

    pub bounds: ParamBounds<T>,
}

impl<T: ParamType> Param<T> {
    /// Read the final, automation-applied value for DSP processing.
    #[inline(always)]
    pub fn get(&self) -> T {
        self.current_value
    }

    /// Read the baseline UI value.
    pub fn get_base(&self) -> T {
        self.base_value
    }

    /// Set the baseline value (e.g., when the user turns a knob in the UI).
    /// This automatically updates the `current_value` and clamps it to valid bounds.
    pub fn set_base(&mut self, value: T) {
        let clamped = value.clamp_value(&self.bounds);
        self.base_value = clamped;

        // If no automation is currently overriding it, update current_value immediately.
        if !self.is_automated {
            self.current_value = clamped;
        }
    }

    pub fn is_automated(&self) -> bool {
        self.is_automated
    }

    pub fn enable_automation(&mut self) {
        self.is_automated = true;
    }

    pub fn disable_automation(&mut self) {
        self.is_automated = false;
    }

    pub fn set_base_from_f64(&mut self, raw_value: f64) {
        let clamped = T::from_f64_clamped(raw_value, &self.bounds);
        self.base_value = clamped;

        // If no automation is currently overriding it, update current_value immediately.
        if !self.is_automated {
            self.current_value = clamped;
        }
    }

    /// Apply an automation frame from the sequencer.
    /// This modifies `current_value` but leaves `base_value` untouched.
    pub fn apply_automation(&mut self, automated_val: T) {
        self.current_value = automated_val.clamp_value(&self.bounds);
    }

    pub fn apply_automation_from_f64(&mut self, automated_val: f64) {
        self.current_value = T::from_f64_clamped(automated_val, &self.bounds);
    }

    /// Clear automation and snap back to the user's base value.
    pub fn clear_automation(&mut self) {
        self.current_value = self.base_value;
    }

    pub fn to_spec(&self) -> ParameterSpec {
        ParameterSpec {
            id: self.id,
            path: String::new(),
            name: self.name.to_string(),
            group: self.group.to_string(),
            value: self.get_base().to_f64(),
            min: match &self.bounds {
                ParamBounds::Continuous { min, .. } => min.to_f64(),
                ParamBounds::Discrete { min, .. } => min.to_f64(),
                ParamBounds::Toggle => 0.0,
                ParamBounds::Choice { .. } => 0.0,
            },
            max: match &self.bounds {
                ParamBounds::Continuous { max, .. } => max.to_f64(),
                ParamBounds::Discrete { max, .. } => max.to_f64(),
                ParamBounds::Toggle => 1.0,
                ParamBounds::Choice { count, .. } => count.saturating_sub(1) as f64,
            },
            default_value: self.base_value.to_f64(),
            step: match &self.bounds {
                ParamBounds::Continuous { step, .. } => step.to_f64(),
                ParamBounds::Discrete { step, .. } => step.to_f64(),
                _ => 1.0, // Toggle and Choice inherently step by 1
            },
            value_type: match &self.bounds {
                ParamBounds::Continuous { .. } => ParameterValueType::Float,
                ParamBounds::Discrete { .. } => ParameterValueType::Int,
                ParamBounds::Toggle => ParameterValueType::Bool,
                ParamBounds::Choice { .. } => ParameterValueType::Choice,
            },
            choices: match &self.bounds {
                ParamBounds::Choice { labels, .. } => {
                    labels.iter().map(|s| s.to_string()).collect()
                }
                _ => vec![],
            },
        }
    }
}

// In parameter.rs

impl Param<f32> {
    pub fn new_f32(
        id: u32,
        name: &str,
        group: &str,
        default: f32,
        min: f32,
        max: f32,
        step: f32,
    ) -> Self {
        Self {
            id,
            name: name.to_owned(),
            group: group.to_owned(),
            default_value: default.clamp(min, max),
            base_value: default.clamp(min, max),
            current_value: default.clamp(min, max),
            bounds: ParamBounds::Continuous { min, max, step },
            is_automated: false,
        }
    }
}

impl Param<f64> {
    pub fn new_f64(
        id: u32,
        name: &str,
        group: &str,
        default: f64,
        min: f64,
        max: f64,
        step: f64,
    ) -> Self {
        Self {
            id,
            name: name.to_owned(),
            group: group.to_owned(),
            default_value: default.clamp(min, max),
            base_value: default.clamp(min, max),
            current_value: default.clamp(min, max),
            bounds: ParamBounds::Continuous { min, max, step },
            is_automated: false,
        }
    }
}

impl Param<i32> {
    pub fn new_i32(
        id: u32,
        name: &str,
        group: &str,
        default: i32,
        min: i32,
        max: i32,
        step: i32,
    ) -> Self {
        Self {
            id,
            name: name.to_owned(),
            group: group.to_owned(),
            default_value: default.clamp(min, max),
            base_value: default.clamp(min, max),
            current_value: default.clamp(min, max),
            bounds: ParamBounds::Discrete { min, max, step },
            is_automated: false,
        }
    }
}

impl Param<bool> {
    pub fn new_bool(id: u32, name: &str, group: &str, default: bool) -> Self {
        Self {
            id,
            name: name.to_owned(),
            group: group.to_owned(),
            default_value: default,
            base_value: default,
            current_value: default,
            bounds: ParamBounds::Toggle,
            is_automated: false,
        }
    }
}

impl Param<usize> {
    pub fn new_choice(id: u32, name: &str, group: &str, default: usize, labels: Vec<&str>) -> Self {
        Self {
            id,
            name: name.to_string(),
            group: group.to_owned(),
            default_value: default,
            base_value: default,
            current_value: default,
            bounds: ParamBounds::Choice {
                count: labels.len(),
                labels: labels.iter().map(|s| s.to_string()).collect(),
            },
            is_automated: false,
        }
    }
}

impl<T: EnumParam> Param<T> {
    /// Create a strictly typed Enum parameter. Labels and counts are extracted automatically!
    pub fn new_enum(id: u32, name: &str, group: &str, default: T) -> Self {
        Self {
            id,
            name: name.to_owned(),
            group: group.to_owned(),
            default_value: default,
            base_value: default,
            current_value: default,
            bounds: ParamBounds::Choice {
                count: T::variants().len(),
                labels: T::variants().iter().map(|s| s.to_string()).collect(),
            },
            is_automated: false,
        }
    }
}

/// Traits that implements the automatic parameters getter, setter, specs, and automation
/// The interface uses generics with f64 fallbacks for optimal precision scaling.
pub trait AutoParams {
    fn auto_set_parameter<V: ParamType>(&mut self, prefix_hash: u32, id: u32, value: V) -> bool;
    fn auto_get_parameter<V: ParamType>(&self, prefix_hash: u32, id: u32) -> Option<V>;
    fn auto_apply_automation<V: ParamType>(&mut self, prefix_hash: u32, id: u32, value: V) -> bool;
    fn auto_clear_automation(&mut self, prefix_hash: u32, id: u32) -> bool;
    fn auto_get_parameter_specs(&self, prefix_hash: u32, prefix_str: &str) -> Vec<ParameterSpec>;
}
