use hashbrown::HashMap;
use karbeat_dsp::filter::{
    BiquadCoefficients, BiquadFilterType, FilterMode, SingleBiquadFilterStage,
};
use karbeat_dsp::windowing::Windowing;
use karbeat_macros::{karbeat_plugin, EnumParam};
use karbeat_plugin_api::prelude::*;
use karbeat_plugin_types::*;
use num_complex::{Complex, Complex32};
use rustfft::{num_traits::Zero, Fft, FftPlanner};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use smallvec::{smallvec, SmallVec};
use std::{any::Any, sync::Arc};

const FFT_SIZE: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Default, EnumParam, Deserialize, Serialize)]
pub enum FilterSlope {
    #[default]
    Db12 = 0, // 1 stage
    Db24 = 1, // 2 stages
    Db36 = 2, // 3 stages
    Db48 = 3, // 4 stages
}

impl From<f32> for FilterSlope {
    fn from(v: f32) -> Self {
        match v.round() as i32 {
            0 => FilterSlope::Db12,
            1 => FilterSlope::Db24,
            2 => FilterSlope::Db36,
            3 => FilterSlope::Db48,
            _ => FilterSlope::Db12,
        }
    }
}

// --------------------------------------------------------------------------
// THE FILTER NODE
// --------------------------------------------------------------------------

#[derive(Clone)]
#[karbeat_plugin]
pub struct DigiParametricEQFilterNode {
    #[param(
        id = "freq",
        name = "Frequency",
        group = "Band",
        min = 20.0,
        max = 20000.0,
        default = 1000.0,
        step = 1.0
    )]
    pub freq: f32,

    #[param(
        id = "gain",
        name = "Gain",
        group = "Band",
        min = -24.0,
        max = 24.0,
        default = 0.0,
        step = 0.1
    )]
    pub gain: f32,

    #[param(
        id = "q",
        name = "Q",
        group = "Band",
        min = 0.1,
        max = 20.0,
        default = 0.707,
        step = 0.1
    )]
    pub q: f32,

    #[param(id = "active", name = "Active", group = "Band", default = 1.0)]
    pub active: bool,

    #[param(id = "type", name = "Type", group = "Band", default = 0.0)]
    pub filter_type: BiquadFilterType,

    #[param(id = "slope", name = "Slope", group = "Band", default = 0.0)]
    pub order: FilterSlope,

    last_computed_hash: u64,

    // Internal: cached biquad coefficients computed from current params
    coeff: BiquadCoefficients,

    // Internal: per-channel cascaded biquad state
    channels: Vec<SingleBiquadFilterStage>,
}

impl DigiParametricEQFilterNode {
    pub fn new(band_idx: usize, default_freq: f32) -> Self {
        let mut node = Self::base_default();

        let group_name = format!("Band {}", band_idx + 1);

        node.freq.set_base(default_freq);
        node.freq.group = group_name.clone();
        node.gain.group = group_name.clone();
        node.q.group = group_name.clone();
        node.active.group = group_name.clone();
        node.filter_type.group = group_name.clone();
        node.order.group = group_name;

        if band_idx == 0 {
            node.filter_type
                .set_base(BiquadFilterType::LowShelf as i32 as f32);
        } else if band_idx == 7 {
            node.filter_type
                .set_base(BiquadFilterType::HighShelf as i32 as f32);
        }

        // Initialize with 2 channels and 1 cascade stage (Db12 default)
        node.channels = vec![SingleBiquadFilterStage::new(1); 2];
        node.coeff = BiquadCoefficients::default();

        node.update_coefficients(48000.0);
        node
    }

    /// Expands or shrinks per-channel state to match the given channel count.
    /// Preserves the current cascade stage count.
    pub fn ensure_channels(&mut self, channels: usize) {
        if self.channels.len() != channels {
            let num_stages = (self.order.get() as usize) + 1;
            self.channels
                .resize_with(channels, || SingleBiquadFilterStage::new(num_stages));
        }
    }

    /// Recomputes biquad coefficients from current parameter values.
    /// Also resizes cascade stages on each channel if the order changed.
    pub fn update_coefficients(&mut self, sample_rate: f32) {
        if sample_rate <= 0.0 {
            return;
        }

        // Create a simple hash or checksum of the current parameters
        // to avoid recalculating if nothing actually changed.
        let current_state = (
            self.freq.get().to_bits(),
            self.gain.get().to_bits(),
            self.q.get().to_bits(),
            self.active.get() as u8,
            self.filter_type.get() as usize,
            self.order.get() as usize,
            sample_rate.to_bits(),
        );

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&current_state, &mut hasher);
        let current_hash = std::hash::Hasher::finish(&hasher);

        if self.last_computed_hash == current_hash {
            return; // Skip expensive math, nothing changed!
        }
        self.last_computed_hash = current_hash;

        let num_stages = (self.order.get() as usize) + 1;
        for ch in &mut self.channels {
            ch.resize_cascades(num_stages);
        }

        self.coeff = self.filter_type.get().get_coefficients(
            self.freq.get(),
            self.q.get(),
            self.gain.get(),
            sample_rate,
        );
    }

    /// Processes a single sample for a given channel through all cascaded stages.
    /// Returns the input unchanged if the band is inactive or its type is Off.
    pub fn process_sample(&mut self, sample: f32, channel: usize) -> f32 {
        if !self.active.get() || self.filter_type.get().is_off() {
            return sample;
        }

        match self.channels.get_mut(channel) {
            Some(ch) => ch.process(sample, &self.coeff),
            None => sample,
        }
    }

    /// Clears all delay-line history across every channel and cascade stage.
    pub fn reset_state(&mut self) {
        for ch in &mut self.channels {
            ch.reset_state();
        }
    }

    /// Returns the magnitude response in dB at the given frequency.
    /// Returns 0.0 if the band is inactive.
    pub fn magnitude_db_at(&self, freq: f32, sample_rate: f32) -> f32 {
        if !self.active.get() {
            return 0.0;
        }

        let num_stages = (self.order.get() as usize) + 1;
        self.coeff.magnitude_db_at(freq, sample_rate, num_stages)
    }
}

// --------------------------------------------------------------------------
// THE MAIN PLUGIN
// --------------------------------------------------------------------------

#[derive(Clone)]
#[karbeat_plugin]
pub struct DigiParametricEQ {
    #[nested(prefix = "band")]
    pub nodes: Vec<DigiParametricEQFilterNode>,

    #[param(
        id = "base_gain",
        name = "Base Gain",
        group = "Master",
        min = -60.0,
        max = 24.0,
        default = 0.0,
        step = 0.1
    )]
    pub base_gain: f32,

    last_sample_rate: f32,
    channels: usize,

    // FFT Analyzer State
    analyzer_buffer: SmallVec<[f32; FFT_SIZE]>,
    analyzer_idx: usize,
    spectrum_history: Vec<f32>,
    fft_instance: Option<Arc<dyn Fft<f32>>>,

    //////////////////////////////////////////
    // Shared Memory buffer
    //////////////////////////////////////////
    pub magnitude_buffer: Arc<Box<[f32]>>,
    pub spectrum_buffer: Arc<Box<[f32]>>,
}

impl Default for DigiParametricEQ {
    fn default() -> Self {
        Self::new()
    }
}

impl DigiParametricEQ {
    pub fn new() -> Self {
        let mut engine = Self::base_default();

        let default_freqs = [60.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];
        for (i, &f) in default_freqs.iter().enumerate() {
            engine.nodes.push(DigiParametricEQFilterNode::new(i, f));
        }

        engine.last_sample_rate = 48000.0;
        engine.channels = 2;

        let mut planner = FftPlanner::new();
        engine.fft_instance = Some(planner.plan_fft_forward(FFT_SIZE));
        engine.analyzer_buffer = smallvec![0.0; FFT_SIZE];
        engine.analyzer_idx = 0;
        engine.spectrum_history = Vec::new();
        engine.magnitude_buffer = Arc::new(Box::new([]));
        engine.spectrum_buffer = Arc::new(Box::new([]));
        engine
    }

    fn update_all_nodes(&mut self) {
        for node in &mut self.nodes {
            node.ensure_channels(self.channels);
            node.update_coefficients(self.last_sample_rate);
        }
    }

    fn handle_side_effects(&mut self, _id: u32) {
        self.update_all_nodes();

        // Compute magnitude ONCE when parameters change, and save it to the shared buffer
        let response = self.compute_magnitude_response(500);
        let flat_array: Vec<f32> = response
            .into_iter()
            .flat_map(|(freq, db)| [freq, db])
            .collect();

        self.magnitude_buffer = Arc::new(flat_array.into_boxed_slice());
    }

    pub fn compute_magnitude_response(&self, num_points: usize) -> Vec<(f32, f32)> {
        let min_freq: f32 = 20.0;
        let max_freq: f32 = 20000.0;
        let log_min = min_freq.log10();
        let log_max = max_freq.log10();

        let mut result = Vec::with_capacity(num_points);

        for i in 0..num_points {
            let t = (i as f32) / ((num_points - 1).max(1) as f32);
            let freq = (10.0_f32).powf(log_min + t * (log_max - log_min));

            let mut total_db: f32 = self.base_gain.get();
            for node in &self.nodes {
                total_db += node.magnitude_db_at(freq, self.last_sample_rate);
            }

            result.push((freq, total_db));
        }

        result
    }
}

impl AudioPlugin for DigiParametricEQ {
    fn name(&self) -> &str {
        "Parametric EQ"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize, _max_buffer_size: usize) {
        let needs_update =
            (sample_rate - self.last_sample_rate).abs() > 0.1 || self.channels != channels;
        if needs_update {
            self.last_sample_rate = sample_rate;
            self.channels = channels;
            self.update_all_nodes();
        }
    }

    fn process(&mut self, buffer: &mut [f32], _context: &ProcessContext) {
        if self.channels == 0 {
            return;
        }

        let current_base_gain = self.base_gain.get();
        let master_linear_gain = if current_base_gain.abs() > 0.01 {
            (10.0_f32).powf(current_base_gain / 20.0)
        } else {
            1.0
        };

        for i in (0..buffer.len()).step_by(self.channels) {
            let mut mono_mix = 0.0;

            for channel in 0..self.channels {
                if i + channel < buffer.len() {
                    let mut sample = buffer[i + channel] * master_linear_gain;

                    for node in &mut self.nodes {
                        sample = node.process_sample(sample, channel);
                    }

                    buffer[i + channel] = sample;
                    mono_mix += sample;
                }
            }

            if self.analyzer_buffer.len() < FFT_SIZE {
                self.analyzer_buffer.resize(FFT_SIZE, 0.0);
            }

            self.analyzer_buffer[self.analyzer_idx] = mono_mix / (self.channels as f32);
            self.analyzer_idx = (self.analyzer_idx + 1) % FFT_SIZE;
        }
    }

    fn reset(&mut self) {
        for node in &mut self.nodes {
            node.reset_state();
        }
        self.analyzer_buffer.fill(0.0);
        self.spectrum_history.fill(-100.0);
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        if self.auto_set_parameter(karbeat_utils::hash::FNV_OFFSET, id, value) {
            self.handle_side_effects(id);
        }
    }

    fn get_parameter(&self, id: u32) -> f32 {
        self.auto_get_parameter(karbeat_utils::hash::FNV_OFFSET, id)
            .unwrap_or(0.0)
    }

    fn apply_automation(&mut self, id: u32, value: f32) {
        if self.auto_apply_automation(karbeat_utils::hash::FNV_OFFSET, id, value) {
            self.handle_side_effects(id);
        }
    }

    fn clear_automation(&mut self, id: u32) {
        if self.auto_clear_automation(karbeat_utils::hash::FNV_OFFSET, id) {
            self.handle_side_effects(id);
        }
    }

    fn default_parameters(&self) -> HashMap<u32, f32> {
        self.auto_get_parameter_specs(karbeat_utils::hash::FNV_OFFSET, "")
            .into_iter()
            .map(|spec| (spec.id, spec.default_value))
            .collect()
    }

    fn get_parameter_specs(&self) -> Vec<ParameterSpec> {
        self.auto_get_parameter_specs(karbeat_utils::hash::FNV_OFFSET, "")
    }

    fn static_parameter_specs() -> Vec<ParameterSpec>
    where
        Self: Sized,
    {
        Self::new().auto_get_parameter_specs(karbeat_utils::hash::FNV_OFFSET, "")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn execute_custom_command(&mut self, command: &str, payload: &Value) -> Option<Value> {
        match command {
            "GET_MAGNITUDE_RESPONSE" => {
                let num_points = payload
                    .get("num_points")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;

                let response = self.compute_magnitude_response(num_points);

                let flat_array: Vec<f32> = response
                    .into_iter()
                    .flat_map(|(freq, db)| [freq, db])
                    .collect();

                Some(json!(flat_array))
            }
            "GET_SPECTRUM" => {
                let num_points = payload
                    .get("num_points")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;

                let mut fft_input: SmallVec<[Complex32; FFT_SIZE]> =
                    smallvec![Complex::zero(); FFT_SIZE];
                for i in 0..FFT_SIZE {
                    // Read backwards from current idx to get sequential chronological data
                    let idx = (self.analyzer_idx + i) % FFT_SIZE;
                    let sample = self.analyzer_buffer[idx];

                    // Hann window to prevent spectral leakage
                    let window_func = Windowing::Hann;
                    let windowed_sample = window_func.apply_single_sample(sample, i, FFT_SIZE);

                    fft_input[i] = Complex::new(windowed_sample, 0.0);
                }

                if let Some(fft) = &self.fft_instance {
                    fft.process(&mut fft_input);
                }

                let mut raw_magnitudes = vec![0.0; FFT_SIZE / 2];
                let norm_factor = (FFT_SIZE as f32) / 2.0;

                for i in 0..FFT_SIZE / 2 {
                    let mag = fft_input[i].norm() / norm_factor;
                    let db = 20.0 * mag.log10();
                    raw_magnitudes[i] = db.clamp(-100.0, 24.0);
                }

                if self.spectrum_history.len() != num_points {
                    self.spectrum_history = vec![-100.0; num_points];
                }
                let min_freq: f32 = 20.0;
                let max_freq: f32 = 20000.0;
                let log_min = min_freq.log10();
                let log_max = max_freq.log10();

                // Pre-allocate the flat array (size = points * 2)
                let mut flat_array = Vec::with_capacity(num_points * 2);

                for i in 0..num_points {
                    let t = (i as f32) / ((num_points - 1).max(1) as f32);
                    let target_freq = (10.0_f32).powf(log_min + t * (log_max - log_min));

                    let bin_exact =
                        (target_freq * (FFT_SIZE as f32)) / self.last_sample_rate.max(1.0);

                    let mut current_db = -100.0;
                    if bin_exact >= 1.0 && (bin_exact as usize) < raw_magnitudes.len() - 1 {
                        let pool_radius = (bin_exact * 0.02).floor() as usize;
                        if pool_radius < 1 {
                            // LOW FREQUENCIES: Fractional Interpolation
                            // This turns the "flat staircase" into a beautiful smooth curve
                            let bin_floor = bin_exact.floor() as usize;
                            let bin_ceil = bin_exact.ceil() as usize;
                            let frac = bin_exact.fract();

                            let mag_floor = raw_magnitudes[bin_floor];
                            let mag_ceil = raw_magnitudes[bin_ceil];

                            // LERP the decibel values
                            current_db = mag_floor + (mag_ceil - mag_floor) * frac;
                        } else {
                            // HIGH FREQUENCIES: Max-Pooling
                            // Your existing logic to catch peaks that might slip between requested UI points
                            let bin_idx = bin_exact.round() as usize;
                            let start = bin_idx.saturating_sub(pool_radius);
                            let end = (bin_idx + pool_radius).min(raw_magnitudes.len() - 1);

                            for b in start..=end {
                                if raw_magnitudes[b] > current_db {
                                    current_db = raw_magnitudes[b];
                                }
                            }
                        }
                    }

                    // Apply Temporal Smoothing (Fast Attack, Slow Release)
                    let prev_db = self.spectrum_history[i];
                    let smoothed_db = if current_db > prev_db {
                        current_db * 0.8 + prev_db * 0.2
                    } else {
                        current_db * 0.1 + prev_db * 0.9
                    };
                    self.spectrum_history[i] = smoothed_db;

                    // Push directly into the flat array
                    flat_array.push(target_freq);
                    flat_array.push(smoothed_db);
                }

                self.spectrum_buffer = Arc::new(flat_array.clone().into_boxed_slice());

                Some(json!(flat_array))
                // None
            }
            _ => None,
        }
    }

    fn latency_samples(&self) -> u32 {
        0
    }

    fn tail_samples(&self) -> u32 {
        // IIR filters technically ring forever, but practically hit the noise floor
        // in less than a millisecond. We return 0, or a tiny arbitrary safety
        // buffer (e.g., 50 milliseconds) to let high-Q bands fully settle.

        // (0.05 seconds * sample_rate)
        (self.last_sample_rate * 0.05) as u32
    }

    fn get_zero_copy_buffer(&self, name: &str) -> Option<ZeroCopyBuffer> {
        match name {
            "magnitude" => {
                Some(ZeroCopyBuffer::Float32(self.magnitude_buffer.clone()))
            }
            "spectrum" => {
                Some(ZeroCopyBuffer::Float32(self.spectrum_buffer.clone()))
            }
            _ => None,
        }
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Effect
    }
}

impl Manifestable for DigiParametricEQ {
    fn build_manifest() -> PluginManifest {
        PluginManifest {
            id: 0, // Set to your global plugin ID
            name: "Digi Parametric EQ".into(),
            internal_type: "DigiParametricEQ".into(),
            is_synth: false,
            parameters: Self::static_parameter_specs(),
        }
    }
}

impl AudioPluginBuilder for DigiParametricEQ {
    fn build() -> Self {
        Self::new()
    }
}
