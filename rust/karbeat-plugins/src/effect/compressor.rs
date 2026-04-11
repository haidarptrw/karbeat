use dasp::slice;
use std::f32::consts::E;

use karbeat_plugin_api::prelude::*;
use karbeat_plugin_types::*;

#[derive(Clone)]
pub struct KarbeatCompressor {
    /// ======================================
    /// User Parameters (automatable via Param<f32>)
    /// IDs start at 2 to avoid collision with base effect IDs (0=Bypass, 1=Mix)
    /// ======================================
    threshold: Param<f32>,
    ratio: Param<f32>,
    attack: Param<f32>,
    release: Param<f32>,
    makeup_gain: Param<f32>,

    /// ======================================
    /// Internal State (not user-facing)
    /// ======================================
    sample_rate: f32,
    current_gr_db: f32, // The current smoothed gain reduction
    attack_coeff: f32,
    release_coeff: f32,
}

impl Default for KarbeatCompressor {
    fn default() -> Self {
        let mut comp = Self {
            threshold: Param::new_float(2, "Threshold", "Compressor", -20.0, -60.0, 0.0),
            ratio: Param::new_float(3, "Ratio", "Compressor", 4.0, 1.0, 20.0),
            attack: Param::new_float(4, "Attack", "Compressor", 10.0, 0.1, 100.0),
            release: Param::new_float(5, "Release", "Compressor", 100.0, 1.0, 1000.0),
            makeup_gain: Param::new_float(6, "Makeup Gain", "Compressor", 0.0, -24.0, 24.0),

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
        let attack_sec = (self.attack.get() / 1000.0).max(0.001); // Prevent div by 0
        let release_sec = (self.release.get() / 1000.0).max(0.001);

        // Standard 1-pole smoothing coefficients
        self.attack_coeff = E.powf(-1.0 / (attack_sec * self.sample_rate));
        self.release_coeff = E.powf(-1.0 / (release_sec * self.sample_rate));
    }
}

// Implement whatever trait your wrapper requires (e.g., `KarbeatEffect`)
impl RawEffectEngine for KarbeatCompressor {
    fn prepare(&mut self, sample_rate: f32, _channels: usize, _buffer_size: usize) {
        self.sample_rate = sample_rate;
        self.recalculate_coefficients();
    }

    fn process(
        &mut self,
        _base: &mut karbeat_plugin_api::effect_base::StandardEffectBase,
        buffer: &mut [f32],
    ) {
        // Read param values once per buffer for performance
        let current_threshold = self.threshold.get();
        let current_ratio = self.ratio.get();
        let current_makeup = self.makeup_gain.get();

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
                if level_db > current_threshold {
                    let overshoot = level_db - current_threshold;
                    // Example: if overshoot is 20dB and ratio is 4:1.
                    // target_gr_db = 20 * (1/4 - 1) = 20 * (-0.75) = -15dB reduction.
                    target_gr_db = overshoot * (1.0 / current_ratio - 1.0);
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
                let total_gain_db = self.current_gr_db + current_makeup;
                let linear_gain = 10.0_f32.powf(total_gain_db / 20.0);

                // 5. Apply the gain to the audio signal
                frame[0] *= linear_gain;
                frame[1] *= linear_gain;
            }
        }
    }

    // --- Standard Wrapper Boilerplate ---

    fn set_custom_parameter(&mut self, param_id: u32, value: f32) {
        if self.threshold.id == param_id {
            self.threshold.set_base(value);
            return;
        }
        if self.ratio.id == param_id {
            self.ratio.set_base(value);
            return;
        }
        if self.attack.id == param_id {
            self.attack.set_base(value);
            self.recalculate_coefficients();
            return;
        }
        if self.release.id == param_id {
            self.release.set_base(value);
            self.recalculate_coefficients();
            return;
        }
        if self.makeup_gain.id == param_id {
            self.makeup_gain.set_base(value);
            return;
        }
    }

    fn get_custom_parameter(&self, param_id: u32) -> Option<f32> {
        if self.threshold.id == param_id {
            return Some(self.threshold.get_base().to_f32());
        }
        if self.ratio.id == param_id {
            return Some(self.ratio.get_base().to_f32());
        }
        if self.attack.id == param_id {
            return Some(self.attack.get_base().to_f32());
        }
        if self.release.id == param_id {
            return Some(self.release.get_base().to_f32());
        }
        if self.makeup_gain.id == param_id {
            return Some(self.makeup_gain.get_base().to_f32());
        }
        None
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
        map.insert(default.threshold.id, default.threshold.get_base().to_f32());
        map.insert(default.ratio.id, default.ratio.get_base().to_f32());
        map.insert(default.attack.id, default.attack.get_base().to_f32());
        map.insert(default.release.id, default.release.get_base().to_f32());
        map.insert(
            default.makeup_gain.id,
            default.makeup_gain.get_base().to_f32(),
        );
        map
    }

    fn get_parameter_specs(&self) -> Vec<PluginParameter> {
        vec![
            self.threshold.to_spec(),
            self.ratio.to_spec(),
            self.attack.to_spec(),
            self.release.to_spec(),
            self.makeup_gain.to_spec(),
        ]
    }

    fn name() -> &'static str
    where
        Self: Sized,
    {
        "Karbeat Compressor"
    }

    fn apply_automation(&mut self, id: u32, value: f32) {
        todo!()
    }

    fn clear_automation(&mut self, id: u32) {
        todo!()
    }
}

pub type KarbeatCompressorWrapper = RawEffectWrapper<KarbeatCompressor>;

pub fn create_compressor(sample_rate: Option<f32>) -> RawEffectWrapper<KarbeatCompressor> {
    RawEffectWrapper::new(
        KarbeatCompressor::default(),
        sample_rate.unwrap_or(48000.0),
        2,
    )
}
