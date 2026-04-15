use std::fmt::Debug;

use serde::{ Deserialize, Serialize };

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
    pub name: String,
    pub group: String, // e.g., "Oscillator 1", "Master"
    pub value: f32, // Current value
    pub min: f32,
    pub max: f32,
    pub default_value: f32,
    pub step: f32, // 0.0 for continuous
    pub value_type: ParameterValueType,
    pub choices: Vec<String>, // Labels for Choice type (index = value)
}

impl ParameterSpec {
    /// Create a new float parameter
    pub fn new_float(
        id: u32,
        name: &str,
        group: &str,
        val: f32,
        min: f32,
        max: f32,
        default: f32
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            group: group.to_string(),
            value: val,
            min,
            max,
            default_value: default,
            step: 0.0,
            value_type: ParameterValueType::Float,
            choices: Vec::new(),
        }
    }

    /// Create a new boolean parameter
    pub fn new_bool(id: u32, name: &str, group: &str, val: bool, default: bool) -> Self {
        Self {
            id,
            name: name.to_string(),
            group: group.to_string(),
            value: if val {
                1.0
            } else {
                0.0
            },
            min: 0.0,
            max: 1.0,
            default_value: if default {
                1.0
            } else {
                0.0
            },
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
        default: u32
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            group: group.to_string(),
            value: val as f32,
            min: 0.0,
            max: choices.len().saturating_sub(1) as f32,
            default_value: default as f32,
            step: 1.0,
            value_type: ParameterValueType::Choice,
            choices,
        }
    }
}

pub trait ParamType: Copy + Clone + Debug + PartialEq {
    fn from_f32_clamped(val: f32, bounds: &ParamBounds<Self>) -> Self;
    fn to_f32(self) -> f32;
}

impl ParamType for f32 {
    fn from_f32_clamped(val: f32, bounds: &ParamBounds<Self>) -> Self {
        match bounds {
            ParamBounds::Continuous { min, max } => val.clamp(*min, *max),
            _ => val,
        }
    }
    fn to_f32(self) -> f32 {
        self
    }
}

impl ParamType for i32 {
    fn from_f32_clamped(val: f32, bounds: &ParamBounds<Self>) -> Self {
        match bounds {
            ParamBounds::Discrete { min, max } =>
                val.round().clamp(*min as f32, *max as f32) as i32,
            _ => val.round() as i32,
        }
    }
    fn to_f32(self) -> f32 {
        self as f32
    }
}

impl ParamType for bool {
    fn from_f32_clamped(val: f32, _bounds: &ParamBounds<Self>) -> Self {
        val >= 0.5
    }
    fn to_f32(self) -> f32 {
        if self { 1.0 } else { 0.0 }
    }
}

// Enum/Choice Implementation (using usize)
impl ParamType for usize {
    fn from_f32_clamped(val: f32, bounds: &ParamBounds<Self>) -> Self {
        match bounds {
            ParamBounds::Choice { count, .. } => {
                let max_idx = count.saturating_sub(1) as f32;
                val.round().clamp(0.0, max_idx) as usize
            }
            _ => val.round() as usize,
        }
    }
    fn to_f32(self) -> f32 {
        self as f32
    }
}

/// A trait that allows an enum to be used safely as an automated parameter.
pub trait EnumParam: Copy + Clone + std::fmt::Debug + PartialEq {
    /// Convert the enum to a raw usize index
    fn to_index(self) -> usize;
    /// Safely convert a usize index back to the enum (falling back to a default if out of bounds)
    fn from_index(index: usize) -> Self;
    /// Provide string labels for the UI
    fn variants() -> &'static [&'static str];
}

impl<T: EnumParam> ParamType for T {
    fn from_f32_clamped(val: f32, _bounds: &ParamBounds<Self>) -> Self {
        // Clamp the float to the exact number of enum variants
        let max_idx = T::variants().len().saturating_sub(1) as f32;
        let idx = val.round().clamp(0.0, max_idx) as usize;
        T::from_index(idx)
    }

    fn to_f32(self) -> f32 {
        self.to_index() as f32
    }
}

/// Defines the constraints and behavior of a parameter.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ParamBounds<T> {
    Continuous {
        min: T,
        max: T,
    },
    Discrete {
        min: T,
        max: T,
    },
    Toggle,
    Choice {
        count: usize,
        labels: Vec<String>,
    },
}

/// A strictly typed, thread-safe parameter wrapper for DSP plugins.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Param<T: ParamType> {
    pub id: u32,
    pub name: String,
    pub group: String,

    /// The value set by the user via the UI (no automation applied)
    base_value: T,

    /// The actual value used by the DSP thread (base_value + automation)
    current_value: T,

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
    pub fn set_base(&mut self, raw_value: f32) {
        let clamped = T::from_f32_clamped(raw_value, &self.bounds);
        self.base_value = clamped;

        // If no automation is currently overriding it, update current_value immediately.
        // TODO: Add `is_automated` flag
        self.current_value = clamped;
    }

    /// Apply an automation frame from the sequencer.
    /// This modifies `current_value` but leaves `base_value` untouched.
    pub fn apply_automation(&mut self, automated_f32: f32) {
        self.current_value = T::from_f32_clamped(automated_f32, &self.bounds);
    }

    /// Clear automation and snap back to the user's base value.
    pub fn clear_automation(&mut self) {
        self.current_value = self.base_value;
    }

    pub fn to_spec(&self) -> ParameterSpec {
        ParameterSpec {
            id: self.id,
            name: self.name.to_string(),
            group: self.group.to_string(),
            value: self.get_base().to_f32(),
            min: match &self.bounds {
                ParamBounds::Continuous { min, .. } => min.to_f32(),
                ParamBounds::Discrete { min, .. } => min.to_f32(),
                ParamBounds::Toggle => 0.0,
                ParamBounds::Choice { .. } => 0.0,
            },
            max: match &self.bounds {
                ParamBounds::Continuous { max, .. } => max.to_f32(),
                ParamBounds::Discrete { max, .. } => max.to_f32(),
                ParamBounds::Toggle => 1.0,
                ParamBounds::Choice { count, .. } => count.saturating_sub(1) as f32,
            },
            default_value: self.base_value.to_f32(),
            step: match &self.bounds {
                ParamBounds::Continuous { .. } => 0.0,
                _ => 1.0,
            },
            value_type: match &self.bounds {
                ParamBounds::Continuous { .. } => ParameterValueType::Float,
                ParamBounds::Discrete { .. } => ParameterValueType::Int,
                ParamBounds::Toggle => ParameterValueType::Bool,
                ParamBounds::Choice { .. } => ParameterValueType::Choice,
            },
            choices: match &self.bounds {
                ParamBounds::Choice { labels, .. } =>
                    labels
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                _ => vec![],
            },
        }
    }
}

// In parameter.rs

impl Param<f32> {
    pub fn new_float(
        id: u32,
        name: &str,
        group: &str,
        default: f32,
        min: f32,
        max: f32
    ) -> Self {
        Self {
            id,
            name: name.to_owned(),
            group: group.to_owned(),
            base_value: default.clamp(min, max),
            current_value: default.clamp(min, max),
            bounds: ParamBounds::Continuous { min, max },
        }
    }
}

impl Param<bool> {
    pub fn new_bool(id: u32, name: &str, group: &str, default: bool) -> Self {
        Self {
            id,
            name: name.to_owned(),
            group: group.to_owned(),
            base_value: default,
            current_value: default,
            bounds: ParamBounds::Toggle,
        }
    }
}

impl Param<usize> {
    pub fn new_choice(
        id: u32,
        name: &str,
        group: &str,
        default: usize,
        labels: Vec<&str>
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            group: group.to_owned(),
            base_value: default,
            current_value: default,
            bounds: ParamBounds::Choice {
                count: labels.len(),
                labels: labels
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
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
            base_value: default,
            current_value: default,
            bounds: ParamBounds::Choice {
                count: T::variants().len(),
                labels: T::variants()
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
        }
    }
}

/// Traits that implements the automatic parameters getter, setter, specs, and automation
pub trait AutoParams {
    fn auto_get_parameter(&self, id: u32) -> Option<f32>;
    fn auto_set_parameter(&mut self, id: u32, value: f32);
    fn auto_apply_automation(&mut self, id: u32, value: f32);
    fn auto_clear_automation(&mut self, id: u32);
    fn auto_get_parameter_specs(&self) -> Vec<ParameterSpec>;
}
