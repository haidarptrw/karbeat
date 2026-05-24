use std::sync::Arc;

use karbeat_macros::{auto_param, karbeat_plugin};
use karbeat_plugin_api::prelude::*;

/// Peak controller plugin. This measures the amplitude envelope of the input signal
/// and publishes it as a control value (0.0 to 1.0) for sidechaining or modulating other parameters.
#[derive(Clone)]
#[karbeat_plugin]
pub struct PeakController {
    #[param(id = "base", name = "Base", default = 0.0, min = -1.0, max = 1.0, step = 0.01)]
    pub base: f64,

    #[param(id = "volume", name = "Volume", default = 1.0, min = -2.0, max = 2.0, step = 0.01)]
    pub volume: f64,

    #[param(
        id = "attack",
        name = "Attack",
        default = 10.0,
        min = 1.0,
        max = 500.0,
        step = 1.0
    )]
    pub attack_ms: f64,

    #[param(
        id = "release",
        name = "Release",
        default = 100.0,
        min = 1.0,
        max = 2000.0,
        step = 1.0
    )]
    pub release_ms: f64,

    #[param(id = "mute", name = "Mute Output", default = false)]
    pub mute: bool,

    sample_rate: f64,
    envelope: f64,

    /// Shared memory buffer exposed to the routing engine/UI containing the live control value.
    pub control_out: Arc<Box<[f32]>>,
}

impl Default for PeakController {
    fn default() -> Self {
        let mut s = Self::base_default();
        s.sample_rate = 48000.0;
        s.envelope = 0.0;
        s.control_out = Arc::new(vec![0.0].into_boxed_slice());
        s
    }
}

#[auto_param]
impl AudioPlugin for PeakController {
    fn name(&self) -> &str {
        "Peak Controller"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Effect
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize, _max_buffer_size: usize) {
        self.sample_rate = sample_rate as f64;
        self.envelope = 0.0;
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
        self.control_out = Arc::new(vec![0.0].into_boxed_slice());
    }

    fn process(&mut self, buffer: &mut [f32], _context: &ProcessContext) {
        if self.sample_rate <= 0.0 {
            return;
        }

        let base = self.base.get();
        let vol = self.volume.get();
        let mute = self.mute.get();

        let att_coef = (-1.0 / (self.attack_ms.get() * 0.001 * self.sample_rate)).exp();
        let rel_coef = (-1.0 / (self.release_ms.get() * 0.001 * self.sample_rate)).exp();

        let mut block_peak = 0.0;

        for sample in buffer.iter_mut() {
            let input_abs = (*sample as f64).abs();

            // Apply attack/release ballistics
            if input_abs > self.envelope {
                self.envelope = att_coef * self.envelope + (1.0 - att_coef) * input_abs;
            } else {
                self.envelope = rel_coef * self.envelope + (1.0 - rel_coef) * input_abs;
            }

            // Track the maximum envelope value achieved in this specific block
            if self.envelope > block_peak {
                block_peak = self.envelope;
            }

            // A common workflow is to use a "Ghost Kick" to trigger the peak controller
            // but mute the actual audio output so the kick isn't heard in the master.
            if mute {
                *sample = 0.0;
            }

            let control_val = (base + (block_peak * vol)).clamp(0.0, 1.0) as f32;

            // Push the new value into the zero-copy buffer so the DAW routing engine can read it instantly
            self.control_out = Arc::new(vec![control_val].into_boxed_slice());
        }
    }

    // Expose the control buffer so the DAW/UI can query it
    fn get_zero_copy_buffer(&self, name: &str) -> Option<ZeroCopyBuffer> {
        if name == "control" {
            Some(ZeroCopyBuffer::Float32(self.control_out.clone()))
        } else {
            None
        }
    }
}
