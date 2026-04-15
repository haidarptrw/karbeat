// src/plugin/wrapper.rs
//
// Generic plugin wrappers that add automation support to generators and effects.
// These wrappers implement KarbeatGenerator/KarbeatEffect traits while routing
// parameters through the automation system.

use std::any::Any;
use std::collections::HashMap;

use indexmap::IndexMap;
use karbeat_plugin_types::ParameterSpec;
use serde_json::Value;

use crate::effect_base::EffectBase;
use crate::traits::{KarbeatEffect, KarbeatGenerator, MidiEvent};

use super::effect_base::StandardEffectBase;
use super::synth_base::StandardSynthBase;

// ============================================================================
// RAW ENGINE TRAITS
// ============================================================================

/// Trait for raw synth engines (core synthesis logic only).
/// Implement this for your custom synthesizer, and wrap it with SynthWrapper
/// to get automation support and base parameter handling.
///
/// # Example
/// ```rust,ignore
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
    fn process(&mut self, base: &mut StandardSynthBase, output: &mut [f32], midi: &[MidiEvent]);

    /// Set a custom parameter (IDs beyond base range, typically >= 8)
    fn set_custom_parameter(&mut self, id: u32, value: f32);

    /// Get a custom parameter value
    fn get_custom_parameter(&self, id: u32) -> Option<f32>;

    /// Get default values for custom parameters
    fn custom_default_parameters() -> HashMap<u32, f32>
    where
        Self: Sized;
    /// Get definition of all custom parameters
    fn get_parameter_specs(&self) -> Vec<ParameterSpec>;

    /// Get the synth name
    fn name() -> &'static str
    where
        Self: Sized;

    /// OPTIONAL: Execute a custom GUI command. Returns an optional JSON string.
    fn execute_custom_command(&mut self, _command: &str, _payload: &Value) -> Option<Value> {
        None
    }

    fn apply_automation(&mut self, id: u32, value: f32);
    fn clear_automation(&mut self, id: u32);
}

/// Trait for raw effect engines (core DSP logic only).
/// Implement this for your custom effect, and wrap it with EffectWrapper
/// to get automation support and base parameter handling.
pub trait RawEffectEngine: Send + Sync {
    /// Prepare the engine (set sample rate, calculate coefficients)
    fn prepare(&mut self, sample_rate: f32, channels: usize, max_buffer_size: usize);

    /// Process audio in-place using the shared base state.
    fn process(&mut self, base: &mut crate::effect_base::StandardEffectBase, buffer: &mut [f32]);

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
    fn get_parameter_specs(&self) -> Vec<ParameterSpec>;

    /// Get the effect name
    fn name() -> &'static str
    where
        Self: Sized;

    /// OPTIONAL: Execute a custom GUI command. Returns an optional JSON Value.
    fn execute_custom_command(&mut self, _command: &str, _payload: &Value) -> Option<Value> {
        None
    }

    fn apply_automation(&mut self, id: u32, value: f32);
    fn clear_automation(&mut self, id: u32);
}

pub trait EffectEngine<B: EffectBase>: Send + Sync {
    fn name(&self) -> &str;
    fn prepare(&mut self, sample_rate: f32, channels: usize, max_buffer_size: usize);
    fn reset(&mut self);
    /// Engine receives the generic Base, allowing it to read custom state
    fn process(&mut self, base: &mut B, buffer: &mut [f32]);
    fn get_custom_parameter(&self, id: u32) -> Option<f32>;
    fn set_custom_parameter(&mut self, id: u32, value: f32);
    fn default_parameters(&self) -> HashMap<u32, f32>;
    fn get_parameter_specs(&self) -> Vec<ParameterSpec>;

    /// OPTIONAL: Execute a custom GUI command. Returns an optional JSON Value.
    fn execute_custom_command(&mut self, _command: &str, _payload: &Value) -> Option<Value> {
        None
    }
}

// ============================================================================
// SYNTH WRAPPER
// ============================================================================

/// Wrapper that adds automation and base parameters to any synth engine.
/// Implements `KarbeatGenerator` so it can be used directly in the audio engine.
#[derive(Clone)]
pub struct RawSynthWrapper<T: RawSynthEngine + Clone> {
    /// The custom synth engine (oscillators, wavetables, etc.)
    pub engine: T,
    /// Shared synth state (voices, filter, envelope, gain)
    pub base: StandardSynthBase,
}

impl<T: RawSynthEngine + Clone> RawSynthWrapper<T> {
    /// Create a new wrapped synth with default settings
    pub fn new(engine: T, sample_rate: f32, channels: usize) -> Self {
        Self {
            engine,
            base: StandardSynthBase::new(sample_rate, channels),
        }
    }

    pub fn get_all_parameters(&self) -> Vec<ParameterSpec> {
        let mut params = self.base.get_parameter_specs();
        params.extend(self.engine.get_parameter_specs());
        params
    }
}

impl<T: RawSynthEngine + Clone + Default> RawSynthWrapper<T> {
    pub fn build() -> Self {
        Self::new(T::default(), 48000.0, 2)
    }
}

impl<T: RawSynthEngine + Clone + 'static> KarbeatGenerator for RawSynthWrapper<T> {
    fn name(&self) -> &str {
        T::name()
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize,  max_buffer_size: usize) {
        self.base.prepare(sample_rate, channels, max_buffer_size);
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
        if !self.base.set_parameter(id, value) {
            self.engine.set_custom_parameter(id, value);
        }
    }

    fn get_parameter(&self, id: u32) -> f32 {
        self.base
            .get_parameter(id)
            .or_else(|| self.engine.get_custom_parameter(id))
            .unwrap_or(0.0)
    }

    fn default_parameters(&self) -> IndexMap<u32, f32> {
        let mut params = StandardSynthBase::default_parameters();
        params.extend(T::custom_default_parameters());
        params
    }

    fn get_parameter_specs(&self) -> Vec<ParameterSpec> {
        self.get_all_parameters()
    }

    fn execute_custom_command(&mut self, command: &str, payload: &Value) -> Option<Value> {
        self.engine.execute_custom_command(command, payload)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn apply_automation(&mut self, id: u32, value: f32) {
        self.base.apply_automation(id, value);
    }
    
    fn clear_automation(&mut self, id: u32) {
        self.base.clear_automation(id);
    }
}

// ============================================================================
// EFFECT WRAPPER
// ============================================================================

/// Wrapper that adds automation and base parameters to any effect engine.
/// Implements `KarbeatEffect` so it can be used directly in the audio engine.
#[derive(Clone)]
pub struct RawEffectWrapper<T: RawEffectEngine + Clone> {
    /// The custom effect engine (reverb, delay, etc.)
    pub engine: T,
    /// Shared effect state (bypass, mix)
    pub base: StandardEffectBase,
    /// Buffer for dry signal (for mix processing)
    dry_buffer: Vec<f32>,
}

impl<T: RawEffectEngine + Clone> RawEffectWrapper<T> {
    /// Create a new wrapped effect with default settings
    pub fn new(engine: T, sample_rate: f32, channels: usize) -> Self {
        Self {
            engine,
            base: StandardEffectBase::new(sample_rate, channels),
            dry_buffer: Vec::new(),
        }
    }


    pub fn get_all_parameters(&self) -> Vec<ParameterSpec> {
        let mut params = StandardEffectBase::get_parameter_specs();
        params.extend(self.engine.get_parameter_specs());
        params
    }
}

impl<T: RawEffectEngine + Clone + Default> RawEffectWrapper<T> {
    pub fn build() -> Self {
        Self::new(T::default(), 48000.0, 2)
    }
}

impl<T: RawEffectEngine + Clone + 'static> KarbeatEffect for RawEffectWrapper<T> {
    fn name(&self) -> &str {
        T::name()
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize, max_buffer_size: usize) {
        self.base.prepare(sample_rate, channels, max_buffer_size);
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
        if !self.base.set_parameter(id, value) {
            self.engine.set_custom_parameter(id, value);
        }
    }

    fn get_parameter(&self, id: u32) -> f32 {
        self.base
            .get_parameter(id)
            .or_else(|| self.engine.get_custom_parameter(id))
            .unwrap_or(0.0)
    }

    fn default_parameters(&self) -> IndexMap<u32, f32> {
        let mut params = StandardEffectBase::default_parameters();
        params.extend(T::custom_default_parameters());
        params
    }

    fn get_parameter_specs(&self) -> Vec<ParameterSpec> {
        self.get_all_parameters()
    }

    fn execute_custom_command(&mut self, command: &str, payload: &Value) -> Option<Value> {
        self.engine.execute_custom_command(command, payload)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn apply_automation(&mut self, id: u32, value: f32) {
        self.base.apply_automation(id, value);
    }
    
    fn clear_automation(&mut self, id: u32) {
        self.base.clear_automation(id);
    }
}

// ================= CUSTOMIZABLE EFFECT WRAPPER ====================

pub struct EffectWrapper<E, B>
where
    B: EffectBase,
    E: EffectEngine<B>,
{
    base: B,
    engine: E,
    dry_buffer: Vec<f32>,
}

impl<E, B> EffectWrapper<E, B>
where
    B: EffectBase,
    E: EffectEngine<B>,
{
    pub fn new(base: B, engine: E) -> Self {
        Self {
            base,
            engine,
            dry_buffer: Vec::new(),
        }
    }

    pub fn get_all_parameters(&self) -> Vec<ParameterSpec> {
        let mut params = B::get_parameter_specs();
        params.extend(self.engine.get_parameter_specs());
        params
    }
}

// Implement KarbeatEffect for the Wrapper
impl<E, B> KarbeatEffect for EffectWrapper<E, B>
where
    B: EffectBase + 'static + Send + Sync,
    E: EffectEngine<B> + 'static + Send + Sync,
{
    fn name(&self) -> &str {
        self.engine.name()
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize, max_buffer_size: usize) {
        // Now calling custom prepare logic!
        self.base.prepare(sample_rate, channels, max_buffer_size);
        self.engine.prepare(sample_rate, channels, max_buffer_size);

        if self.dry_buffer.len() < max_buffer_size * 2 {
            self.dry_buffer.resize(max_buffer_size * 2, 0.0);
        }
    }

    fn reset(&mut self) {
        self.base.reset();
        self.engine.reset();
    }

    fn process(&mut self, buffer: &mut [f32]) {
        // Use the trait's bypass check
        if self.base.is_bypass() {
            return;
        }

        let buf_len = buffer.len();
        if self.dry_buffer.len() < buf_len {
            self.dry_buffer.resize(buf_len, 0.0);
        }
        self.dry_buffer[..buf_len].copy_from_slice(buffer);

        // Pass the generic base to the engine
        self.engine.process(&mut self.base, buffer);

        // Use the trait's mix function
        self.base.apply_mix(&self.dry_buffer[..buf_len], buffer);
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        if !self.base.set_parameter(id, value) {
            self.engine.set_custom_parameter(id, value);
        }
    }

    fn get_parameter(&self, id: u32) -> f32 {
        self.base
            .get_parameter(id)
            .or_else(|| self.engine.get_custom_parameter(id))
            .unwrap_or(0.0)
    }

    fn default_parameters(&self) -> IndexMap<u32, f32> {
        let mut map = B::default_parameters();
        map.extend(self.engine.default_parameters());
        map
    }

    fn get_parameter_specs(&self) -> Vec<ParameterSpec> {
        let mut specs = B::get_parameter_specs();
        specs.extend(self.engine.get_parameter_specs());
        specs
    }

    fn execute_custom_command(&mut self, command: &str, payload: &Value) -> Option<Value> {
        self.engine.execute_custom_command(command, payload)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn apply_automation(&mut self, id: u32, value: f32) {
        self.base.apply_automation(id, value);
    }
    
    fn clear_automation(&mut self, id: u32) {
        self.base.clear_automation(id);
    }
}
