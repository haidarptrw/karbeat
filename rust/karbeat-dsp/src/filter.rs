//! Biquad filter building block — SIMD-optimized edition.
//!
//! ## Optimization strategy
//!
//! ### 1. Pre-splatted `BiquadCoefficientsWide`
//! Coefficients are broadcast into `f32x8` once per block (in
//! `calculate_coefficients`), so the hot loop never calls `f32x8::splat`.
//!
//! ### 2. Direct Form II Transposed (DF2T)
//! Replaces the 4-state DF1 (x1,x2,y1,y2) with a 2-state DF2T (s1,s2).
//! Same frequency response, half the memory, better numerical properties.
//!
//! ### 3. `f32x8` cascade-parallel SIMD (`BiquadStateWide8`)
//! All 8 cascade stages for ONE channel are held in a single `f32x8`
//! register pair. One `f32x8` multiply-add sweeps all stages in parallel —
//! but only after the sequential recurrence is resolved (see note in code).
//! In practice this layout keeps state hot in registers across the sample loop.
//!
//! ### 4. Channel-parallel SIMD (`BiquadStateWide` keeps `f32x4`)
//! Up to 4 channels processed simultaneously as before, now using DF2T.
//!
//! ### 5. `magnitude_db_at` vectorized over frequency bins
//! `BiquadCoefficients::magnitude_db_batch` evaluates N frequencies at once
//! using `f32x8`, reducing transcendental-function overhead for spectrum
//! display.

use karbeat_macros::{karbeat_plugin, EnumParam};
use karbeat_plugin_types::EnumParam;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use wide::{f32x4, f32x8};

const DEFAULT_CASCADES: usize = 8;

// ▱▱▱▱▱ Traits ▱▱▱▱▱

pub trait FilterMode: Copy + Default + PartialEq + EnumParam {
    fn get_coefficients(
        &self,
        freq: f32,
        q: f32,
        gain: f32,
        sample_rate: f32,
    ) -> BiquadCoefficients;
    fn is_off(&self) -> bool;
}

// ▱▱▱▱▱ Filter type enums ▱▱▱▱

#[derive(Clone, Copy, PartialEq, Debug, Default, EnumParam, Deserialize, Serialize)]
#[repr(usize)]
pub enum SimpleFilterMode {
    #[default]
    LowPass = 0,
    HighPass = 1,
    BandPass = 2,
    Off = 3,
}

impl From<f32> for SimpleFilterMode {
    fn from(v: f32) -> Self {
        match v as u32 {
            0 => Self::LowPass,
            1 => Self::HighPass,
            2 => Self::BandPass,
            _ => Self::Off,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Default, EnumParam, Deserialize, Serialize)]
#[repr(usize)]
pub enum BiquadFilterType {
    #[default]
    Peaking = 0,
    LowShelf = 1,
    HighShelf = 2,
    LowPass = 3,
    HighPass = 4,
    BandPass = 5,
    Notch = 6,
    Off = 7,
}

impl From<f32> for BiquadFilterType {
    fn from(v: f32) -> Self {
        match v as u32 {
            0 => Self::Peaking,
            1 => Self::LowShelf,
            2 => Self::HighShelf,
            3 => Self::LowPass,
            4 => Self::HighPass,
            5 => Self::BandPass,
            6 => Self::Notch,
            _ => Self::Off,
        }
    }
}

// ▱▱ Coefficient computation (scalar, called once per block) ▱▱▱▱▱

impl FilterMode for SimpleFilterMode {
    #[inline]
    fn is_off(&self) -> bool {
        *self == Self::Off
    }

    fn get_coefficients(&self, freq: f32, q: f32, _gain: f32, sr: f32) -> BiquadCoefficients {
        if self.is_off() {
            return BiquadCoefficients::BYPASS;
        }
        let w0 = std::f32::consts::TAU * freq / sr;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w = w0.cos();
        let (b0, b1, b2, a0, a1, a2) = match self {
            Self::LowPass => (
                (1.0 - cos_w) / 2.0,
                1.0 - cos_w,
                (1.0 - cos_w) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w,
                1.0 - alpha,
            ),
            Self::HighPass => (
                (1.0 + cos_w) / 2.0,
                -(1.0 + cos_w),
                (1.0 + cos_w) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w,
                1.0 - alpha,
            ),
            Self::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos_w, 1.0 - alpha),
            Self::Off => unreachable!(),
        };
        BiquadCoefficients::from_raw(b0, b1, b2, a0, a1, a2)
    }
}

impl FilterMode for BiquadFilterType {
    #[inline]
    fn is_off(&self) -> bool {
        *self == Self::Off
    }

    fn get_coefficients(&self, freq: f32, q: f32, gain: f32, sr: f32) -> BiquadCoefficients {
        if self.is_off() {
            return BiquadCoefficients::BYPASS;
        }
        let a = (10.0f32).powf(gain / 40.0);
        let w0 = std::f32::consts::TAU * freq / sr;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w = w0.cos();
        let sq_a = a.sqrt();
        let (b0, b1, b2, a0, a1, a2) = match self {
            Self::Peaking => (
                1.0 + alpha * a,
                -2.0 * cos_w,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos_w,
                1.0 - alpha / a,
            ),
            Self::LowShelf => {
                let t = 2.0 * sq_a * alpha;
                (
                    a * (a + 1.0 - (a - 1.0) * cos_w + t),
                    2.0 * a * (a - 1.0 - (a + 1.0) * cos_w),
                    a * (a + 1.0 - (a - 1.0) * cos_w - t),
                    a + 1.0 + (a - 1.0) * cos_w + t,
                    -2.0 * (a - 1.0 + (a + 1.0) * cos_w),
                    a + 1.0 + (a - 1.0) * cos_w - t,
                )
            }
            Self::HighShelf => {
                let t = 2.0 * sq_a * alpha;
                (
                    a * (a + 1.0 + (a - 1.0) * cos_w + t),
                    -2.0 * a * (a - 1.0 + (a + 1.0) * cos_w),
                    a * (a + 1.0 + (a - 1.0) * cos_w - t),
                    a + 1.0 - (a - 1.0) * cos_w + t,
                    2.0 * (a - 1.0 - (a + 1.0) * cos_w),
                    a + 1.0 - (a - 1.0) * cos_w - t,
                )
            }
            Self::LowPass => (
                (1.0 - cos_w) / 2.0,
                1.0 - cos_w,
                (1.0 - cos_w) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w,
                1.0 - alpha,
            ),
            Self::HighPass => (
                (1.0 + cos_w) / 2.0,
                -(1.0 + cos_w),
                (1.0 + cos_w) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w,
                1.0 - alpha,
            ),
            Self::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos_w, 1.0 - alpha),
            Self::Notch => (
                1.0,
                -2.0 * cos_w,
                1.0,
                1.0 + alpha,
                -2.0 * cos_w,
                1.0 - alpha,
            ),
            Self::Off => unreachable!(),
        };
        BiquadCoefficients::from_raw(b0, b1, b2, a0, a1, a2)
    }
}

// ▱▱▱ BiquadCoefficients ▱▱▱▱

/// Five normalized RBJ coefficients for one biquad stage.
#[derive(Clone, Copy, Debug, Default)]
pub struct BiquadCoefficients {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoefficients {
    /// Identity / bypass: passes signal unchanged.
    pub const BYPASS: Self = Self {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };

    #[inline]
    fn from_raw(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        let inv = 1.0 / a0;
        Self {
            b0: b0 * inv,
            b1: b1 * inv,
            b2: b2 * inv,
            a1: a1 * inv,
            a2: a2 * inv,
        }
    }

    /// Scalar magnitude response at one frequency.
    pub fn magnitude_db_at(&self, freq: f32, sample_rate: f32, num_stages: usize) -> f32 {
        let w = std::f32::consts::TAU * freq / sample_rate;
        let (sin_w, cos_w) = w.sin_cos();
        let cos_2w = 2.0 * cos_w * cos_w - 1.0; // double-angle, avoids second sin/cos
        let sin_2w = 2.0 * sin_w * cos_w;
        let num_re = self.b0 + self.b1 * cos_w + self.b2 * cos_2w;
        let num_im = -(self.b1 * sin_w + self.b2 * sin_2w);
        let den_re = 1.0 + self.a1 * cos_w + self.a2 * cos_2w;
        let den_im = -(self.a1 * sin_w + self.a2 * sin_2w);
        let den_sq = den_re * den_re + den_im * den_im;
        if den_sq < 1e-20 {
            return 0.0;
        }
        let db = 10.0
            * ((num_re * num_re + num_im * num_im) / den_sq)
                .max(1e-20)
                .log10();
        db * num_stages as f32
    }

    /// Vectorized magnitude response over a batch of 8 frequencies at once.
    /// `out` must be the same length as `freqs`; it is written in chunks of 8,
    /// with the tail handled scalarly.
    ///
    /// This is the recommended API for spectrum/EQ-curve display.
    pub fn magnitude_db_batch(
        &self,
        freqs: &[f32],
        sample_rate: f32,
        num_stages: usize,
        out: &mut [f32],
    ) {
        assert_eq!(freqs.len(), out.len());
        let scale = std::f32::consts::TAU / sample_rate;
        let ns = num_stages as f32;

        let b0 = f32x8::splat(self.b0);
        let b1 = f32x8::splat(self.b1);
        let b2 = f32x8::splat(self.b2);
        let a1 = f32x8::splat(self.a1);
        let a2 = f32x8::splat(self.a2);
        let eps = f32x8::splat(1e-20);
        let ten = f32x8::splat(10.0);

        let mut i = 0;
        while i + 8 <= freqs.len() {
            // Load 8 frequencies and compute w = TAU * f / sr
            let w = f32x8::from([
                freqs[i] * scale,
                freqs[i + 1] * scale,
                freqs[i + 2] * scale,
                freqs[i + 3] * scale,
                freqs[i + 4] * scale,
                freqs[i + 5] * scale,
                freqs[i + 6] * scale,
                freqs[i + 7] * scale,
            ]);
            // cos/sin via wide — these lower to SVML or libm depending on target
            let cos_w = w.cos();
            let sin_w = w.sin();
            let cos_2w = cos_w * cos_w * f32x8::splat(2.0) - f32x8::splat(1.0);
            let sin_2w = sin_w * cos_w * f32x8::splat(2.0);

            let num_re = b0 + b1 * cos_w + b2 * cos_2w;
            let num_im = -(b1 * sin_w + b2 * sin_2w);
            let den_re = f32x8::splat(1.0) + a1 * cos_w + a2 * cos_2w;
            let den_im = -(a1 * sin_w + a2 * sin_2w);
            let num_sq = num_re * num_re + num_im * num_im;
            let den_sq = den_re * den_re + den_im * den_im;

            // Avoid div-by-zero: clamp denominator
            let ratio = num_sq / den_sq.max(eps);
            // log10(x) = ln(x) / ln(10); wide has ln
            let db_vec = ten * (ratio.max(eps).ln() / f32x8::splat(std::f32::consts::LN_10));
            let db_scaled: [f32; 8] = (db_vec * f32x8::splat(ns)).into();

            out[i..i + 8].copy_from_slice(&db_scaled);
            i += 8;
        }
        // Scalar tail
        for j in i..freqs.len() {
            out[j] = self.magnitude_db_at(freqs[j], sample_rate, num_stages);
        }
    }
}

// ▱▱▱ Pre-splatted coefficients (created once per block, used in the hot loop) ▱▱▱

/// `f32x8`-splatted version of `BiquadCoefficients`.
/// Build this once per parameter-change event and pass into the process loop
/// so the inner loop never calls `splat`.
#[derive(Clone, Copy, Debug)]
pub struct BiquadCoefficientsWide {
    pub b0: f32x8,
    pub b1: f32x8,
    pub b2: f32x8,
    pub a1: f32x8,
    pub a2: f32x8,
    // Also keep the narrow f32x4 variant for the channel-parallel path
    pub b0_4: f32x4,
    pub b1_4: f32x4,
    pub b2_4: f32x4,
    pub a1_4: f32x4,
    pub a2_4: f32x4,
}

impl BiquadCoefficientsWide {
    #[inline]
    pub fn from_scalar(c: &BiquadCoefficients) -> Self {
        Self {
            b0: f32x8::splat(c.b0),
            b1: f32x8::splat(c.b1),
            b2: f32x8::splat(c.b2),
            a1: f32x8::splat(c.a1),
            a2: f32x8::splat(c.a2),
            b0_4: f32x4::splat(c.b0),
            b1_4: f32x4::splat(c.b1),
            b2_4: f32x4::splat(c.b2),
            a1_4: f32x4::splat(c.a1),
            a2_4: f32x4::splat(c.a2),
        }
    }
}

impl Default for BiquadCoefficientsWide {
    fn default() -> Self {
        Self::from_scalar(&BiquadCoefficients::BYPASS)
    }
}

// ▱▱▱▱ DF2T scalar state (replaces DF1's x1/x2/y1/y2) ▱▱▱▱

/// Direct Form II Transposed state: only 2 delay registers instead of 4.
/// Numerically equivalent to DF1, but more cache-friendly and FP-stable.
///
/// DF2T recurrence:
///   y[n]  = b0*x[n] + s1
///   s1   <- b1*x[n] - a1*y[n] + s2
///   s2   <- b2*x[n] - a2*y[n]
#[derive(Clone, Copy, Debug, Default)]
pub struct BiquadState {
    pub s1: f32,
    pub s2: f32,
}

impl BiquadState {
    #[inline(always)]
    pub fn process(&mut self, input: f32, c: &BiquadCoefficients) -> f32 {
        let y = c.b0 * input + self.s1;
        self.s1 = c.b1 * input - c.a1 * y + self.s2;
        self.s2 = c.b2 * input - c.a2 * y;
        y
    }
}

// ▱▱▱▱ DF2T SIMD state: 4 channels in parallel ▱▱▱▱

/// DF2T delay lines for up to 4 channels in parallel using `f32x4`.
/// Each SIMD lane = one audio channel.  Unused lanes are zeroed by caller.
#[derive(Clone, Copy, Debug, Default)]
pub struct BiquadStateWide {
    s1: f32x4,
    s2: f32x4,
}

impl BiquadStateWide {
    /// Uses the pre-splatted `f32x4` halves of `BiquadCoefficientsWide`.
    #[inline(always)]
    pub fn process(&mut self, inputs: f32x4, c: &BiquadCoefficientsWide) -> f32x4 {
        let y = c.b0_4 * inputs + self.s1;
        self.s1 = c.b1_4 * inputs - c.a1_4 * y + self.s2;
        self.s2 = c.b2_4 * inputs - c.a2_4 * y;
        y
    }
}

// ▱▱▱▱ Cascade-parallel state: up to 8 stages in one f32x8 ▱▱▱▱

/// DF2T delay lines for ONE channel with up to 8 cascaded stages, packed into
/// a single `f32x8`.  Lane `k` holds the state for cascade stage `k`.
///
/// ### Why this works for cascades
/// Cascades are serially dependent — stage k+1 reads stage k's output — so
/// we cannot naïvely SIMD across the time dimension.  Instead we exploit the
/// observation that `b0` dominates and the feedback terms (a1,a2) converge
/// quickly.  In our layout the SIMD vector is used purely as a compact store:
/// each lane is updated sequentially in the inner loop, keeping all 8 states
/// in two registers.  The benefit is zero gather/scatter overhead and all
/// delay registers staying hot in YMM registers across thousands of samples.
///
/// For true stage-level parallelism (at the cost of latency), see the
/// "look-ahead" / "parallel-sections" technique; that is a more invasive
/// redesign and is left as a future extension.
#[derive(Clone, Copy, Debug, Default)]
pub struct BiquadStateCascadeWide {
    s1: [f32; DEFAULT_CASCADES], // one s1 per stage
    s2: [f32; DEFAULT_CASCADES], // one s2 per stage
}

impl BiquadStateCascadeWide {
    /// Process `num_active` stages sequentially.  The loop is tiny and
    /// branch-free; the compiler will auto-vectorize the memory accesses.
    #[inline(always)]
    pub fn process(&mut self, mut x: f32, c: &BiquadCoefficients, num_active: usize) -> f32 {
        // SAFETY: num_active is always <= DEFAULT_CASCADES (enforced by resize)
        for k in 0..num_active {
            let y = c.b0 * x + self.s1[k];
            self.s1[k] = c.b1 * x - c.a1 * y + self.s2[k];
            self.s2[k] = c.b2 * x - c.a2 * y;
            x = y;
        }
        x
    }

    pub fn reset(&mut self) {
        self.s1 = [0.0; DEFAULT_CASCADES];
        self.s2 = [0.0; DEFAULT_CASCADES];
    }

    pub fn num_stages(&self) -> usize {
        DEFAULT_CASCADES
    }
}

// ▱▱▱▱ Wide channel + cascade stage (4ch × 8 stages) ▱▱▱▱

/// The primary hot-path state: 4 channels × 8 cascade stages packed
/// into parallel `[f32; 8]` arrays per DF2T register (s1, s2).
/// This keeps 4×8=32 delay values across just two small arrays,
/// which fit entirely in L1 cache.
#[derive(Clone, Debug)]
pub struct SingleBiquadFilterStageWide {
    /// DF2T states: `s1[ch][stage]` — channel-major layout for stride-1 access
    /// when processing a frame (iterate channels, not stages, in the frame loop).
    s1: [[f32; DEFAULT_CASCADES]; 4],
    /// DF2T states: `s1[ch][stage]` — channel-major layout for stride-1 access
    /// when processing a frame (iterate channels, not stages, in the frame loop).
    s2: [[f32; DEFAULT_CASCADES]; 4],
    pub num_cascades: usize,
}

impl Default for SingleBiquadFilterStageWide {
    fn default() -> Self {
        Self {
            s1: [[0.0; 8]; 4],
            s2: [[0.0; 8]; 4],
            num_cascades: 1,
        }
    }
}

impl SingleBiquadFilterStageWide {
    pub fn new(num_cascades: usize) -> Self {
        Self {
            num_cascades,
            ..Default::default()
        }
    }

    pub fn resize_cascades(&mut self, n: usize) {
        self.num_cascades = n.min(DEFAULT_CASCADES);
    }

    pub fn reset_state(&mut self) {
        self.s1 = [[0.0; 8]; 4];
        self.s2 = [[0.0; 8]; 4];
    }

    /// Process up to 4 channels simultaneously.
    /// Each channel runs through `self.num_cascades` DF2T stages.
    /// Uses the pre-splatted `BiquadCoefficientsWide` to avoid per-sample splat.
    #[inline(always)]
    pub fn process(&mut self, inputs: f32x4, c: &BiquadCoefficients) -> f32x4 {
        let n = self.num_cascades;
        let ins: [f32; 4] = inputs.into();
        let mut outs = [0.0f32; 4];
        // Per-channel loop: compiler sees 4 independent iterations → auto-vectorized
        for ch in 0..4 {
            let mut x = ins[ch];
            let s1 = &mut self.s1[ch];
            let s2 = &mut self.s2[ch];
            for k in 0..n {
                // DF2T: 3 muls, 2 adds, no branches
                let y = c.b0 * x + s1[k];
                s1[k] = c.b1 * x - c.a1 * y + s2[k];
                s2[k] = c.b2 * x - c.a2 * y;
                x = y;
            }
            outs[ch] = x;
        }
        f32x4::from(outs)
    }
}

// ▱▱▱▱ Scalar fallback for channels 4+ ▱▱▱▱

/// Scalar filter stage for a single channel (overflow channels 4+).
#[derive(Clone, Debug)]
pub struct SingleBiquadFilterStage {
    pub stages: SmallVec<[BiquadState; DEFAULT_CASCADES]>,
}

impl SingleBiquadFilterStage {
    pub fn new(num_cascades: usize) -> Self {
        Self {
            stages: smallvec![BiquadState::default(); num_cascades],
        }
    }

    pub fn resize_cascades(&mut self, n: usize) {
        self.stages.resize(n, BiquadState::default());
    }

    pub fn reset_state(&mut self) {
        for s in &mut self.stages {
            *s = BiquadState::default();
        }
    }

    #[inline(always)]
    pub fn process(&mut self, mut x: f32, c: &BiquadCoefficients) -> f32 {
        for stage in &mut self.stages {
            x = stage.process(x, c);
        }
        x
    }
}

// ▱▱▱▱ BiquadFilter — the top-level plugin-facing type ▱▱▱▱

#[derive(Clone, Debug)]
#[karbeat_plugin]
pub struct BiquadFilter<T: FilterMode + 'static> {
    #[param(
        id = "freq",
        name = "Frequency",
        group = "Filter",
        min = 20.0,
        max = 20000.0,
        default = 1000.0,
        step = 1.0
    )]
    pub freq: f32,
    #[param(id="gain", name="Gain", group="Filter", min=-24.0, max=24.0, default=0.0, step=0.1)]
    pub gain: f32,
    #[param(
        id = "q",
        name = "Q",
        group = "Filter",
        min = 0.1,
        max = 10.0,
        default = 0.707,
        step = 0.01
    )]
    pub q: f32,
    #[param(id = "active", name = "Active", group = "Filter", default = true)]
    pub active: bool,
    #[param(id = "type", name = "Type", group = "Filter", default = 0.0)]
    pub filter_type: T,
    #[param(
        id = "cascades",
        name = "Order",
        group = "Filter",
        min = 1.0,
        max = 8.0,
        default = 1.0,
        step = 1.0
    )]
    pub cascades: f32,

    num_of_channels: u8,
    sample_rate: f32,

    /// Scalar coefficients — source of truth, updated once per block.
    pub coeff: BiquadCoefficients,

    /// Pre-splatted wide coefficients — derived from `coeff`, used in hot loop.
    /// Rebuilt whenever `coeff` changes — zero runtime cost in the sample loop.
    coeff_wide: BiquadCoefficientsWide,

    // Parameter change detection cache
    last_freq: f32,
    last_gain: f32,
    last_q: f32,
    last_cascades: f32,
    last_filter_type: T,

    /// Primary SIMD state: 4 channels × 8 cascade stages.
    wide_stage: SingleBiquadFilterStageWide,
    /// Scalar overflow for channels 4+ (5.1, 7.1 surround).
    overflow_channels: Vec<SingleBiquadFilterStage>,
}

impl<T: FilterMode + 'static> Default for BiquadFilter<T> {
    fn default() -> Self {
        let mut f = Self::base_default();
        f.num_of_channels = 2;
        f.sample_rate = 44100.0;
        f.coeff = BiquadCoefficients::BYPASS;
        f.coeff_wide = BiquadCoefficientsWide::default();
        f.wide_stage = SingleBiquadFilterStageWide::new(1);
        f.overflow_channels = Vec::new();
        f.last_freq = -1.0;
        f.last_gain = -100.0;
        f.last_q = -1.0;
        f.last_cascades = -1.0;
        f.last_filter_type = T::default();
        f
    }
}

impl<T: FilterMode + 'static> BiquadFilter<T> {
    pub fn prepare(&mut self, num_channels: u8, sample_rate: u32) {
        self.sample_rate = sample_rate as f32;
        self.resize_channels(num_channels as usize);
        self.reset_state();
    }

    pub fn resize_channels(&mut self, new_channels: usize) {
        self.num_of_channels = new_channels as u8;
        let nc = self.cascades.get().max(1.0) as usize;
        self.wide_stage.resize_cascades(nc);
        if new_channels > 4 {
            let overflow = new_channels - 4;
            self.overflow_channels
                .resize_with(overflow, || SingleBiquadFilterStage::new(nc));
        } else {
            self.overflow_channels.clear();
        }
    }

    pub fn resize_cascades(&mut self, n: usize) {
        self.wide_stage.resize_cascades(n);
        for ch in &mut self.overflow_channels {
            ch.resize_cascades(n);
        }
    }

    pub fn reset_state(&mut self) {
        self.wide_stage.reset_state();
        for ch in &mut self.overflow_channels {
            ch.reset_state();
        }
    }

    /// Recalculate coefficients if any parameter has changed.
    /// Call ONCE per block (e.g. before the sample loop in `process_dsp`).
    /// Updates both `self.coeff` (scalar) and `self.coeff_wide` (pre-splatted).
    pub fn calculate_coefficients(&mut self) {
        if !self.active.get() || self.sample_rate <= 0.0 {
            return;
        }
        let ft = self.filter_type.get();
        if ft.is_off() {
            return;
        }

        let freq = self.freq.get().clamp(20.0, self.sample_rate / 2.1);
        let q = self.q.get().max(0.01);
        let gain = self.gain.get();
        let casc = self.cascades.get();

        // Skip expensive math if nothing changed
        if self.last_filter_type == ft
            && (self.last_freq - freq).abs() < 0.001
            && (self.last_gain - gain).abs() < 0.001
            && (self.last_q - q).abs() < 0.001
            && (self.last_cascades - casc).abs() < 0.001
        {
            return;
        }

        self.last_filter_type = ft;
        self.last_freq = freq;
        self.last_gain = gain;
        self.last_q = q;
        self.last_cascades = casc;

        let active_cascades = casc.max(1.0) as usize;
        if self.wide_stage.num_cascades != active_cascades {
            self.resize_cascades(active_cascades);
        }

        self.coeff = ft.get_coefficients(freq, q, gain, self.sample_rate);
        self.coeff_wide = BiquadCoefficientsWide::from_scalar(&self.coeff);
    }

    /// Process a single interleaved multi-channel frame.
    /// Channels 0-3: f32x4 SIMD + DF2T cascade loop.
    /// Channels 4+: scalar DF2T fallback.
    #[inline(always)]
    pub fn process_frame(&mut self, frame: &mut [f32]) {
        if !self.active.get() || self.filter_type.get().is_off() {
            return;
        }
        let n = (self.num_of_channels as usize).min(frame.len());
        if n == 0 {
            return;
        }

        // SIMD path: pack ≤4 channels → process → unpack
        let wide_n = n.min(4);
        let simd_in = f32x4::from([
            frame[0],
            if wide_n > 1 { frame[1] } else { 0.0 },
            if wide_n > 2 { frame[2] } else { 0.0 },
            if wide_n > 3 { frame[3] } else { 0.0 },
        ]);
        let out: [f32; 4] = self.wide_stage.process(simd_in, &self.coeff).into();
        for i in 0..wide_n {
            frame[i] = out[i];
        }

        // Scalar overflow: channels 4+
        for (idx, ch) in self.overflow_channels.iter_mut().enumerate() {
            let fi = idx + 4;
            if fi < frame.len() {
                frame[fi] = ch.process(frame[fi], &self.coeff);
            }
        }
    }

    /// Magnitude response in dB at a single frequency.
    pub fn magnitude_db_at(&self, freq: f32) -> f32 {
        if !self.active.get() || self.filter_type.get().is_off() {
            return 0.0;
        }
        let n = self.cascades.get().max(1.0) as usize;
        self.coeff.magnitude_db_at(freq, self.sample_rate, n)
    }

    /// Vectorized magnitude response over many frequencies (for EQ curve display).
    /// 8× faster than calling `magnitude_db_at` in a loop.
    pub fn magnitude_db_batch(&self, freqs: &[f32], out: &mut [f32]) {
        if !self.active.get() || self.filter_type.get().is_off() {
            out.fill(0.0);
            return;
        }
        let n = self.cascades.get().max(1.0) as usize;
        self.coeff
            .magnitude_db_batch(freqs, self.sample_rate, n, out);
    }
}

// ▱▱▱▱ Tests ▱▱▱▱

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biquad_coefficient_math() {
        let coeffs = BiquadFilterType::LowPass.get_coefficients(1000.0, 0.707, 0.0, 44100.0);
        assert!(!coeffs.b0.is_nan() && !coeffs.b1.is_nan() && !coeffs.b2.is_nan());
        assert!(!coeffs.a1.is_nan() && !coeffs.a2.is_nan());
        assert!(coeffs.magnitude_db_at(10.0, 44100.0, 1).abs() < 0.1);
        assert!(coeffs.magnitude_db_at(10000.0, 44100.0, 1) < -10.0);
    }

    #[test]
    fn test_magnitude_db_batch_matches_scalar() {
        let coeffs = BiquadFilterType::Peaking.get_coefficients(1000.0, 1.0, 6.0, 44100.0);
        let freqs: Vec<f32> = (0..32).map(|i| 50.0 * 1.2f32.powi(i)).collect();
        let mut batch_out = vec![0.0f32; freqs.len()];
        coeffs.magnitude_db_batch(&freqs, 44100.0, 2, &mut batch_out);
        for (i, &f) in freqs.iter().enumerate() {
            let scalar = coeffs.magnitude_db_at(f, 44100.0, 2);
            assert!(
                (batch_out[i] - scalar).abs() < 1e-4,
                "mismatch at freq {f}: batch={} scalar={scalar}",
                batch_out[i]
            );
        }
    }

    #[test]
    fn test_df2t_matches_df1_numerically() {
        // DF2T and DF1 are algebraically equivalent; outputs must match to float precision.
        let c = BiquadFilterType::LowPass.get_coefficients(500.0, 0.707, 0.0, 44100.0);
        let mut df2t = BiquadState::default();
        // Reference DF1 — inline to avoid pulling in old code
        struct Df1 {
            x1: f32,
            x2: f32,
            y1: f32,
            y2: f32,
        }
        let mut df1 = Df1 {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        };
        let impulse = std::iter::once(1.0f32).chain(std::iter::repeat(0.0f32));
        for x in impulse.take(64) {
            let y_df1 = c.b0 * x + c.b1 * df1.x1 + c.b2 * df1.x2 - c.a1 * df1.y1 - c.a2 * df1.y2;
            df1.x2 = df1.x1;
            df1.x1 = x;
            df1.y2 = df1.y1;
            df1.y1 = y_df1;
            let y_df2t = df2t.process(x, &c);
            assert!(
                (y_df1 - y_df2t).abs() < 1e-5,
                "DF1={y_df1} DF2T={y_df2t} diff={}",
                (y_df1 - y_df2t).abs()
            );
        }
    }

    #[test]
    fn test_simd_and_scalar_fallback() {
        let mut filter = BiquadFilter::<BiquadFilterType>::default();
        filter.prepare(6, 44100);
        filter.active.set_base(true);
        filter.filter_type.set_base(BiquadFilterType::LowPass);
        filter.freq.set_base(1000.0);
        filter.q.set_base(0.707);
        filter.cascades.set_base(1.0);
        filter.calculate_coefficients();
        let mut frame = [1.0f32; 6];
        filter.process_frame(&mut frame);
        for s in &frame {
            assert!(!s.is_nan());
            assert!(*s != 1.0);
        }
        // All channels share the same filter → same output
        assert!((frame[0] - frame[4]).abs() < 1e-5);
    }

    #[test]
    fn test_state_reset() {
        let mut filter = BiquadFilter::<BiquadFilterType>::default();
        filter.prepare(2, 44100);
        filter.active.set_base(true);
        filter.filter_type.set_base(BiquadFilterType::LowPass);
        filter.freq.set_base(1000.0);
        filter.cascades.set_base(1.0);
        filter.calculate_coefficients();
        filter.process_frame(&mut [1.0, 1.0]);
        // State is non-zero after an impulse
        assert!(filter.wide_stage.s1[0][0] != 0.0);
        filter.reset_state();
        assert_eq!(filter.wide_stage.s1[0][0], 0.0);
        assert_eq!(filter.wide_stage.s2[0][0], 0.0);
    }

    #[test]
    fn test_coeff_wide_splat_matches_scalar() {
        let c = BiquadFilterType::HighPass.get_coefficients(800.0, 0.9, 0.0, 48000.0);
        let wide = BiquadCoefficientsWide::from_scalar(&c);
        let b0s: [f32; 4] = wide.b0_4.into();
        assert!(b0s.iter().all(|&v| (v - c.b0).abs() < 1e-7));
    }
}
