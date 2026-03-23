use dasp::slice;
use std::f32::consts::E;

use karbeat_plugin_api::wrapper::{PluginParameter, RawEffectEngine, RawEffectWrapper};

// Assuming you have a standard parameter spec struct in your codebase
// use crate::core::project::plugin::ParameterSpec;

#[derive(Clone)]
pub struct KarbeatCompressor {
    // User Parameters
    threshold_db: f32,
    ratio: f32,
    attack_ms: f32,
    release_ms: f32,
    makeup_gain_db: f32,

    // Internal State
    sample_rate: f32,
    current_gr_db: f32, // The current smoothed gain reduction
    attack_coeff: f32,
    release_coeff: f32,
}

impl Default for KarbeatCompressor {
    fn default() -> Self {
        let mut comp = Self {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            makeup_gain_db: 0.0,

            sample_rate: 48000.0,
            current_gr_db: 0.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
        };
        comp.recalculate_coefficients();
        comp
    }
}

impl KarbeatCompressor {
    /// Recalculates the 1-pole filter coefficients when sample rate or time params change
    fn recalculate_coefficients(&mut self) {
        // Convert ms to seconds
        let attack_sec = (self.attack_ms / 1000.0).max(0.001); // Prevent div by 0
        let release_sec = (self.release_ms / 1000.0).max(0.001);

        // Standard 1-pole smoothing coefficients
        self.attack_coeff = E.powf(-1.0 / (attack_sec * self.sample_rate));
        self.release_coeff = E.powf(-1.0 / (release_sec * self.sample_rate));
    }
}

// Implement whatever trait your wrapper requires (e.g., `KarbeatEffect`)
impl RawEffectEngine for KarbeatCompressor {
    fn prepare(&mut self, sample_rate: f32, _buffer_size: usize) {
        self.sample_rate = sample_rate;
        self.recalculate_coefficients();
    }

    fn process(
        &mut self,
        _base: &mut karbeat_plugin_api::effect_base::StandardEffectBase,
        buffer: &mut [f32],
    ) {
        // We use dasp::slice to process stereo frames cleanly,
        // identical to your Phase 1/Phase 2 math in engine.rs
        if let Some(frames) = slice::from_sample_slice_mut::<&mut [[f32; 2]], f32>(buffer) {
            for frame in frames.iter_mut() {
                // 1. Stereo-linked Detector (use the loudest channel)
                let max_abs = frame[0].abs().max(frame[1].abs());

                // Convert linear amplitude to Decibels (cap at -120dB to prevent log(0) -Infinity)
                let level_db = if max_abs > 0.000001 {
                    20.0 * max_abs.log10()
                } else {
                    -120.0
                };

                // 2. Calculate target gain reduction
                let mut target_gr_db = 0.0;
                if level_db > self.threshold_db {
                    let overshoot = level_db - self.threshold_db;
                    // Example: if overshoot is 20dB and ratio is 4:1.
                    // target_gr_db = 20 * (1/4 - 1) = 20 * (-0.75) = -15dB reduction.
                    target_gr_db = overshoot * (1.0 / self.ratio - 1.0);
                }

                // 3. Smooth the gain reduction using Attack/Release envelope
                // Note: GR is a negative number. So target < current means we are compressing MORE (Attack).
                let coeff = if target_gr_db < self.current_gr_db {
                    self.attack_coeff
                } else {
                    self.release_coeff
                };

                // Apply 1-pole filter
                self.current_gr_db =
                    (target_gr_db - self.current_gr_db) * (1.0 - coeff) + self.current_gr_db;

                // 4. Convert GR dB and Makeup Gain dB back to a linear multiplier
                let total_gain_db = self.current_gr_db + self.makeup_gain_db;
                let linear_gain = 10.0_f32.powf(total_gain_db / 20.0);

                // 5. Apply the gain to the audio signal
                frame[0] *= linear_gain;
                frame[1] *= linear_gain;
            }
        }
    }

    // --- Standard Wrapper Boilerplate ---

    fn set_custom_parameter(&mut self, param_id: u32, value: f32) {
        match param_id {
            0 => self.threshold_db = value,
            1 => self.ratio = value,
            2 => {
                self.attack_ms = value;
                self.recalculate_coefficients();
            }
            3 => {
                self.release_ms = value;
                self.recalculate_coefficients();
            }
            4 => self.makeup_gain_db = value,
            _ => {}
        }
    }

    fn get_custom_parameter(&self, param_id: u32) -> Option<f32> {
        match param_id {
            0 => Some(self.threshold_db),
            1 => Some(self.ratio),
            2 => Some(self.attack_ms),
            3 => Some(self.release_ms),
            4 => Some(self.makeup_gain_db),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.current_gr_db = 0.0;
    }

    fn custom_default_parameters() -> std::collections::HashMap<u32, f32>
    where
        Self: Sized,
    {
        let mut map = std::collections::HashMap::new();
        let default = Self::default();
        map.insert(0, default.threshold_db);
        map.insert(1, default.ratio);
        map.insert(2, default.attack_ms);
        map.insert(3, default.release_ms);
        map.insert(4, default.makeup_gain_db);
        map
    }

    fn get_parameter_specs(&self) -> Vec<PluginParameter> {
        vec![
            PluginParameter::new_float(
                0,
                "Threshold",
                "Compressor",
                self.threshold_db,
                -60.0,
                0.0,
                -20.0,
            ),
            PluginParameter::new_float(1, "Ratio", "Compressor", self.ratio, 1.0, 20.0, 4.0),
            PluginParameter::new_float(2, "Attack", "Compressor", self.attack_ms, 0.1, 100.0, 10.0),
            PluginParameter::new_float(
                3,
                "Release",
                "Compressor",
                self.release_ms,
                1.0,
                1000.0,
                100.0,
            ),
            PluginParameter::new_float(
                4,
                "Makeup Gain",
                "Compressor",
                self.makeup_gain_db,
                -24.0,
                24.0,
                0.0,
            ),
        ]
    }

    fn name() -> &'static str
    where
        Self: Sized,
    {
        "Karbeat Compressor"
    }
}

pub type KarbeatCompressorWrapper = RawEffectWrapper<KarbeatCompressor>;

pub fn create_compressor(sample_rate: Option<f32>) -> RawEffectWrapper<KarbeatCompressor> {
    RawEffectWrapper::new(KarbeatCompressor::default(), sample_rate.unwrap_or(48000.0))
}
