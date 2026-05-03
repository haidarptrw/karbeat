use hashbrown::HashMap;
use karbeat_dsp::windowing::Windowing;
use karbeat_macros::{karbeat_plugin, EnumParam};
use karbeat_plugin_api::prelude::*;
use karbeat_plugin_types::*;
use num_complex::{Complex, Complex32};
use rustfft::{
    num_traits::{Float, Zero},
    Fft, FftPlanner,
};
use serde_json::{json, Value};
use smallvec::{smallvec, SmallVec};
use std::{any::Any, sync::Arc};

/// Maximum number of cascaded biquad stages per band (order 0..3 = 1..4 stages)
const MAX_ORDER: usize = 8;
const FFT_SIZE: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Default, EnumParam)]
pub enum FilterType {
    #[default]
    Peaking = 0,
    LowShelf = 1,
    HighShelf = 2,
    LowPass = 3,
    HighPass = 4,
    BandPass = 5,
    Notch = 6,
}

impl From<f32> for FilterType {
    fn from(v: f32) -> Self {
        match v.round() as i32 {
            0 => FilterType::Peaking,
            1 => FilterType::LowShelf,
            2 => FilterType::HighShelf,
            3 => FilterType::LowPass,
            4 => FilterType::HighPass,
            5 => FilterType::BandPass,
            6 => FilterType::Notch,
            _ => FilterType::Peaking,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default, EnumParam)]
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
pub struct KarbeatParametricEQFilterNode {
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

    #[param(id = "gain", name = "Gain", group = "Band", min = -24.0, max = 24.0, default = 0.0, step = 0.1)]
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
    pub filter_type: FilterType,

    #[param(id = "slope", name = "Slope", group = "Band", default = 0.0)]
    pub order: FilterSlope,

    // Internal Runtime Coefficients
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,

    // Cascaded biquad state
    x1: Vec<[f32; MAX_ORDER]>,
    x2: Vec<[f32; MAX_ORDER]>,
    y1: Vec<[f32; MAX_ORDER]>,
    y2: Vec<[f32; MAX_ORDER]>,
}

impl KarbeatParametricEQFilterNode {
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
                .set_base(FilterType::LowShelf as i32 as f32);
        } else if band_idx == 7 {
            node.filter_type
                .set_base(FilterType::HighShelf as i32 as f32);
        }

        node.x1 = vec![[0.0; MAX_ORDER]; 2];
        node.x2 = vec![[0.0; MAX_ORDER]; 2];
        node.y1 = vec![[0.0; MAX_ORDER]; 2];
        node.y2 = vec![[0.0; MAX_ORDER]; 2];

        node.update_coefficients(48000.0);
        node
    }

    pub fn ensure_channels(&mut self, channels: usize) {
        if self.x1.len() != channels {
            self.x1.resize(channels, [0.0; MAX_ORDER]);
            self.x2.resize(channels, [0.0; MAX_ORDER]);
            self.y1.resize(channels, [0.0; MAX_ORDER]);
            self.y2.resize(channels, [0.0; MAX_ORDER]);
        }
    }

    pub fn update_coefficients(&mut self, sample_rate: f32) {
        if sample_rate <= 0.0 {
            return;
        }

        let freq = self.freq.get();
        let q = self.q.get();
        let gain = self.gain.get();

        let w0 = (2.0 * std::f32::consts::PI * freq) / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);
        let a = (10.0_f32).powf(gain / 40.0);

        let (b0_raw, b1_raw, b2_raw, a0_raw, a1_raw, a2_raw) = match self.filter_type.get() {
            FilterType::Peaking => {
                let alpha_peak = sin_w0 / (2.0 * q);
                (
                    1.0 + alpha_peak * a,
                    -2.0 * cos_w0,
                    1.0 - alpha_peak * a,
                    1.0 + alpha_peak / a,
                    -2.0 * cos_w0,
                    1.0 - alpha_peak / a,
                )
            }
            FilterType::LowShelf => {
                let sqrt_a = a.sqrt();
                let alpha_s = sin_w0 / (2.0 * q);
                (
                    a * (a + 1.0 - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha_s),
                    2.0 * a * (a - 1.0 - (a + 1.0) * cos_w0),
                    a * (a + 1.0 - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha_s),
                    a + 1.0 + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha_s,
                    -2.0 * (a - 1.0 + (a + 1.0) * cos_w0),
                    a + 1.0 + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha_s,
                )
            }
            FilterType::HighShelf => {
                let sqrt_a = a.sqrt();
                let alpha_s = sin_w0 / (2.0 * q);
                (
                    a * (a + 1.0 + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha_s),
                    -2.0 * a * (a - 1.0 + (a + 1.0) * cos_w0),
                    a * (a + 1.0 + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha_s),
                    a + 1.0 - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha_s,
                    2.0 * (a - 1.0 - (a + 1.0) * cos_w0),
                    a + 1.0 - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha_s,
                )
            }
            FilterType::LowPass => (
                (1.0 - cos_w0) / 2.0,
                1.0 - cos_w0,
                (1.0 - cos_w0) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            FilterType::HighPass => (
                (1.0 + cos_w0) / 2.0,
                -(1.0 + cos_w0),
                (1.0 + cos_w0) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            FilterType::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha),
            FilterType::Notch => (
                1.0,
                -2.0 * cos_w0,
                1.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
        };

        let inv_a0 = 1.0 / a0_raw;
        self.b0 = b0_raw * inv_a0;
        self.b1 = b1_raw * inv_a0;
        self.b2 = b2_raw * inv_a0;
        self.a1 = a1_raw * inv_a0;
        self.a2 = a2_raw * inv_a0;
    }

    pub fn process_sample(&mut self, sample: f32, channel: usize) -> f32 {
        if !self.active.get() || channel >= self.x1.len() {
            return sample;
        }

        let num_stages = (self.order.get() as usize + 1).min(MAX_ORDER);
        let mut signal = sample;

        for stage in 0..num_stages {
            let x0 = signal;
            let y0 = self.b0 * x0
                + self.b1 * self.x1[channel][stage]
                + self.b2 * self.x2[channel][stage]
                - self.a1 * self.y1[channel][stage]
                - self.a2 * self.y2[channel][stage];

            self.x2[channel][stage] = self.x1[channel][stage];
            self.x1[channel][stage] = x0;
            self.y2[channel][stage] = self.y1[channel][stage];
            self.y1[channel][stage] = y0;

            signal = y0;
        }
        signal
    }

    pub fn reset_state(&mut self) {
        for channel in 0..self.x1.len() {
            self.x1[channel] = [0.0; MAX_ORDER];
            self.x2[channel] = [0.0; MAX_ORDER];
            self.y1[channel] = [0.0; MAX_ORDER];
            self.y2[channel] = [0.0; MAX_ORDER];
        }
    }

    pub fn magnitude_db_at(&self, freq: f32, sample_rate: f32) -> f32 {
        if !self.active.get() {
            return 0.0;
        }

        let w = (2.0 * std::f32::consts::PI * freq) / sample_rate;
        let cos_w = w.cos();
        let cos_2w = (2.0 * w).cos();
        let sin_w = w.sin();
        let sin_2w = (2.0 * w).sin();

        let num_re = self.b0 + self.b1 * cos_w + self.b2 * cos_2w;
        let num_im = -(self.b1 * sin_w + self.b2 * sin_2w);
        let num_mag_sq = num_re * num_re + num_im * num_im;

        let den_re = 1.0 + self.a1 * cos_w + self.a2 * cos_2w;
        let den_im = -(self.a1 * sin_w + self.a2 * sin_2w);
        let den_mag_sq = den_re * den_re + den_im * den_im;

        if den_mag_sq < 1e-20 {
            return 0.0;
        }

        let single_stage_db = 10.0 * (num_mag_sq / den_mag_sq).max(1e-20).log10();
        let num_stages = (self.order.get() as usize as f32) + 1.0;
        single_stage_db * num_stages
    }
}

// --------------------------------------------------------------------------
// THE MAIN PLUGIN
// --------------------------------------------------------------------------

#[derive(Clone)]
#[karbeat_plugin]
pub struct KarbeatParametricEQ {
    #[nested(prefix = "band")]
    pub nodes: Vec<KarbeatParametricEQFilterNode>,

    #[param(id = "base_gain", name = "Base Gain", group = "Master", min = -60.0, max = 24.0, default = 0.0, step = 0.1)]
    pub base_gain: f32,

    last_sample_rate: f32,
    channels: usize,

    // FFT Analyzer State
    analyzer_buffer: SmallVec<[f32; FFT_SIZE]>,
    analyzer_idx: usize,
    spectrum_history: Vec<f32>,
    fft_instance: Option<Arc<dyn Fft<f32>>>,
}

impl Default for KarbeatParametricEQ {
    fn default() -> Self {
        Self::new()
    }
}

impl KarbeatParametricEQ {
    pub fn new() -> Self {
        let mut engine = Self::base_default();

        let default_freqs = [60.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];
        for (i, &f) in default_freqs.iter().enumerate() {
            engine.nodes.push(KarbeatParametricEQFilterNode::new(i, f));
        }

        engine.last_sample_rate = 48000.0;
        engine.channels = 2;

        let mut planner = FftPlanner::new();
        engine.fft_instance = Some(planner.plan_fft_forward(FFT_SIZE));
        engine.analyzer_buffer = smallvec![0.0; FFT_SIZE];
        engine.analyzer_idx = 0;
        engine.spectrum_history = Vec::new();

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

// ============================================================================
// DIRECT KARBEAT EFFECT TRAIT
// ============================================================================

impl KarbeatPlugin for KarbeatParametricEQ {
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

            self.analyzer_buffer[self.analyzer_idx] = mono_mix / self.channels as f32;
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

                let json_response: Vec<Value> = response
                    .into_iter()
                    .map(|(freq, db)| json!({ "frequency": freq, "magnitude_db": db }))
                    .collect();

                Some(json!(json_response))
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

                for i in 0..(FFT_SIZE / 2) {
                    let mag = fft_input[i].norm() / norm_factor;
                    let db = 20.0 * mag.log10();
                    raw_magnitudes[i] = db.clamp(-100.0, 24.0);
                }

                if self.spectrum_history.len() != num_points {
                    self.spectrum_history = vec![-100.0; num_points];
                }
                let min_freq = 20.0;
                let max_freq = 20000.0;
                let log_min = min_freq.log10();
                let log_max = max_freq.log10();

                let mut json_response = Vec::with_capacity(num_points);

                for i in 0..num_points {
                    let t = (i as f32) / ((num_points - 1).max(1) as f32);
                    let target_freq = (10.0_f32).powf(log_min + t * (log_max - log_min));

                    let bin_exact =
                        target_freq * (FFT_SIZE as f32) / self.last_sample_rate.max(1.0);
                    let bin_idx = bin_exact.round() as usize;

                    let mut current_db = -100.0;
                    if bin_idx > 0 && bin_idx < raw_magnitudes.len() {
                        // Max pooling over nearby bins prevents skipping spikes at high frequencies
                        let pool_radius = (bin_exact * 0.02).max(1.0) as usize;
                        let start = bin_idx.saturating_sub(pool_radius);
                        let end = (bin_idx + pool_radius).min(raw_magnitudes.len() - 1);

                        for b in start..=end {
                            if raw_magnitudes[b] > current_db {
                                current_db = raw_magnitudes[b];
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

                    json_response
                        .push(json!({ "frequency": target_freq, "magnitude_db": smoothed_db }));
                }

                // log::debug!("Send some Spectrum response");
                Some(json!(json_response))
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

    fn category(&self) -> PluginCategory {
        PluginCategory::Effect
    }
}

impl Manifestable for KarbeatParametricEQ {
    fn build_manifest() -> PluginManifest {
        PluginManifest {
            id: 0, // Set to your global plugin ID
            name: "Karbeat Parametric EQ".into(),
            internal_type: "KarbeatParametricEQ".into(),
            is_synth: false,
            parameters: Self::static_parameter_specs(),
        }
    }
}

impl AudioPluginBuilder for KarbeatParametricEQ {
    fn build() -> Self {
        Self::new()
    }
}
