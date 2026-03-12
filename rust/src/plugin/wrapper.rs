// src/plugin/wrapper.rs
//
// Generic plugin wrappers that add automation support to generators and effects.
// These wrappers implement KarbeatGenerator/KarbeatEffect traits while routing
// parameters through the automation system.

use std::any::Any;
use std::collections::HashMap;

use crate::core::project::plugin::{KarbeatEffect, KarbeatGenerator, MidiEvent};

use super::effect_base::EffectBase;
use super::synth_base::SynthBase;
use crate::core::project::automation::AutomationManager;

// ============================================================================
// PARAMETER API
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParameterValueType {
    Float,
    Int,
    Bool,
    Choice,
}

/// Generic description of a plugin parameter for UI generation
#[derive(Clone, Debug)]
pub struct PluginParameter {
    pub id: u32,
    pub name: String,
    pub group: String, // e.g., "Oscillator 1", "Master"
    pub value: f32,    // Current value
    pub min: f32,
    pub max: f32,
    pub default_value: f32,
    pub step: f32, // 0.0 for continuous
    pub value_type: ParameterValueType,
    pub choices: Vec<String>, // Labels for Choice type (index = value)
}

impl PluginParameter {
    /// Create a new float parameter
    pub fn new_float(
        id: u32,
        name: &str,
        group: &str,
        val: f32,
        min: f32,
        max: f32,
        default: f32,
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
            name: name.to_string(),
            group: group.to_string(),
            value: val as f32,
            min: 0.0,
            max: (choices.len().saturating_sub(1)) as f32,
            default_value: default as f32,
            step: 1.0,
            value_type: ParameterValueType::Choice,
            choices,
        }
    }
}

// ============================================================================
// RAW ENGINE TRAITS
// ============================================================================

/// Trait for raw synth engines (core synthesis logic only).
/// Implement this for your custom synthesizer, and wrap it with SynthWrapper
/// to get automation support and base parameter handling.
///
/// # Example
/// ```rust
/// struct MyOscillatorEngine {
///     waveform: Waveform,
///     detune: f32,
/// }
///
/// impl RawSynthEngine for MyOscillatorEngine {
///     fn process(&mut self, base: &mut SynthBase, output: &mut [f32], midi: &[MidiEvent]) {
///         // Use base.active_voices, base.amp_envelope, etc.
///         // Write audio to output buffer
///     }
///     // ...
/// }
///
/// // Usage:
/// type MySynth = SynthWrapper<MyOscillatorEngine>;
/// ```
pub trait RawSynthEngine: Send + Sync {
    /// Process audio using the shared base state.
    /// Write stereo interleaved audio to output buffer.
    fn process(&mut self, base: &mut SynthBase, output: &mut [f32], midi: &[MidiEvent]);

    /// Set a custom parameter (IDs beyond base range, typically >= 8)
    fn set_custom_parameter(&mut self, id: u32, value: f32);

    /// Get a custom parameter value
    fn get_custom_parameter(&self, id: u32) -> Option<f32>;

    /// Get default values for custom parameters
    fn custom_default_parameters() -> HashMap<u32, f32>
    where
        Self: Sized;
    /// Get definition of all custom parameters
    fn get_parameter_specs(&self) -> Vec<PluginParameter>;

    /// Get the synth name
    fn name() -> &'static str
    where
        Self: Sized;
}

/// Trait for raw effect engines (core DSP logic only).
/// Implement this for your custom effect, and wrap it with EffectWrapper
/// to get automation support and base parameter handling.
pub trait RawEffectEngine: Send + Sync {
    /// Process audio in-place using the shared base state.
    fn process(&mut self, base: &mut EffectBase, buffer: &mut [f32]);

    /// Reset internal effect state (delay lines, etc.)
    fn reset(&mut self);

    /// Set a custom parameter (IDs beyond base range, typically >= 2)
    fn set_custom_parameter(&mut self, id: u32, value: f32);

    /// Get a custom parameter value
    fn get_custom_parameter(&self, id: u32) -> Option<f32>;

    /// Get default values for custom parameters
    fn custom_default_parameters() -> HashMap<u32, f32>
    where
        Self: Sized;

    /// Get definition of all custom parameters
    fn get_parameter_specs(&self) -> Vec<PluginParameter>;

    /// Get the effect name
    fn name() -> &'static str
    where
        Self: Sized;
}

// ============================================================================
// SYNTH WRAPPER
// ============================================================================

/// Wrapper that adds automation and base parameters to any synth engine.
/// Implements `KarbeatGenerator` so it can be used directly in the audio engine.
#[derive(Clone)]
pub struct SynthWrapper<T: RawSynthEngine + Clone> {
    /// The custom synth engine (oscillators, wavetables, etc.)
    pub engine: T,
    /// Shared synth state (voices, filter, envelope, gain)
    pub base: SynthBase,
    /// Automation lanes for parameter modulation
    pub automation: AutomationManager,
}

impl<T: RawSynthEngine + Clone> SynthWrapper<T> {
    /// Create a new wrapped synth with default settings
    pub fn new(engine: T, sample_rate: f32) -> Self {
        Self {
            engine,
            base: SynthBase::new(sample_rate),
            automation: AutomationManager::new(),
        }
    }

    /// Apply automation values at the given time in ticks
    pub fn apply_automation(&mut self, time_ticks: u32) {
        for (param_id, value) in self.automation.get_values_at(time_ticks) {
            self.set_parameter_internal(param_id, value);
        }
    }

    /// Internal parameter setter (routes to base or engine)
    fn set_parameter_internal(&mut self, id: u32, value: f32) {
        if !self.base.set_parameter(id, value) {
            self.engine.set_custom_parameter(id, value);
        }
    }

    /// Public API to get ALL parameters (Base + Custom) for the UI
    pub fn get_all_parameters(&self) -> Vec<PluginParameter> {
        let mut params = self.base.get_parameter_specs();
        params.extend(self.engine.get_parameter_specs());
        params
    }
}

impl<T: RawSynthEngine + Clone + 'static> KarbeatGenerator for SynthWrapper<T> {
    fn name(&self) -> &str {
        T::name()
    }

    fn prepare(&mut self, sample_rate: f32, max_buffer_size: usize) {
        self.base.prepare(sample_rate, max_buffer_size);
    }

    fn reset(&mut self) {
        self.base.reset();
    }

    fn process(&mut self, output_buffer: &mut [f32], midi_events: &[MidiEvent]) {
        // Let the engine handle all processing including MIDI
        self.engine
            .process(&mut self.base, output_buffer, midi_events);
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        self.set_parameter_internal(id, value);
    }

    fn get_parameter(&self, id: u32) -> f32 {
        self.base
            .get_parameter(id)
            .or_else(|| self.engine.get_custom_parameter(id))
            .unwrap_or(0.0)
    }

    fn default_parameters(&self) -> HashMap<u32, f32> {
        let mut params = SynthBase::default_parameters();
        params.extend(T::custom_default_parameters());
        params
    }

    fn get_parameter_specs(&self) -> Vec<PluginParameter> {
        self.get_all_parameters()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// EFFECT WRAPPER
// ============================================================================

/// Wrapper that adds automation and base parameters to any effect engine.
/// Implements `KarbeatEffect` so it can be used directly in the audio engine.
#[derive(Clone)]
pub struct EffectWrapper<T: RawEffectEngine + Clone> {
    /// The custom effect engine (reverb, delay, etc.)
    pub engine: T,
    /// Shared effect state (bypass, mix)
    pub base: EffectBase,
    /// Automation lanes for parameter modulation
    pub automation: AutomationManager,
    /// Buffer for dry signal (for mix processing)
    dry_buffer: Vec<f32>,
}

impl<T: RawEffectEngine + Clone> EffectWrapper<T> {
    /// Create a new wrapped effect with default settings
    pub fn new(engine: T, sample_rate: f32) -> Self {
        Self {
            engine,
            base: EffectBase::new(sample_rate),
            automation: AutomationManager::new(),
            dry_buffer: Vec::new(),
        }
    }

    /// Apply automation values at the given time in ticks
    pub fn apply_automation(&mut self, time_ticks: u32) {
        for (param_id, value) in self.automation.get_values_at(time_ticks) {
            self.set_parameter_internal(param_id, value);
        }
    }

    /// Internal parameter setter (routes to base or engine)
    fn set_parameter_internal(&mut self, id: u32, value: f32) {
        if !self.base.set_parameter(id, value) {
            self.engine.set_custom_parameter(id, value);
        }
    }

    pub fn get_all_parameters(&self) -> Vec<PluginParameter> {
        let mut params = self.base.get_parameter_specs();
        params.extend(self.engine.get_parameter_specs());
        params
    }
}

impl<T: RawEffectEngine + Clone + 'static> KarbeatEffect for EffectWrapper<T> {
    fn name(&self) -> &str {
        T::name()
    }

    fn prepare(&mut self, sample_rate: f32, max_buffer_size: usize) {
        self.base.prepare(sample_rate, max_buffer_size);
        // Pre-allocate dry buffer for mix
        if self.dry_buffer.len() < max_buffer_size * 2 {
            self.dry_buffer.resize(max_buffer_size * 2, 0.0);
        }
    }

    fn reset(&mut self) {
        self.base.reset();
        self.engine.reset();
    }

    fn process(&mut self, buffer: &mut [f32]) {
        if self.base.bypass {
            return; // Bypass: don't modify buffer
        }

        // Store dry signal for mix
        let buf_len = buffer.len();
        if self.dry_buffer.len() < buf_len {
            self.dry_buffer.resize(buf_len, 0.0);
        }
        self.dry_buffer[..buf_len].copy_from_slice(buffer);

        // Process through engine
        self.engine.process(&mut self.base, buffer);

        // Apply dry/wet mix
        self.base.apply_mix(&self.dry_buffer[..buf_len], buffer);
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        self.set_parameter_internal(id, value);
    }

    fn get_parameter(&self, id: u32) -> f32 {
        self.base
            .get_parameter(id)
            .or_else(|| self.engine.get_custom_parameter(id))
            .unwrap_or(0.0)
    }

    fn default_parameters(&self) -> HashMap<u32, f32> {
        let mut params = EffectBase::default_parameters();
        params.extend(T::custom_default_parameters());
        params
    }

    fn get_parameter_specs(&self) -> Vec<PluginParameter> {
        self.get_all_parameters()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
