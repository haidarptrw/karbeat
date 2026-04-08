// src/plugin/effect_base.rs
//
// Shared effect infrastructure for all effect plugins.
// Use composition pattern: embed EffectBase in your effect struct.
use indexmap::IndexMap;
use karbeat_plugin_types::PluginParameter;

pub trait EffectBase: Send + Sync + Clone {
    fn prepare(&mut self, sample_rate: f32, channels: usize, max_buffer_size: usize);
    fn reset(&mut self);
    fn is_bypass(&self) -> bool;
    fn apply_mix(&self, dry_buffer: &[f32], wet_buffer: &mut [f32]);

    /// Returns true if the parameter was handled by the base
    fn set_parameter(&mut self, id: u32, value: f32) -> bool;
    fn get_parameter(&self, id: u32) -> Option<f32>;
    fn channels(&self) -> usize;
    fn sample_rate(&self) -> f32;

    fn default_parameters() -> IndexMap<u32, f32>;
    fn get_parameter_specs() -> Vec<PluginParameter>;
}

// ============================================================================
// EFFECT BASE (Composition Pattern)
// ============================================================================

/// Common effect state for all effect plugins.
/// Embed this in your effect struct instead of duplicating fields.
///
/// # Example
/// ```rust,ignore
/// pub struct MyReverb {
///     pub base: EffectBase,
///     pub room_size: f32,  // Effect-specific
///     pub damping: f32,
/// }
/// ```
#[derive(Clone, Debug)]
pub struct StandardEffectBase {
    pub sample_rate: f32,
    pub bypass: bool,
    pub mix: f32, // Dry/wet mix (0.0 = fully dry, 1.0 = fully wet)
    pub channels: usize,
}

impl Default for StandardEffectBase {
    fn default() -> Self {
        Self::new(48000.0, 2)
    }
}

impl StandardEffectBase {
    /// Create a new EffectBase with the given sample rate
    pub fn new(sample_rate: f32, channels: usize) -> Self {
        Self {
            sample_rate,
            bypass: false,
            mix: 1.0,
            channels,
        }
    }

    /// Prepare for playback
    pub fn prepare(&mut self, sample_rate: f32, channels: usize, _max_buffer_size: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels;
    }

    /// Reset effect state
    pub fn reset(&mut self) {
        // Base has no state to reset, but effects can override
    }

    /// Apply dry/wet mix to processed buffer
    ///
    /// - `dry_buffer`: The original unprocessed audio
    /// - `wet_buffer`: The processed audio (modified in-place to become mixed output)
    pub fn apply_mix(&self, dry_buffer: &[f32], wet_buffer: &mut [f32]) {
        if self.mix >= 1.0 {
            return; // Fully wet, no mixing needed
        }
        if self.mix <= 0.0 {
            wet_buffer.copy_from_slice(dry_buffer);
            return;
        }

        let dry_gain = 1.0 - self.mix;
        let wet_gain = self.mix;

        for (wet, dry) in wet_buffer.iter_mut().zip(dry_buffer.iter()) {
            *wet = *dry * dry_gain + *wet * wet_gain;
        }
    }

    /// Handle base parameters (IDs 0-1). Returns true if parameter was handled.
    ///
    /// Parameter mapping:
    /// - 0: bypass (0.0 = off, 1.0 = on)
    /// - 1: mix (0.0-1.0, dry/wet)
    pub fn set_parameter(&mut self, id: u32, value: f32) -> bool {
        match id {
            0 => {
                self.bypass = value >= 0.5;
                true
            }
            1 => {
                self.mix = value.clamp(0.0, 1.0);
                true
            }
            _ => false,
        }
    }

    /// Get base parameters (IDs 0-1). Returns Some if parameter was handled.
    pub fn get_parameter(&self, id: u32) -> Option<f32> {
        match id {
            0 => Some(if self.bypass { 1.0 } else { 0.0 }),
            1 => Some(self.mix),
            _ => None,
        }
    }

    /// Get default parameter values for base parameters (IDs 0-1)
    pub fn default_parameters() -> IndexMap<u32, f32> {
        let mut map = IndexMap::new();
        map.insert(0, 0.0); // bypass off
        map.insert(1, 1.0); // fully wet
        map
    }

    pub fn get_parameter_specs() -> Vec<PluginParameter> {
        vec![
            PluginParameter::new_bool(0, "Bypass", "General", false, false),
            PluginParameter::new_float(1, "Mix", "General", 1.0, 0.0, 1.0, 1.0),
        ]
    }

    /// Base parameter IDs reserved by EffectBase (0-1)
    pub const BASE_PARAM_COUNT: u32 = 2;
}
