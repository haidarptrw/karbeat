use karbeat_dsp::filter::{
    BiquadCoefficients, BiquadFilterType, FilterMode, SingleBiquadFilterStage,
};

use karbeat_macros::{karbeat_plugin, EnumParam};
use karbeat_plugin_api::prelude::*;
use num_complex::{Complex, Complex32};
use rustfft::{num_traits::Zero, Fft, FftPlanner};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use smallvec::{smallvec, SmallVec};
use std::sync::Arc;

const FFT_SIZE: usize = 4096;
/// 20.0 / ln(10.0) — used for fast dB conversion: dB = FAST_DB_SCALE * ln(x)
const FAST_DB_SCALE: f32 = 8.685_889_6; // 20.0 / LN_10
                                        // const LN_10: f32 = std::f32::consts::LN_10;

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
            node.filter_type.set_base(BiquadFilterType::LowShelf);
        } else if band_idx == 7 {
            node.filter_type.set_base(BiquadFilterType::HighShelf);
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

/// Log-spaced frequency table and Hann window, computed once at `prepare()`.
#[derive(Clone)]
struct AnalyzerTables {
    /// `freqs[i]` = the target frequency for spectrum display point `i`.
    /// Length = `num_spectrum_points`.
    freqs: Box<[f32]>,
    /// `bins[i]` = exact FFT bin index (float) for `freqs[i]`.
    bins: Box<[f32]>,
    /// Hann window for FFT_SIZE samples.
    hann: Box<[f32]>,
    num_points: usize,
    sample_rate: f32,
}

impl AnalyzerTables {
    fn build(num_points: usize, sample_rate: f32) -> Self {
        // ── Hann window: w[i] = 0.5 * (1 - cos(2π·i / (N-1))) ──────────────
        let hann: Box<[f32]> = (0..FFT_SIZE)
            .map(|i| {
                0.5 * (1.0 - ((std::f32::consts::TAU * (i as f32)) / ((FFT_SIZE - 1) as f32)).cos())
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        // ── Log-spaced display frequencies & their exact FFT bins ─────────────
        // Use exp2/log2 instead of powf(10, …) for speed:
        //   10^x = 2^(x * log2(10))
        let log2_min = (20.0f32).log2();
        let log2_max = (20000.0f32).log2();
        let bin_scale = (FFT_SIZE as f32) / sample_rate.max(1.0);

        let (freqs, bins): (Vec<f32>, Vec<f32>) = (0..num_points)
            .map(|i| {
                let t = (i as f32) / ((num_points - 1).max(1) as f32);
                // exp2 is ~3× faster than powf(10.0, …) for log-spaced tables
                let freq = (log2_min + t * (log2_max - log2_min)).exp2();
                let bin = freq * bin_scale;
                (freq, bin)
            })
            .unzip();

        Self {
            freqs: freqs.into_boxed_slice(),
            bins: bins.into_boxed_slice(),
            hann,
            num_points,
            sample_rate,
        }
    }

    fn rebuild_if_needed(&mut self, num_points: usize, sample_rate: f32) {
        if self.num_points != num_points || (self.sample_rate - sample_rate).abs() > 0.1 {
            *self = Self::build(num_points, sample_rate);
        }
    }
}

#[derive(Clone)]
struct AnalyzerScratch {
    /// FFT I/O buffer — reused every FFT call, no allocation.
    fft_buf: Vec<Complex32>,
    /// `raw_db[i]` = dB magnitude of FFT bin `i`, for `i < FFT_SIZE/2`.
    raw_db: Vec<f32>,
    /// Flat output: [freq0, db0, freq1, db1, …].  Reused; Arc wraps a clone.
    flat: Vec<f32>,
}

impl AnalyzerScratch {
    fn new() -> Self {
        Self {
            fft_buf: vec![Complex::zero(); FFT_SIZE],
            raw_db: vec![0.0f32; FFT_SIZE / 2],
            flat: Vec::new(),
        }
    }
}

impl Default for AnalyzerScratch {
    fn default() -> Self {
        Self::new()
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

    /// Temporal smoothing history for spectrum display.
    /// Initialized once in `prepare`; `clear` on `reset`.
    spectrum_history: Box<[f32]>,
    fft_instance: Option<Arc<dyn Fft<f32>>>,
    samples_since_last_fft: usize,

    /// Precomputed Hann window + log-spaced frequency/bin tables.
    /// `Option` so we can lazy-init without a dummy sample_rate.
    tables: Option<Box<AnalyzerTables>>,

    /// Reusable scratch buffers — never allocate in `compute_and_update_spectrum`.
    #[serde(skip)]
    scratch: Box<AnalyzerScratch>,

    //////////////////////////////////////////
    // Shared Memory buffer
    //////////////////////////////////////////
    pub magnitude_buffer: Arc<Box<[f32]>>,
    pub spectrum_buffer: Arc<Box<[f32]>>,

    /// Determines if the FFT and ring buffer should collect data
    enable_spectrum_analyzer: bool,
    enable_magnitude_curve: bool,
}

impl Default for DigiParametricEQ {
    fn default() -> Self {
        Self::new()
    }
}

impl DigiParametricEQ {
    const SPECTRUM_POINTS: usize = 300;
    const MAGNITUDE_POINTS: usize = 500;
    const FFT_TRIGGER_SAMPLES: usize = 1600;
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
        engine.spectrum_history = vec![-100.0f32; Self::SPECTRUM_POINTS].into_boxed_slice();
        engine.tables = Some(Box::new(AnalyzerTables::build(
            Self::SPECTRUM_POINTS,
            48000.0,
        )));
        engine.scratch = Box::new(AnalyzerScratch::new());
        engine.magnitude_buffer = Arc::new(Box::new([]));
        engine.spectrum_buffer = Arc::new(Box::new([]));
        engine.enable_spectrum_analyzer = false;
        engine.enable_magnitude_curve = false;
        engine.handle_side_effects(0);
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
        if self.enable_magnitude_curve {
            self.magnitude_buffer = Arc::new(
                self.compute_magnitude_response_flat(Self::MAGNITUDE_POINTS)
                    .into_boxed_slice(),
            );
        }
    }

    /// Computes the combined EQ magnitude curve over `num_points` log-spaced
    /// frequencies.  Returns a flat `[freq, db, freq, db, …]` `Vec<f32>`.
    ///
    /// ### Key optimizations vs original
    /// 1. **Single frequency sweep, all nodes at once** — iterates over points
    ///    once; each node's `magnitude_db_at` is called once per point.
    ///    The original also did this, but the `(f32, f32)` tuple indirection
    ///    forced an extra heap allocation + two iterator passes.
    /// 2. **Pre-allocated flat output** — no intermediate `Vec<(f32,f32)>` +
    ///    `flat_map` — writes directly into the output vector.
    /// 3. **exp2 instead of powf(10.0, …)** for the frequency table.
    pub fn compute_magnitude_response_flat(&self, num_points: usize) -> Vec<f32> {
        let log2_min = (20.0f32).log2();
        let log2_max = (20000.0f32).log2();
        let base_gain = self.base_gain.get();
        let sr = self.last_sample_rate;

        let mut out = vec![0.0f32; num_points * 2];
        for i in 0..num_points {
            let t = (i as f32) / ((num_points - 1).max(1) as f32);
            let freq = (log2_min + t * (log2_max - log2_min)).exp2();
            let mut db = base_gain;
            for node in &self.nodes {
                db += node.magnitude_db_at(freq, sr);
            }
            out[i * 2] = freq;
            out[i * 2 + 1] = db;
        }
        out
    }

    pub fn compute_magnitude_response(&self, num_points: usize) -> Vec<(f32, f32)> {
        let flat = self.compute_magnitude_response_flat(num_points);
        flat.chunks_exact(2).map(|c| (c[0], c[1])).collect()
    }

    /// Runs the FFT analyzer and updates `spectrum_buffer`.
    ///
    /// ### Optimizations vs original
    ///
    /// **1. Precomputed Hann window**
    /// The original called `Windowing::Hann.apply_single_sample(s, i, FFT_SIZE)`
    /// 4 096 times per call, which computed `cos(2π·i/N)` each time.
    /// Now it's a single multiply: `sample * self.tables.hann[i]`.
    ///
    /// **2. Fast dB conversion in the magnitude loop**
    /// `20.0 * mag.log10()` → `FAST_DB_SCALE * mag.ln()`
    /// Both return the same value; `ln` maps to a single x87/SSE instruction
    /// while `log10` is computed as `ln(x) / ln(10)` internally anyway.
    ///
    /// **3. Reused scratch buffers — zero allocations per call**
    /// `fft_buf`, `raw_db`, and `flat` are all reused.
    /// The original allocated a new `Vec` + `Box` + `Arc` every ~33 ms.
    ///
    /// **4. exp2 in frequency→bin mapping**
    /// The 300 `powf(10.0, …)` calls in the point loop are replaced with
    /// `exp2` using the precomputed `tables.bins` slice — the loop just indexes.
    ///
    /// **5. Branchless lerp in the normal-radius path**
    /// The original had a nested `if pool_radius < 1 { lerp } else { max-pool }`.
    /// Restructured to: compute lerp always (no branch), then decide max-pool
    /// only when `pool_radius >= 1`.  Max-pool uses `Iterator::fold` which
    /// the compiler can auto-vectorize.
    fn compute_and_update_spectrum(&mut self) {
        // Ensure tables are initialized
        let tables = match &mut self.tables {
            Some(t) => {
                t.rebuild_if_needed(Self::SPECTRUM_POINTS, self.last_sample_rate);
                t
            }
            None => {
                self.tables = Some(Box::new(AnalyzerTables::build(
                    Self::SPECTRUM_POINTS,
                    self.last_sample_rate,
                )));
                self.tables.as_mut().unwrap()
            }
        };

        let num_points = Self::SPECTRUM_POINTS;

        // 1. Apply Hann window and fill FFT input buffer
        {
            let fft_buf = &mut self.scratch.fft_buf;
            let hann = &tables.hann;
            let ring = &self.analyzer_buffer;
            let idx = self.analyzer_idx;
            // Unroll into two memcpy-friendly halves to help the compiler vectorize
            let tail_len = FFT_SIZE - idx;
            for i in 0..tail_len {
                fft_buf[i] = Complex::new(ring[idx + i] * hann[i], 0.0);
            }
            for i in 0..idx {
                fft_buf[tail_len + i] = Complex::new(ring[i] * hann[tail_len + i], 0.0);
            }
        }

        // 2. FFT
        if let Some(fft) = &self.fft_instance {
            fft.process(&mut self.scratch.fft_buf);
        }

        // 3. Convert to dB magnitude
        // `norm()` = sqrt(re²+im²); avoid sqrt by using norm_sqr then 0.5× the log.
        // dB = 20·log10(|X|/N) = 20·log10(|X|) − 20·log10(N)
        //    = FAST_DB_SCALE·(ln|X| − ln(N))
        //    = FAST_DB_SCALE·(0.5·ln(|X|²) − ln(N))
        {
            let norm_factor = (FFT_SIZE as f32) / 2.0;
            let ln_norm = norm_factor.ln(); // precompute: constant per block
            let raw_db = &mut self.scratch.raw_db;
            let fft_buf = &self.scratch.fft_buf;
            for i in 0..FFT_SIZE / 2 {
                // norm_sqr() avoids the sqrt inside norm()
                let ln_mag = 0.5 * fft_buf[i].norm_sqr().max(1e-30).ln() - ln_norm;
                raw_db[i] = (FAST_DB_SCALE * ln_mag).clamp(-100.0, 24.0);
            }
        }

        // 4. Map FFT bins → display points with temporal smoothing
        let flat = &mut self.scratch.flat;
        flat.clear();
        flat.reserve(num_points * 2); // no-op after first call (capacity already there)

        let raw_db = &self.scratch.raw_db;
        let half_bins = FFT_SIZE / 2;

        for i in 0..num_points {
            let bin_exact = tables.bins[i];
            let target_freq = tables.freqs[i];

            // Branchless: compute lerp unconditionally (2 array reads + 1 mul + 1 add)
            let bin_floor = bin_exact.floor() as usize;
            let bin_ceil = (bin_floor + 1).min(half_bins - 1);
            let frac = bin_exact.fract();
            let lerp_db = raw_db[bin_floor] + (raw_db[bin_ceil] - raw_db[bin_floor]) * frac;

            // Only do max-pool when we're in a dense frequency region
            let pool_radius = (bin_exact * 0.02) as usize; // floor via cast, no branch
            let current_db = if pool_radius >= 1 && bin_floor < half_bins {
                let start = bin_floor.saturating_sub(pool_radius);
                let end = (bin_floor + pool_radius).min(half_bins - 1);
                // fold is auto-vectorizable; avoids repeated bounds checks vs explicit loop
                raw_db[start..=end]
                    .iter()
                    .copied()
                    .fold(-100.0f32, f32::max)
            } else {
                lerp_db
            };

            // Fast attack / slow release smoothing
            let prev = self.spectrum_history[i];
            let smoothed = if current_db > prev {
                current_db.mul_add(0.8, prev * 0.2) // mul_add = fused FMA on supported targets
            } else {
                current_db.mul_add(0.1, prev * 0.9)
            };
            self.spectrum_history[i] = smoothed;

            flat.push(target_freq);
            flat.push(smoothed);
        }

        // 5. Publish — clone the flat buffer into a new Arc<Box<[f32]>>
        // This is the only remaining allocation per call (~2.4 kB for 300 points).
        // A lock-free ring of pre-allocated Arcs could eliminate it entirely,
        // but for a 33 ms update interval the single clone is negligible.
        self.spectrum_buffer = Arc::new(flat.clone().into_boxed_slice());
    }

    fn process_dsp(&mut self, buffer: &mut [f32]) {
        let current_base_gain = self.base_gain.get();
        let master_linear_gain = if current_base_gain.abs() > 0.01 {
            (10.0_f32).powf(current_base_gain / 20.0)
        } else {
            1.0
        };

        let mut active_bands: SmallVec<[&mut DigiParametricEQFilterNode; 8]> = SmallVec::new();
        for node in self.nodes.iter_mut() {
            if node.active.get() && !node.filter_type.get().is_off() {
                active_bands.push(node);
            }
        }

        if self.channels == 2 {
            for chunk in buffer.chunks_exact_mut(2) {
                let mut l = chunk[0] * master_linear_gain;
                let mut r = chunk[1] * master_linear_gain;
                for node in active_bands.iter_mut() {
                    l = node.channels[0].process(l, &node.coeff);
                    r = node.channels[1].process(r, &node.coeff);
                }
                chunk[0] = l;
                chunk[1] = r;
                if self.enable_spectrum_analyzer {
                    let mono_mix = (l + r) * 0.5;
                    self.analyzer_buffer[self.analyzer_idx] = mono_mix;
                    self.analyzer_idx = (self.analyzer_idx + 1) % FFT_SIZE;
                }
            }
        } else {
            for i in (0..buffer.len()).step_by(self.channels) {
                let mut mono_mix = 0.0;
                for channel in 0..self.channels {
                    let mut sample = buffer[i + channel] * master_linear_gain;
                    for node in active_bands.iter_mut() {
                        sample = node.channels[channel].process(sample, &node.coeff);
                    }
                    buffer[i + channel] = sample;
                    mono_mix += sample;
                }
                if self.enable_spectrum_analyzer {
                    self.analyzer_buffer[self.analyzer_idx] = mono_mix / (self.channels as f32);
                    self.analyzer_idx = (self.analyzer_idx + 1) % FFT_SIZE;
                }
            }
        }
    }
}

#[karbeat_macros::auto_param(on_change = "self.handle_side_effects(id)")]
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
            // Rebuild precomputed tables for the new sample rate
            self.tables = Some(Box::new(AnalyzerTables::build(
                Self::SPECTRUM_POINTS,
                sample_rate,
            )));
            // Ensure spectrum_history has the right length
            if self.spectrum_history.len() != Self::SPECTRUM_POINTS {
                self.spectrum_history = vec![-100.0f32; Self::SPECTRUM_POINTS].into_boxed_slice();
            }
            self.update_all_nodes();
            self.handle_side_effects(0);
        }
    }

    fn process(&mut self, buffer: &mut [f32], _context: &ProcessContext) {
        if self.channels == 0 || buffer.is_empty() {
            return;
        }
        let num_frames = buffer.len() / self.channels;
        self.process_dsp(buffer);
        if self.enable_spectrum_analyzer {
            self.samples_since_last_fft += num_frames;
            if self.samples_since_last_fft >= Self::FFT_TRIGGER_SAMPLES {
                self.samples_since_last_fft = 0;
                self.compute_and_update_spectrum();
            }
        }
    }
    fn reset(&mut self) {
        for node in &mut self.nodes {
            node.reset_state();
        }
        self.analyzer_buffer.fill(0.0);
        self.spectrum_history.fill(-100.0);

        self.compute_and_update_spectrum();
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
            "SET_MAGNITUDE_ACTIVE" => {
                if let Some(active) = payload.get("active").and_then(|v| v.as_bool()) {
                    self.enable_magnitude_curve = active;

                    // If the UI was just opened, calculate it immediately so it's ready!
                    if active {
                        self.magnitude_buffer = Arc::new(
                            self.compute_magnitude_response_flat(Self::MAGNITUDE_POINTS)
                                .into_boxed_slice(),
                        );
                    }
                }
                None
            }
            "SET_SPECTRUM_ACTIVE" => {
                if let Some(active) = payload.get("active").and_then(|v| v.as_bool()) {
                    if active && !self.enable_spectrum_analyzer {
                        // If toggled ON, reset buffers to prevent a visual glitch
                        // from stale audio jumping into new audio
                        self.analyzer_buffer.fill(0.0);
                        self.spectrum_history.fill(-100.0);
                        self.samples_since_last_fft = 0;
                    }
                    self.enable_spectrum_analyzer = active;
                }
                None
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
            "magnitude" => Some(ZeroCopyBuffer::Float32(self.magnitude_buffer.clone())),
            "spectrum" => Some(ZeroCopyBuffer::Float32(self.spectrum_buffer.clone())),
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
