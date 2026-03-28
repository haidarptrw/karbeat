use std::collections::HashMap;

/// Maximum number of cascaded biquad stages per band (order 0..3 = 1..4 stages)
const MAX_ORDER: usize = 4;

use karbeat_plugin_api::wrapper::{RawEffectEngine, RawEffectWrapper};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterType {
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
    /// Type of filter
    pub filter_type: FilterType,

    /// Q value of this filter node
    pub q: f32,

    /// Order of the filter (0 = 1 stage/12dB, 1 = 2 stages/24dB, 2 = 3/36dB, 3 = 4/48dB)
    pub order: u16,

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

    // Cascaded biquad state: [channel(L/R...N)][stage]
    x1: Vec<[f32; MAX_ORDER]>,
    x2: Vec<[f32; MAX_ORDER]>,
    y1: Vec<[f32; MAX_ORDER]>,
    y2: Vec<[f32; MAX_ORDER]>,
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
            x1: vec![[0.0; MAX_ORDER]; 2], // Default to stereo, will be resized if necessary
            x2: vec![[0.0; MAX_ORDER]; 2],
            y1: vec![[0.0; MAX_ORDER]; 2],
            y2: vec![[0.0; MAX_ORDER]; 2],
        };
        // Initial calc assuming 48k, will be updated in process
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

    /// Calculate Biquad Coefficients for Peaking EQ
    ///
    /// The calculation of biquad coefficients is based on the standard formulas for digital biquad filters,
    /// which depend on the filter type, frequency, Q factor, and gain.
    /// The coefficients are normalized by a0 to ensure stability and consistent gain across different filter types and parameters.
    ///
    /// The calculation algorithm is adapted from the Audio EQ Cookbook by Robert Bristow-Johnson,
    /// which provides a comprehensive set of formulas for various biquad filter types.
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

        // Normalize by a0
        let inv_a0 = 1.0 / a0_raw;
        self.b0 = b0_raw * inv_a0;
        self.b1 = b1_raw * inv_a0;
        self.b2 = b2_raw * inv_a0;
        self.a1 = a1_raw * inv_a0;
        self.a2 = a2_raw * inv_a0;
    }

    pub fn process_sample(&mut self, sample: f32, channel: usize) -> f32 {
        if !self.active || channel >= self.x1.len() {
            return sample;
        }

        let num_stages = (self.order as usize + 1).min(MAX_ORDER);
        let mut signal = sample;

        for stage in 0..num_stages {
            // Direct Form I per stage
            let x0 = signal;
            let y0 = self.b0 * x0
                + self.b1 * self.x1[channel][stage]
                + self.b2 * self.x2[channel][stage]
                - self.a1 * self.y1[channel][stage]
                - self.a2 * self.y2[channel][stage];

            // Shift state for this stage
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

    /// Compute the magnitude response in dB at a given frequency.
    /// Uses the biquad transfer function: H(z) = (b0 + b1·z⁻¹ + b2·z⁻²) / (1 + a1·z⁻¹ + a2·z⁻²)
    /// where z = e^(jω), ω = 2π·f/sample_rate
    pub fn magnitude_db_at(&self, freq: f32, sample_rate: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        let w = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let cos_w = w.cos();
        let cos_2w = (2.0 * w).cos();
        let sin_w = w.sin();
        let sin_2w = (2.0 * w).sin();

        // Numerator: b0 + b1·e^(-jω) + b2·e^(-j2ω)
        let num_re = self.b0 + self.b1 * cos_w + self.b2 * cos_2w;
        let num_im = -(self.b1 * sin_w + self.b2 * sin_2w);
        let num_mag_sq = num_re * num_re + num_im * num_im;

        // Denominator: 1 + a1·e^(-jω) + a2·e^(-j2ω)
        let den_re = 1.0 + self.a1 * cos_w + self.a2 * cos_2w;
        let den_im = -(self.a1 * sin_w + self.a2 * sin_2w);
        let den_mag_sq = den_re * den_re + den_im * den_im;

        if den_mag_sq < 1e-20 {
            return 0.0;
        }

        let single_stage_db = 10.0 * (num_mag_sq / den_mag_sq).max(1e-20).log10();
        // Cascading N identical biquads = N × single-stage dB
        let num_stages = (self.order as f32) + 1.0;
        single_stage_db * num_stages
    }
}

#[derive(Clone)]
pub struct KarbeatParametricEQEngine {
    /// Nodes
    pub nodes: Vec<KarbeatParametricEQFilterNode>,
    /// Base gain for all eq
    pub base_gain: f32,
    /// Cache sample rate to detect changes
    last_sample_rate: f32,
    /// Number of channels
    channels: usize,
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
            channels: 2,
        }
    }

    fn update_all_nodes(&mut self) {
        for node in &mut self.nodes {
            node.ensure_channels(self.channels);
            node.update_coefficients(self.last_sample_rate);
        }
    }

    /// Compute the composite magnitude response curve at log-spaced frequency points.
    /// Returns Vec<(frequency_hz, magnitude_db)> pairs.
    pub fn compute_magnitude_response(&self, num_points: usize) -> Vec<(f32, f32)> {
        let min_freq: f32 = 20.0;
        let max_freq: f32 = 20000.0;
        let log_min = min_freq.log10();
        let log_max = max_freq.log10();

        let mut result = Vec::with_capacity(num_points);

        for i in 0..num_points {
            let t = i as f32 / (num_points - 1).max(1) as f32;
            let freq = 10.0_f32.powf(log_min + t * (log_max - log_min));

            // Composite magnitude: sum of dB contributions from all bands
            // (multiplication of linear magnitudes = addition of dB values)
            let mut total_db: f32 = self.base_gain;
            for node in &self.nodes {
                total_db += node.magnitude_db_at(freq, self.last_sample_rate);
            }

            result.push((freq, total_db));
        }

        result
    }
}

impl RawEffectEngine for KarbeatParametricEQEngine {
    fn prepare(&mut self, sample_rate: f32, channels: usize, _max_buffer_size: usize) {
        let needs_update = (sample_rate - self.last_sample_rate).abs() > 0.1 || self.channels != channels;
        if needs_update {
            self.last_sample_rate = sample_rate;
            self.channels = channels;
            self.update_all_nodes();
        }
    }

    fn process(
        &mut self,
        base: &mut karbeat_plugin_api::effect_base::StandardEffectBase,
        buffer: &mut [f32],
    ) {
        // Check for sample rate or channel changes
        let needs_update = (base.sample_rate - self.last_sample_rate).abs() > 0.1 || base.channels != self.channels;
        if needs_update {
            self.last_sample_rate = base.sample_rate;
            self.channels = base.channels;
            self.update_all_nodes();
        }

        let master_linear_gain = if self.base_gain.abs() > 0.01 {
            10.0_f32.powf(self.base_gain / 20.0)
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

                    // Apply all EQ bands in series
                    for node in &mut self.nodes {
                        if node.active {
                            sample = node.process_sample(sample, channel);
                        }
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

    fn set_custom_parameter(&mut self, id: u32, value: f32) {
        match id {
            2 => self.base_gain = value.clamp(-60.0, 24.0),
            _ => {
                if id < 3 {
                    return;
                }
                let relative_id = id - 3;
                let band_idx = (relative_id / 6) as usize;
                let param_type = relative_id % 6;

                if let Some(node) = self.nodes.get_mut(band_idx) {
                    match param_type {
                        0 => node.freq = value.clamp(20.0, 22000.0),
                        1 => node.gain = value.clamp(-24.0, 24.0),
                        2 => node.q = value.clamp(0.1, 20.0),
                        3 => node.active = value > 0.5,
                        4 => node.filter_type = FilterType::from(value),
                        5 => {
                            node.order = (value.round() as u16).min((MAX_ORDER - 1) as u16);
                            node.reset_state(); // Reset state when order changes
                        }
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
                let band_idx = (relative_id / 6) as usize;
                let param_type = relative_id % 6;

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
                    5 => node.order as f32,
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
            let base = 3 + (i as u32) * 6;

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
            params.insert(base + 5, 0.0); // Order (0 = 12dB/oct)
        }

        params
    }

    fn name() -> &'static str
    where
        Self: Sized,
    {
        "Parametric EQ"
    }

    fn get_parameter_specs(&self) -> Vec<karbeat_plugin_api::wrapper::PluginParameter> {
        use karbeat_plugin_api::wrapper::PluginParameter;

        let filter_type_choices = vec![
            "Peaking".into(),
            "Low Shelf".into(),
            "High Shelf".into(),
            "Low Pass".into(),
            "High Pass".into(),
            "Band Pass".into(),
            "Notch".into(),
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

        let order_choices = vec![
            "12 dB/oct".into(),
            "24 dB/oct".into(),
            "36 dB/oct".into(),
            "48 dB/oct".into(),
        ];

        for (i, node) in self.nodes.iter().enumerate() {
            let base_id = 3 + (i as u32) * 6;
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
                20000.0,
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
            params.push(PluginParameter::new_choice(
                base_id + 5,
                "Slope",
                &group,
                node.order as u32,
                order_choices.clone(),
                0, // Default: 12 dB/oct
            ));
        }

        params
    }
}

pub type KarbeatParametricEQ = RawEffectWrapper<KarbeatParametricEQEngine>;

pub fn create_parametric_eq(
    sample_rate: Option<f32>,
    channels: usize,
) -> RawEffectWrapper<KarbeatParametricEQEngine> {
    let mut engine = KarbeatParametricEQEngine::new();
    // Default config initialization if sample rate and channels are passed
    if let Some(sr) = sample_rate {
        engine.last_sample_rate = sr;
    }
    engine.channels = channels;
    engine.update_all_nodes();

    RawEffectWrapper::new(
        engine,
        sample_rate.unwrap_or(48000.0),
        channels
    )
}


