//! karbeat-dsp/comppressor.rs
//!
//! DSP module for commonly used compressor
//! as a component or building block for
//! DSP implementation

use karbeat_macros::{karbeat_plugin, EnumParam};
use serde::{Deserialize, Serialize};

/// Standard compressor DSP Module with lookahead delay
#[derive(Clone)]
#[karbeat_plugin]
pub struct Compressor {
    #[param(
        id = "attack",
        name = "Attack",
        default = 5,
        min = 1,
        max = 1000,
        step = 1
    )]
    pub attack_ms: i32,
    #[param(
        id = "release",
        name = "Release",
        default = 5,
        min = 1,
        max = 1000,
        step = 1
    )]
    pub release_ms: i32,
    #[param(id = "ratio", name = "Ratio", default = 1.0, max = 24.0, step = 0.1)]
    pub ratio: f64,
    #[param(
        id = "threshold",
        name = "Threshold",
        default = 0.0,
        max = 24.0,
        step = 0.1
    )]
    pub threshold: f64, // In dB
    #[param(
        id = "delay",
        name = "Delay",
        default = 5,
        min = 0,
        max = 100,
        step = 1
    )]
    pub delay_ms: i32, // Lookahead delay
    #[param(id = "knee", name = "Knee", default = 0.0, max = 24.0, step = 0.1)]
    pub knee: f64, // In dB
    #[param(
        id = "makeup_gain",
        name = "MakeUp Gain",
        default = 0.0,
        max = 24.0,
        step = 0.1
    )]
    pub makeup_gain: f64, // In dB
    #[param(id = "wetness", name = "Wetness", default = 0.0, max = 1.0, step = 0.1)]
    pub wetness: f64, // 0.0 to 1.0

    sample_rate: f64,
    envelope_db: f64,       // Tracks the smoothed gain reduction
    delay_buffer: Vec<f64>, // Ring buffer for lookahead
    delay_index: usize,
}

impl Compressor {
    pub fn prepare(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.envelope_db = 0.0;
        self.update_delay_buffer();
    }

    /// Resizes the delay buffer gracefully if the user changes the delay time
    #[inline(always)]
    fn update_delay_buffer(&mut self) {
        let delay_samples =
            (self.delay_ms.get() as f64 * 0.001 * self.sample_rate).max(0.0) as usize;

        if self.delay_buffer.len() != delay_samples {
            // Using resize prevents completely dropping the backing allocation
            // if the buffer is shrinking, which is safer for the audio thread.
            self.delay_buffer.resize(delay_samples, 0.0);
            if self.delay_index >= delay_samples {
                self.delay_index = 0;
            }
        }
    }

    // WE use f32 because CPAL only accepts f32 as the highest audio buffer stream
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let input_f64 = input as f64;

        // Ensure lookahead buffer matches current automated delay parameter
        self.update_delay_buffer();

        let threshold = self.threshold.get();
        let knee = self.knee.get();
        let ratio = self.ratio.get().max(1.0);
        let attack_ms = self.attack_ms.get().max(1) as f64;
        let release_ms = self.release_ms.get().max(1) as f64;
        let makeup_gain = self.makeup_gain.get();
        let wetness = self.wetness.get();

        let input_abs = input_f64.abs();
        let input_db = if input_abs > 1e-6 {
            20.0 * input_abs.log10()
        } else {
            -120.0
        };

        let overshoot = if knee > 0.0
            && input_db > (threshold - knee / 2.0)
            && input_db < (threshold + knee / 2.0)
        {
            // Inside the soft knee region (quadratic interpolation)
            (input_db - threshold + knee / 2.0).powi(2) / (2.0 * knee)
        } else if input_db > threshold {
            // Above the knee
            input_db - threshold
        } else {
            // Below the threshold
            0.0
        };

        let gain_reduction_target_db = overshoot * (1.0 - 1.0 / ratio);

        let attack_coef = (-1.0 / (attack_ms * 0.001 * self.sample_rate)).exp();
        let release_coef = (-1.0 / (release_ms * 0.001 * self.sample_rate)).exp();

        if gain_reduction_target_db > self.envelope_db {
            // Attack phase (compressing more)
            self.envelope_db =
                attack_coef * self.envelope_db + (1.0 - attack_coef) * gain_reduction_target_db;
        } else {
            // Release phase (compressing less)
            self.envelope_db =
                release_coef * self.envelope_db + (1.0 - release_coef) * gain_reduction_target_db;
        }

        let delayed_sample = if self.delay_buffer.is_empty() {
            input_f64
        } else {
            let out = self.delay_buffer[self.delay_index];
            self.delay_buffer[self.delay_index] = input_f64;
            self.delay_index = (self.delay_index + 1) % self.delay_buffer.len();
            out
        };

        let linear_gain = 10.0_f64.powf((-self.envelope_db + makeup_gain) / 20.0);
        let wet_signal = delayed_sample * linear_gain;

        let out = (1.0 - wetness) * delayed_sample + wetness * wet_signal;

        out as f32
    }
}

/// Sidechain compressor DSP Module.
/// Listens to an auxiliary sidechain input to duck the main input.
#[derive(Clone)]
#[karbeat_plugin]
pub struct SidechainCompressor {
    #[param(
        id = "attack",
        name = "Attack",
        default = 5,
        min = 1,
        max = 1000,
        step = 1
    )]
    pub attack_ms: i32,
    #[param(
        id = "release",
        name = "Release",
        default = 5,
        min = 1,
        max = 1000,
        step = 1
    )]
    pub release_ms: i32,
    #[param(
        id = "ratio",
        name = "Ratio",
        default = 4.0,
        min = 1.0,
        max = 24.0,
        step = 0.1
    )]
    pub ratio: f64,
    #[param(id = "threshold", name = "Threshold", default = 0.0, min = -60.0, max = 0.0, step = 0.1)]
    pub threshold: f64, // In dB
    #[param(
        id = "delay",
        name = "Delay",
        default = 0,
        min = 0,
        max = 100,
        step = 1
    )]
    pub delay_ms: i32, // Lookahead delay applied to the MAIN input
    #[param(
        id = "knee",
        name = "Knee",
        default = 6.0,
        min = 0.0,
        max = 24.0,
        step = 0.1
    )]
    pub knee: f64, // In dB
    #[param(
        id = "makeup_gain",
        name = "MakeUp Gain",
        default = 0.0,
        min = -24.0,
        max = 24.0,
        step = 0.1,
    )]
    pub makeup_gain: f64, // In dB
    #[param(
        id = "wetness",
        name = "Wetness",
        default = 1.0,
        min = 0.0,
        max = 1.0,
        step = 0.01
    )]
    pub wetness: f64, // 0.0 to 1.0

    sample_rate: f64,
    envelope_db: f64,       // Tracks the smoothed gain reduction
    delay_buffer: Vec<f64>, // Ring buffer for lookahead (stores main input)
    delay_index: usize,
}

impl Default for SidechainCompressor {
    fn default() -> Self {
        Self::base_default()
    }
}

impl SidechainCompressor {
    pub fn prepare(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.envelope_db = 0.0;
        self.update_delay_buffer();
    }

    /// Resizes the delay buffer gracefully if the user changes the delay time
    #[inline(always)]
    fn update_delay_buffer(&mut self) {
        let delay_samples =
            (self.delay_ms.get() as f64 * 0.001 * self.sample_rate).max(0.0) as usize;

        if self.delay_buffer.len() != delay_samples {
            self.delay_buffer.resize(delay_samples, 0.0);
            if self.delay_index >= delay_samples {
                self.delay_index = 0;
            }
        }
    }

    /// Processes a single sample using an external sidechain signal to drive the envelope.
    pub fn process_sample(&mut self, main_input: f32, sidechain_input: f32) -> f32 {
        let main_f64 = main_input as f64;
        let sc_f64 = sidechain_input as f64;

        // Ensure lookahead buffer matches current automated delay parameter
        self.update_delay_buffer();

        let threshold = self.threshold.get();
        let knee = self.knee.get();
        let ratio = self.ratio.get().max(1.0);
        let attack_ms = self.attack_ms.get().max(1) as f64;
        let release_ms = self.release_ms.get().max(1) as f64;
        let makeup_gain = self.makeup_gain.get();
        let wetness = self.wetness.get();

        // Level Detection (Peak) -> DRIVEN BY SIDECHAIN INPUT
        let sc_abs = sc_f64.abs();
        let sc_db = if sc_abs > 1e-6 {
            20.0 * sc_abs.log10()
        } else {
            -120.0
        };

        // Gain Computer (Soft Knee)
        let overshoot =
            if knee > 0.0 && sc_db > (threshold - knee / 2.0) && sc_db < (threshold + knee / 2.0) {
                // Inside the soft knee region (quadratic interpolation)
                (sc_db - threshold + knee / 2.0).powi(2) / (2.0 * knee)
            } else if sc_db > threshold {
                // Above the knee
                sc_db - threshold
            } else {
                // Below the threshold
                0.0
            };

        let gain_reduction_target_db = overshoot * (1.0 - 1.0 / ratio);

        // Envelope Smoothing (Ballistics)
        let attack_coef = (-1.0 / (attack_ms * 0.001 * self.sample_rate)).exp();
        let release_coef = (-1.0 / (release_ms * 0.001 * self.sample_rate)).exp();

        if gain_reduction_target_db > self.envelope_db {
            // Attack phase (compressing more)
            self.envelope_db =
                attack_coef * self.envelope_db + (1.0 - attack_coef) * gain_reduction_target_db;
        } else {
            // Release phase (compressing less)
            self.envelope_db =
                release_coef * self.envelope_db + (1.0 - release_coef) * gain_reduction_target_db;
        }

        // Lookahead Delay Line -> APPLIED TO MAIN INPUT
        let delayed_main_sample = if self.delay_buffer.is_empty() {
            main_f64
        } else {
            let out = self.delay_buffer[self.delay_index];
            self.delay_buffer[self.delay_index] = main_f64;
            self.delay_index = (self.delay_index + 1) % self.delay_buffer.len();
            out
        };

        // Apply Gain Reduction and Makeup Gain
        let linear_gain = 10.0_f64.powf((-self.envelope_db + makeup_gain) / 20.0);
        let wet_signal = delayed_main_sample * linear_gain;

        // Dry/Wet Mix
        let out = (1.0 - wetness) * delayed_main_sample + wetness * wet_signal;

        out as f32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default, EnumParam, Deserialize, Serialize)]
pub enum DynamicsMode {
    #[default]
    Compressor = 0,
    DownwardExpander = 1,
    NoiseGate = 2,
}

/// Unified Dynamics DSP Module with lookahead delay
#[derive(Clone)]
#[karbeat_plugin]
pub struct DynamicsProcessor {
    #[param(id = "mode", name = "Mode", default = 0.0)]
    pub mode: DynamicsMode,
    #[param(
        id = "attack",
        name = "Attack",
        default = 5,
        min = 1,
        max = 1000,
        step = 1
    )]
    pub attack_ms: i32,
    #[param(
        id = "release",
        name = "Release",
        default = 100,
        min = 1,
        max = 2000,
        step = 1
    )]
    pub release_ms: i32,
    #[param(
        id = "ratio",
        name = "Ratio",
        default = 4.0,
        min = 1.0,
        max = 100.0,
        step = 0.1
    )]
    pub ratio: f64,
    #[param(id = "threshold", name = "Threshold", default = 0.0, min = -80.0, max = 24.0, step = 0.1)]
    pub threshold: f64, // In dB
    #[param(
        id = "delay",
        name = "Delay",
        default = 0,
        min = 0,
        max = 100,
        step = 1
    )]
    pub delay_ms: i32, // Lookahead delay applied to the MAIN input
    #[param(
        id = "knee",
        name = "Knee",
        default = 6.0,
        min = 0.0,
        max = 24.0,
        step = 0.1
    )]
    pub knee: f64, // In dB
    #[param(id = "makeup_gain", name = "MakeUp Gain", default = 0.0, min = -24.0, max = 24.0, step = 0.1)]
    pub makeup_gain: f64, // In dB
    #[param(
        id = "wetness",
        name = "Wetness",
        default = 1.0,
        min = 0.0,
        max = 1.0,
        step = 0.01
    )]
    pub wetness: f64, // 0.0 to 1.0

    sample_rate: f64,
    envelope_db: f64, // Tracks the smoothed gain reduction (positive value = reduction)
    delay_buffer: Vec<f64>, // Ring buffer for lookahead (stores main input)
    delay_index: usize,
}

impl DynamicsProcessor {
    pub fn prepare(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.envelope_db = 0.0;
        self.update_delay_buffer();
    }

    /// Resizes the delay buffer gracefully if the user changes the delay time
    #[inline(always)]
    fn update_delay_buffer(&mut self) {
        let delay_samples =
            (self.delay_ms.get() as f64 * 0.001 * self.sample_rate).max(0.0) as usize;

        if self.delay_buffer.len() != delay_samples {
            self.delay_buffer.resize(delay_samples, 0.0);
            if self.delay_index >= delay_samples {
                self.delay_index = 0;
            }
        }
    }

    /// Processes a single sample.
    /// - For standard dynamics: pass the same audio to `main_input` and `detector_input`.
    /// - For sidechain dynamics: pass the sidechain signal to `detector_input`.
    pub fn process_sample(&mut self, main_input: f32, detector_input: f32) -> f32 {
        let main_f64 = main_input as f64;
        let det_f64 = detector_input as f64;

        self.update_delay_buffer();

        let threshold = self.threshold.get();
        let knee = self.knee.get();
        let ratio = self.ratio.get().max(1.0);
        let attack_ms = self.attack_ms.get().max(1) as f64;
        let release_ms = self.release_ms.get().max(1) as f64;
        let makeup_gain = self.makeup_gain.get();
        let wetness = self.wetness.get();
        let mode = self.mode.get();

        // 1. Level Detection (Peak)
        let det_abs = det_f64.abs();
        let det_db = if det_abs > 1e-6 {
            20.0 * det_abs.log10()
        } else {
            -120.0
        };

        // 2. Gain Computer
        let lower_knee = threshold - knee / 2.0;
        let upper_knee = threshold + knee / 2.0;

        let gain_reduction_target_db = match mode {
            DynamicsMode::Compressor => {
                let overshoot = if knee > 0.0 && det_db > lower_knee && det_db < upper_knee {
                    (det_db - lower_knee).powi(2) / (2.0 * knee)
                } else if det_db > threshold {
                    det_db - threshold
                } else {
                    0.0
                };
                overshoot * (1.0 - 1.0 / ratio)
            }
            DynamicsMode::DownwardExpander => {
                let undershoot = if knee > 0.0 && det_db < upper_knee && det_db > lower_knee {
                    (upper_knee - det_db).powi(2) / (2.0 * knee)
                } else if det_db < threshold {
                    threshold - det_db
                } else {
                    0.0
                };
                // Example: Ratio 2.0 means for every 1dB below threshold, we push it down an EXTRA 1dB.
                undershoot * (ratio - 1.0)
            }
            DynamicsMode::NoiseGate => {
                let undershoot = if knee > 0.0 && det_db < upper_knee && det_db > lower_knee {
                    (upper_knee - det_db).powi(2) / (2.0 * knee)
                } else if det_db < threshold {
                    threshold - det_db
                } else {
                    0.0
                };
                // A gate is basically an expander with a massive ratio (e.g., 100:1)
                undershoot * 100.0
            }
        };

        // 3. Envelope Smoothing (Ballistics)
        let attack_coef = (-1.0 / (attack_ms * 0.001 * self.sample_rate)).exp();
        let release_coef = (-1.0 / (release_ms * 0.001 * self.sample_rate)).exp();

        if gain_reduction_target_db > self.envelope_db {
            // Attack phase: The envelope is catching up to a higher reduction target
            self.envelope_db =
                attack_coef * self.envelope_db + (1.0 - attack_coef) * gain_reduction_target_db;
        } else {
            // Release phase: The envelope is dropping back down to a lower reduction target
            self.envelope_db =
                release_coef * self.envelope_db + (1.0 - release_coef) * gain_reduction_target_db;
        }

        // 4. Lookahead Delay Line (Applied ONLY to the main signal)
        let delayed_main_sample = if self.delay_buffer.is_empty() {
            main_f64
        } else {
            let out = self.delay_buffer[self.delay_index];
            self.delay_buffer[self.delay_index] = main_f64;
            self.delay_index = (self.delay_index + 1) % self.delay_buffer.len();
            out
        };

        // 5. Apply Gain Reduction and Makeup Gain
        // envelope_db is a positive number representing attenuation, so we subtract it
        let linear_gain = 10.0_f64.powf((-self.envelope_db + makeup_gain) / 20.0);
        let wet_signal = delayed_main_sample * linear_gain;

        // 6. Dry/Wet Mix
        let out = (1.0 - wetness) * delayed_main_sample + wetness * wet_signal;

        out as f32
    }
}
