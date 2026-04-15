
use karbeat_macros::{inject_plugin_routing, karbeat_plugin};
use karbeat_plugin_api::prelude::*;
use karbeat_plugin_types::*;
use serde_json::{ json, Value };

/// Maximum number of cascaded biquad stages per band (order 0..3 = 1..4 stages)
const MAX_ORDER: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Default, karbeat_macros::EnumParam)]
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
            _ => FilterType::Peaking,
        }
    }
}

/// Node of each filter
#[derive(Clone)]
pub struct KarbeatParametricEQFilterNode {
    pub freq: Param<f32>,
    pub gain: Param<f32>,
    pub q: Param<f32>,
    pub active: Param<bool>,
    pub filter_type: Param<FilterType>,
    pub order: Param<usize>,

    // Internal Runtime Coefficients
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,

    // Cascaded biquad state: [channel(L/R...N)][stage]
    x1: Vec<[f32; MAX_ORDER]>,
    x2: Vec<[f32; MAX_ORDER]>,
    y1: Vec<[f32; MAX_ORDER]>,
    y2: Vec<[f32; MAX_ORDER]>,
}

impl KarbeatParametricEQFilterNode {
    pub fn new(band_idx: usize, default_freq: f32) -> Self {
        let base_id = 3 + (band_idx as u32) * 6;
        let group = format!("Band {}", band_idx + 1);

        let default_type = if band_idx == 0 {
            FilterType::LowShelf
        } else if band_idx == 7 {
            FilterType::HighShelf
        } else {
            FilterType::Peaking
        };

        let mut node = Self {
            freq: Param::new_float(base_id, "Frequency", &group, default_freq, 20.0, 20000.0),
            gain: Param::new_float(base_id + 1, "Gain", &group, 0.0, -24.0, 24.0),
            q: Param::new_float(base_id + 2, "Q", &group, 0.707, 0.1, 20.0),
            active: Param::new_bool(base_id + 3, "Active", &group, true),
            filter_type: Param::new_enum(base_id + 4, "Type", &group, default_type),
            order: Param::new_choice(
                base_id + 5,
                "Slope",
                &group,
                0,
                vec!["12 dB/oct", "24 dB/oct", "36 dB/oct", "48 dB/oct"]
            ),

            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: vec![[0.0; MAX_ORDER]; 2],
            x2: vec![[0.0; MAX_ORDER]; 2],
            y1: vec![[0.0; MAX_ORDER]; 2],
            y2: vec![[0.0; MAX_ORDER]; 2],
        };

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

        // Pulling DSP values from the thread-safe Param wrappers
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
            FilterType::LowPass =>
                (
                    (1.0 - cos_w0) / 2.0,
                    1.0 - cos_w0,
                    (1.0 - cos_w0) / 2.0,
                    1.0 + alpha,
                    -2.0 * cos_w0,
                    1.0 - alpha,
                ),
            FilterType::HighPass =>
                (
                    (1.0 + cos_w0) / 2.0,
                    -(1.0 + cos_w0),
                    (1.0 + cos_w0) / 2.0,
                    1.0 + alpha,
                    -2.0 * cos_w0,
                    1.0 - alpha,
                ),
            FilterType::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha),
            FilterType::Notch => (1.0, -2.0 * cos_w0, 1.0, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha),
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

        let num_stages = (self.order.get() + 1).min(MAX_ORDER);
        let mut signal = sample;

        for stage in 0..num_stages {
            let x0 = signal;
            let y0 =
                self.b0 * x0 +
                self.b1 * self.x1[channel][stage] +
                self.b2 * self.x2[channel][stage] -
                self.a1 * self.y1[channel][stage] -
                self.a2 * self.y2[channel][stage];

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
        let num_stages = (self.order.get() as f32) + 1.0;
        single_stage_db * num_stages
    }
}

impl AutoParams for KarbeatParametricEQFilterNode {
    fn auto_get_parameter(&self, id: u32) -> Option<f32> {
        if id == self.freq.id {
            Some(self.freq.get_base().to_f32())
        } else if id == self.gain.id {
            Some(self.gain.get_base().to_f32())
        } else if id == self.q.id {
            Some(self.q.get_base().to_f32())
        } else if id == self.active.id {
            Some(self.active.get_base().to_f32())
        } else if id == self.filter_type.id {
            Some(self.filter_type.get_base().to_f32())
        } else if id == self.order.id {
            Some(self.order.get_base().to_f32())
        } else {
            None
        }
    }

    fn auto_set_parameter(&mut self, id: u32, value: f32) {
        if id == self.freq.id {
            self.freq.set_base(value)
        } else if id == self.gain.id {
            self.gain.set_base(value)
        } else if id == self.q.id {
            self.q.set_base(value)
        } else if id == self.active.id {
            self.active.set_base(value)
        } else if id == self.filter_type.id {
            self.filter_type.set_base(value)
        } else if id == self.order.id {
            self.order.set_base(value)
        }
    }

    fn auto_apply_automation(&mut self, id: u32, value: f32) {
        if id == self.freq.id {
            self.freq.apply_automation(value)
        } else if id == self.gain.id {
            self.gain.apply_automation(value)
        } else if id == self.q.id {
            self.q.apply_automation(value)
        } else if id == self.active.id {
            self.active.apply_automation(value)
        } else if id == self.filter_type.id {
            self.filter_type.apply_automation(value)
        } else if id == self.order.id {
            self.order.apply_automation(value)
        }
    }

    fn auto_clear_automation(&mut self, id: u32) {
        if id == self.freq.id {
            self.freq.clear_automation()
        } else if id == self.gain.id {
            self.gain.clear_automation()
        } else if id == self.q.id {
            self.q.clear_automation()
        } else if id == self.active.id {
            self.active.clear_automation()
        } else if id == self.filter_type.id {
            self.filter_type.clear_automation()
        } else if id == self.order.id {
            self.order.clear_automation()
        }
    }

    fn auto_get_parameter_specs(&self) -> Vec<PluginParameter> {
        vec![
            self.freq.to_spec(),
            self.gain.to_spec(),
            self.q.to_spec(),
            self.active.to_spec(),
            self.filter_type.to_spec(),
            self.order.to_spec()
        ]
    }
}

#[derive(Clone)]
#[karbeat_plugin]
pub struct KarbeatParametricEQEngine {
    #[nested]
    pub nodes: Vec<KarbeatParametricEQFilterNode>,

    #[param(id = 2, name = "Base Gain", group = "Master", min = -60.0, max = 24.0, default = 0.0)]
    pub base_gain: f32,

    // Ignored natively by macro
    last_sample_rate: f32,
    channels: usize,
}

impl Default for KarbeatParametricEQEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl KarbeatParametricEQEngine {
    pub fn new() -> Self {
        // base_default is generated by the macro!
        let mut engine = Self::base_default();

        let default_freqs = [60.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];
        for (i, &f) in default_freqs.iter().enumerate() {
            engine.nodes.push(KarbeatParametricEQFilterNode::new(i, f));
        }

        engine.last_sample_rate = 48000.0;
        engine.channels = 2;
        engine
    }

    fn update_all_nodes(&mut self) {
        for node in &mut self.nodes {
            node.ensure_channels(self.channels);
            node.update_coefficients(self.last_sample_rate);
        }
    }

    /// Triggered whenever an AutoParam mutates state, allowing us to safely
    /// rebuild DSP coefficients without giant match statements.
    fn handle_node_side_effects(&mut self, id: u32) {
        for node in &mut self.nodes {
            let base_id = node.freq.id;
            // The stride is 6, so if the ID falls in this node's range, update it.
            if id >= base_id && id <= base_id + 5 {
                if id == base_id + 5 {
                    node.reset_state(); // Clear delay buffers if slope order changed
                }
                node.update_coefficients(self.last_sample_rate);
                break;
            }
        }
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

#[inject_plugin_routing(handle_node_side_effects)]
impl RawEffectEngine for KarbeatParametricEQEngine {
    fn name() -> &'static str where Self: Sized {
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

    fn process(
        &mut self,
        base: &mut karbeat_plugin_api::effect_base::StandardEffectBase,
        buffer: &mut [f32]
    ) {
        let needs_update =
            (base.sample_rate - self.last_sample_rate).abs() > 0.1 ||
            base.channels != self.channels;

        if needs_update {
            self.last_sample_rate = base.sample_rate;
            self.channels = base.channels;
            self.update_all_nodes();
        }

        let current_base_gain = self.base_gain.get();
        let master_linear_gain = if current_base_gain.abs() > 0.01 {
            (10.0_f32).powf(current_base_gain / 20.0)
        } else {
            1.0
        };

        if self.channels == 0 {
            return;
        }

        for i in (0..buffer.len()).step_by(self.channels) {
            for channel in 0..self.channels {
                if i + channel < buffer.len() {
                    let mut sample = buffer[i + channel] * master_linear_gain;

                    for node in &mut self.nodes {
                        sample = node.process_sample(sample, channel);
                    }

                    buffer[i + channel] = sample;
                }
            }
        }
    }

    fn reset(&mut self) {
        for node in &mut self.nodes {
            node.reset_state();
        }
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
            _ => None,
        }
    }
}

pub type KarbeatParametricEQ = RawEffectWrapper<KarbeatParametricEQEngine>;
