use std::collections::HashMap;

use crate::{
    core::project::PluginInstance,
    plugin::wrapper::{EffectWrapper, RawEffectEngine},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterType {
    Peaking = 0,
    LowShelf = 1,
    HighShelf = 2,
    LowPass = 3,
    HighPass = 4,
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
#[derive(Clone, Copy)]
pub struct KarbeatParametricEQFilterNode {
    /// Type of filter
    pub filter_type: FilterType,

    /// Q value of this filter node
    pub q: f32,

    /// Order of the filter
    pub order: u16, // starts from order 0

    /// Indicates whether this node is effective or not
    pub active: bool,

    /// gain of this node
    pub gain: f32,

    /// Frequency of this node
    pub freq: f32,

    // Internal Runtime Coefficients
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,

    // x[n-1], x[n-2] for L/R
    x1: [f32; 2],
    x2: [f32; 2],
    // y[n-1], y[n-2] for L/R
    y1: [f32; 2],
    y2: [f32; 2],
}

impl KarbeatParametricEQFilterNode {
    pub fn new(freq: f32) -> Self {
        let mut node = Self {
            filter_type: FilterType::Peaking,
            q: 0.707, // Default Q (Butterworth for LPF/HPF)
            order: 0,
            active: true,
            gain: 0.0,
            freq,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: [0.0; 2],
            x2: [0.0; 2],
            y1: [0.0; 2],
            y2: [0.0; 2],
        };
        // Initial calc assuming 48k, will be updated in process
        node.update_coefficients(48000.0);
        node
    }

    /// Calculate Biquad Coefficients for Peaking EQ
    pub fn update_coefficients(&mut self, sample_rate: f32) {
        if sample_rate <= 0.0 {
            return;
        }

        let w0 = 2.0 * std::f32::consts::PI * self.freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * self.q);

        // A = 10^(dBgain/40) for Peaking/Shelving
        let a = 10.0_f32.powf(self.gain / 40.0);

        let (b0_raw, b1_raw, b2_raw, a0_raw, a1_raw, a2_raw) = match self.filter_type {
            FilterType::Peaking => {
                let alpha_peak = sin_w0 / (2.0 * self.q);
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
                let alpha_s = sin_w0 / (2.0 * self.q);
                (
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha_s),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha_s),
                    (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha_s,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
                    (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha_s,
                )
            }
            FilterType::HighShelf => {
                let sqrt_a = a.sqrt();
                let alpha_s = sin_w0 / (2.0 * self.q);

                (
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha_s),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha_s),
                    (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha_s,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
                    (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha_s,
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
        };

        // Normalize by a0
        let inv_a0 = 1.0 / a0_raw;
        self.b0 = b0_raw * inv_a0;
        self.b1 = b1_raw * inv_a0;
        self.b2 = b2_raw * inv_a0;
        self.a1 = a1_raw * inv_a0;
        self.a2 = a2_raw * inv_a0;
    }

    /// Process a single sample for a specific channel (0=Left, 1=Right)
    #[inline]
    pub fn process_sample(&mut self, sample: f32, channel: usize) -> f32 {
        if !self.active {
            return sample;
        }

        // Direct Form I
        // y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]
        let x0 = sample;
        let y0 = self.b0 * x0 + self.b1 * self.x1[channel] + self.b2 * self.x2[channel]
            - self.a1 * self.y1[channel]
            - self.a2 * self.y2[channel];

        // Shift state
        self.x2[channel] = self.x1[channel];
        self.x1[channel] = x0;
        self.y2[channel] = self.y1[channel];
        self.y1[channel] = y0;

        y0
    }

    pub fn reset_state(&mut self) {
        self.x1 = [0.0; 2];
        self.x2 = [0.0; 2];
        self.y1 = [0.0; 2];
        self.y2 = [0.0; 2];
    }
}

#[derive(Clone)]
pub struct KarbeatParametricEQEngine {
    /// Nodes
    pub nodes: Vec<KarbeatParametricEQFilterNode>,
    /// Base gain  for all eq
    pub base_gain: f32,
    /// Cache sample rate to detect changes
    last_sample_rate: f32,
}

impl KarbeatParametricEQEngine {
    pub fn new() -> Self {
        // Initialize 8 default bands
        let default_freqs = [60.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];
        let mut nodes = Vec::with_capacity(8);
        for &f in default_freqs.iter() {
            nodes.push(KarbeatParametricEQFilterNode::new(f));
        }

        Self {
            nodes,
            base_gain: 0.0, // 0 dB default
            last_sample_rate: 48000.0,
        }
    }

    fn update_all_nodes(&mut self) {
        for node in &mut self.nodes {
            node.update_coefficients(self.last_sample_rate);
        }
    }
}

impl RawEffectEngine for KarbeatParametricEQEngine {
    fn process(&mut self, base: &mut crate::plugin::effect_base::EffectBase, buffer: &mut [f32]) {
        // Check for sample rate changes
        if (base.sample_rate - self.last_sample_rate).abs() > 0.1 {
            self.last_sample_rate = base.sample_rate;
            self.update_all_nodes();
        }

        let master_linear_gain = if self.base_gain.abs() > 0.01 {
            10.0_f32.powf(self.base_gain / 20.0)
        } else {
            1.0
        };

        for i in (0..buffer.len()).step_by(2) {
            let mut l = buffer[i] * master_linear_gain;
            let mut r = buffer[i + 1] * master_linear_gain;

            // Apply all EQ bands in series
            for node in &mut self.nodes {
                if node.active {
                    l = node.process_sample(l, 0);
                    r = node.process_sample(r, 1);
                }
            }

            buffer[i] = l;
            buffer[i + 1] = r;
        }
    }

    fn reset(&mut self) {
        for node in &mut self.nodes {
            node.reset_state();
        }
    }

    fn set_custom_parameter(&mut self, id: u32, value: f32) {
        match id {
            2 => self.base_gain = value.clamp(-60.0, 24.0),
            _ => {
                if id < 3 {
                    return;
                }
                let relative_id = id - 3;
                let band_idx = (relative_id / 5) as usize;
                let param_type = relative_id % 5;

                if let Some(node) = self.nodes.get_mut(band_idx) {
                    match param_type {
                        0 => node.freq = value.clamp(20.0, 22000.0),
                        1 => node.gain = value.clamp(-24.0, 24.0),
                        2 => node.q = value.clamp(0.1, 20.0),
                        3 => node.active = value > 0.5,
                        4 => node.filter_type = FilterType::from(value),
                        _ => {}
                    }
                    node.update_coefficients(self.last_sample_rate);
                }
            }
        }
    }

    fn get_custom_parameter(&self, id: u32) -> Option<f32> {
        match id {
            2 => Some(self.base_gain),
            _ => {
                if id < 3 {
                    return None;
                }
                let relative_id = id - 3;
                let band_idx = (relative_id / 5) as usize;
                let param_type = relative_id % 5;

                self.nodes.get(band_idx).map(|node| match param_type {
                    0 => node.freq,
                    1 => node.gain,
                    2 => node.q,
                    3 => {
                        if node.active {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    4 => node.filter_type as i32 as f32,
                    _ => 0.0,
                })
            }
        }
    }

    fn custom_default_parameters() -> HashMap<u32, f32>
    where
        Self: Sized,
    {
        let mut params = HashMap::new();
        params.insert(2, 0.0);

        let default_freqs = [60.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];

        for (i, &freq) in default_freqs.iter().enumerate() {
            let base = 3 + (i as u32) * 5;

            let default_type = if i == 0 {
                FilterType::LowShelf
            } else if i == 7 {
                FilterType::HighShelf
            } else {
                FilterType::Peaking
            };

            params.insert(base + 0, freq); // Freq
            params.insert(base + 1, 0.0); // Gain
            params.insert(base + 2, 0.707); // Q
            params.insert(base + 3, 1.0); // Active
            params.insert(base + 4, default_type as i32 as f32);
        }

        params
    }

    fn name() -> &'static str
    where
        Self: Sized,
    {
        "Parametric EQ"
    }

    fn get_parameter_specs(&self) -> Vec<crate::plugin::wrapper::PluginParameter> {
        use crate::plugin::wrapper::PluginParameter;

        let filter_type_choices = vec![
            "Peaking".into(),
            "Low Shelf".into(),
            "High Shelf".into(),
            "Low Pass".into(),
            "High Pass".into(),
        ];

        let default_freqs = [60.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];

        let mut params = vec![PluginParameter::new_float(
            2,
            "Base Gain",
            "Master",
            self.base_gain,
            -60.0,
            24.0,
            0.0,
        )];

        for (i, node) in self.nodes.iter().enumerate() {
            let base_id = 3 + (i as u32) * 5;
            let group = format!("Band {}", i + 1);

            let default_type = if i == 0 {
                FilterType::LowShelf
            } else if i == 7 {
                FilterType::HighShelf
            } else {
                FilterType::Peaking
            };

            params.push(PluginParameter::new_float(
                base_id,
                "Frequency",
                &group,
                node.freq,
                20.0,
                22000.0,
                default_freqs[i],
            ));
            params.push(PluginParameter::new_float(
                base_id + 1,
                "Gain",
                &group,
                node.gain,
                -24.0,
                24.0,
                0.0,
            ));
            params.push(PluginParameter::new_float(
                base_id + 2,
                "Q",
                &group,
                node.q,
                0.1,
                20.0,
                0.707,
            ));
            params.push(PluginParameter::new_bool(
                base_id + 3,
                "Active",
                &group,
                node.active,
                true,
            ));
            params.push(PluginParameter::new_choice(
                base_id + 4,
                "Type",
                &group,
                node.filter_type as u32,
                filter_type_choices.clone(),
                default_type as u32,
            ));
        }

        params
    }
}

pub type KarbeatParametricEQ = EffectWrapper<KarbeatParametricEQEngine>;

pub fn create_parametric_eq(sample_rate: Option<f32>) -> EffectWrapper<KarbeatParametricEQEngine> {
    EffectWrapper::new(
        KarbeatParametricEQEngine::new(),
        sample_rate.unwrap_or(48000.0),
    )
}

impl From<KarbeatParametricEQ> for PluginInstance {
    fn from(_wrapper: KarbeatParametricEQ) -> Self {
        PluginInstance {
            name: "Parametric EQ".to_string(),
            internal_type: "PARAM_EQ".to_string(),
            bypass: false,
            parameters: HashMap::new(),
        }
    }
}
