//! Biquad filter building block. A general purpose, reusable biquad filter
//! Can be used for things such as Equalizer or Compressor

use karbeat_macros::{karbeat_plugin, EnumParam};
use karbeat_plugin_types::EnumParam;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use wide::f32x4;

const DEFAULT_CASCADES: usize = 8;

pub trait FilterMode: Copy + Default + PartialEq + EnumParam {
    /// Get coefficients based on type and current audio parameters
    fn get_coefficients(
        &self,
        freq: f32,
        q: f32,
        gain: f32,
        sample_rate: f32,
    ) -> BiquadCoefficients;
    /// Fast check if the filter is completely bypassed
    fn is_off(&self) -> bool;
}

/// Options of Simple filter often used (LPF, HPF, and BPF)
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
            0 => SimpleFilterMode::LowPass,
            1 => SimpleFilterMode::HighPass,
            2 => SimpleFilterMode::BandPass,
            _ => SimpleFilterMode::Off,
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
            0 => BiquadFilterType::Peaking,
            1 => BiquadFilterType::LowShelf,
            2 => BiquadFilterType::HighShelf,
            3 => BiquadFilterType::LowPass,
            4 => BiquadFilterType::HighPass,
            5 => BiquadFilterType::BandPass,
            6 => BiquadFilterType::Notch,
            _ => BiquadFilterType::Off,
        }
    }
}

impl FilterMode for SimpleFilterMode {
    #[inline]
    fn is_off(&self) -> bool {
        *self == SimpleFilterMode::Off
    }

    fn get_coefficients(
        &self,
        freq: f32,
        q: f32,
        _gain: f32,
        sample_rate: f32,
    ) -> BiquadCoefficients {
        if self.is_off() {
            return BiquadCoefficients::default();
        }

        let w0 = (2.0 * std::f32::consts::PI * freq) / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let (b0, b1, b2, a0, a1, a2) = match self {
            SimpleFilterMode::LowPass => (
                (1.0 - cos_w0) / 2.0,
                1.0 - cos_w0,
                (1.0 - cos_w0) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            SimpleFilterMode::HighPass => (
                (1.0 + cos_w0) / 2.0,
                -(1.0 + cos_w0),
                (1.0 + cos_w0) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            SimpleFilterMode::BandPass => {
                (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha)
            }
            SimpleFilterMode::Off => unreachable!(),
        };

        let inv_a0 = 1.0 / a0;
        BiquadCoefficients {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
        }
    }
}

impl FilterMode for BiquadFilterType {
    #[inline]
    fn is_off(&self) -> bool {
        *self == BiquadFilterType::Off
    }

    fn get_coefficients(
        &self,
        freq: f32,
        q: f32,
        gain: f32,
        sample_rate: f32,
    ) -> BiquadCoefficients {
        if self.is_off() {
            return BiquadCoefficients::default();
        }

        let a = (10.0f32).powf(gain / 40.0);
        let w0 = (2.0 * std::f32::consts::PI * freq) / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let (b0, b1, b2, a0, a1, a2) = match self {
            BiquadFilterType::Peaking => (
                1.0 + alpha * a,
                -2.0 * cos_w0,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos_w0,
                1.0 - alpha / a,
            ),
            BiquadFilterType::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                (
                    a * (a + 1.0 - (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
                    2.0 * a * (a - 1.0 - (a + 1.0) * cos_w0),
                    a * (a + 1.0 - (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
                    a + 1.0 + (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
                    -2.0 * (a - 1.0 + (a + 1.0) * cos_w0),
                    a + 1.0 + (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
                )
            }
            BiquadFilterType::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                (
                    a * (a + 1.0 + (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
                    -2.0 * a * (a - 1.0 + (a + 1.0) * cos_w0),
                    a * (a + 1.0 + (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
                    a + 1.0 - (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
                    2.0 * (a - 1.0 - (a + 1.0) * cos_w0),
                    a + 1.0 - (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
                )
            }
            BiquadFilterType::LowPass => (
                (1.0 - cos_w0) / 2.0,
                1.0 - cos_w0,
                (1.0 - cos_w0) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            BiquadFilterType::HighPass => (
                (1.0 + cos_w0) / 2.0,
                -(1.0 + cos_w0),
                (1.0 + cos_w0) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            BiquadFilterType::BandPass => {
                (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha)
            }
            BiquadFilterType::Notch => (
                1.0,
                -2.0 * cos_w0,
                1.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            BiquadFilterType::Off => unreachable!(),
        };

        let inv_a0 = 1.0 / a0;
        BiquadCoefficients {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
        }
    }
}

/// The 5 RBJ Cookbook coefficients for a single biquad stage
#[derive(Clone, Copy, Debug, Default)]
pub struct BiquadCoefficients {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoefficients {
    /// Returns the magnitude response in dB at a given frequency for N cascaded stages.
    ///
    /// Uses the analytical H(e^jw) evaluation via the direct DFT form:
    ///   H(z) = (b0 + b1*z^-1 + b2*z^-2) / (1 + a1*z^-1 + a2*z^-2)
    /// evaluated at z = e^(j*w), where w = 2*pi*freq/sample_rate.
    /// The single-stage power ratio is multiplied by `num_stages` in the dB domain
    /// to reflect cascaded magnitude accumulation.
    ///
    /// Returns 0.0 if the denominator magnitude is degenerate (near zero).
    pub fn magnitude_db_at(&self, freq: f32, sample_rate: f32, num_stages: usize) -> f32 {
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
        single_stage_db * num_stages as f32
    }
}

/// The historical delay lines (x[n-1], x[n-2], y[n-1], y[n-2]) for ONE channel in ONE stage
#[derive(Clone, Copy, Debug, Default)]
pub struct BiquadState {
    pub x1: f32,
    pub x2: f32,
    pub y1: f32,
    pub y2: f32,
}

impl BiquadState {
    #[inline(always)]
    pub fn process(&mut self, input: f32, coeffs: &BiquadCoefficients) -> f32 {
        let output = coeffs.b0 * input + coeffs.b1 * self.x1 + coeffs.b2 * self.x2
            - coeffs.a1 * self.y1
            - coeffs.a2 * self.y2;

        // Shift delay lines
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }
}

/// A filter chain for a SINGLE channel that handles N cascaded biquad stages
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

    /// Expands or shrinks the number of cascaded stages
    pub fn resize_cascades(&mut self, num_cascades: usize) {
        self.stages.resize(num_cascades, BiquadState::default());
    }

    pub fn reset_state(&mut self) {
        for stage in &mut self.stages {
            *stage = BiquadState::default();
        }
    }

    #[inline(always)]
    pub fn process(&mut self, mut input: f32, coeffs: &BiquadCoefficients) -> f32 {
        // Cascade the signal through all configured biquad stages
        for stage in &mut self.stages {
            input = stage.process(input, coeffs);
        }
        input
    }
}

// ======================================
// SIMD-wide biquad types
// Process up to 4 channels simultaneously using f32x4.
// All channels share the same coefficients — suitable for multi-channel EQ.
// ======================================

/// Delay lines for up to 4 channels simultaneously using f32x4 SIMD.
/// Each SIMD lane corresponds to one audio channel (L=0, R=1, ...).
/// Unused lanes must be zeroed by the caller; their outputs are discarded.
#[derive(Clone, Copy, Debug, Default)]
pub struct BiquadStateWide {
    x1: f32x4,
    x2: f32x4,
    y1: f32x4,
    y2: f32x4,
}

impl BiquadStateWide {
    /// Broadcasts scalar coefficients across all SIMD lanes and evaluates
    /// the Direct Form I biquad equation for all channels simultaneously.
    #[inline(always)]
    pub fn process(&mut self, inputs: f32x4, coeffs: &BiquadCoefficients) -> f32x4 {
        let b0 = f32x4::splat(coeffs.b0);
        let b1 = f32x4::splat(coeffs.b1);
        let b2 = f32x4::splat(coeffs.b2);
        let a1 = f32x4::splat(coeffs.a1);
        let a2 = f32x4::splat(coeffs.a2);

        let output = b0 * inputs + b1 * self.x1 + b2 * self.x2 - a1 * self.y1 - a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = inputs;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }
}

/// A filter chain for up to 4 channels simultaneously with N cascaded SIMD stages.
/// Note: all channels share one set of biquad coefficients.
#[derive(Clone, Debug, Default)]
pub struct SingleBiquadFilterStageWide {
    pub stages: SmallVec<[BiquadStateWide; DEFAULT_CASCADES]>,
}

impl SingleBiquadFilterStageWide {
    pub fn new(num_cascades: usize) -> Self {
        Self {
            stages: smallvec![BiquadStateWide::default(); num_cascades],
        }
    }

    /// Expands or shrinks the number of cascaded stages
    pub fn resize_cascades(&mut self, num_cascades: usize) {
        self.stages.resize(num_cascades, BiquadStateWide::default());
    }

    pub fn reset_state(&mut self) {
        for stage in &mut self.stages {
            *stage = BiquadStateWide::default();
        }
    }

    /// Cascades the SIMD signal through all configured biquad stages
    #[inline(always)]
    pub fn process(&mut self, mut inputs: f32x4, coeffs: &BiquadCoefficients) -> f32x4 {
        for stage in &mut self.stages {
            inputs = stage.process(inputs, coeffs);
        }
        inputs
    }
}

/// Biquad filter implemented from RBJ EQ Cookbook
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

    #[param(id = "gain", name = "Gain", group = "Filter", min = -24.0, max = 24.0, default = 0.0, step = 0.1)]
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

    // Internal state ignored by parameter UI
    num_of_channels: u8,
    sample_rate: f32,
    coeff: BiquadCoefficients,

    // Cache values to avoid recalculating when parameters are the same
    last_freq: f32,
    last_gain: f32,
    last_q: f32,
    last_cascades: f32,
    last_filter_type: T,

    // ======================================
    // SIMD channel storage
    // wide_stage covers channels 0..min(num_of_channels, 4) simultaneously.
    // overflow_channels is a scalar fallback for channels 4+ (rare in practice).
    // ======================================
    wide_stage: SingleBiquadFilterStageWide,
    overflow_channels: Vec<SingleBiquadFilterStage>,
}

impl<T: FilterMode + 'static> Default for BiquadFilter<T> {
    fn default() -> Self {
        let mut filter = Self::base_default();
        filter.num_of_channels = 2;
        filter.sample_rate = 44100.0;
        filter.coeff = BiquadCoefficients::default();
        filter.wide_stage = SingleBiquadFilterStageWide::new(1);
        filter.overflow_channels = Vec::new();

        filter.last_freq = -1.0;
        filter.last_gain = -100.0;
        filter.last_q = -1.0;
        filter.last_cascades = -1.0;
        filter.last_filter_type = T::default();
        filter
    }
}

impl<T: FilterMode + 'static> BiquadFilter<T> {
    /// Initializes the filter for the current audio stream format
    pub fn prepare(&mut self, num_channels: u8, sample_rate: u32) {
        self.sample_rate = sample_rate as f32;
        self.resize_channels(num_channels as usize);
        self.reset_state();
    }

    /// Expands or shrinks the filter to handle N discrete channels.
    /// Channels 0-3 are covered by the SIMD wide stage; channels 4+ use scalar fallback.
    pub fn resize_channels(&mut self, new_channels: usize) {
        self.num_of_channels = new_channels as u8;
        let active_cascades = self.cascades.get().max(1.0) as usize;
        self.wide_stage.resize_cascades(active_cascades);
        if new_channels > 4 {
            self.overflow_channels.resize_with(new_channels - 4, || {
                SingleBiquadFilterStage::new(active_cascades)
            });
        } else {
            self.overflow_channels.clear();
        }
    }

    /// Expands or shrinks the filter's processing order (cascades)
    pub fn resize_cascades(&mut self, new_cascades: usize) {
        self.wide_stage.resize_cascades(new_cascades);
        for ch in &mut self.overflow_channels {
            ch.resize_cascades(new_cascades);
        }
    }

    /// Clears all historical delay lines (prevents clicking when jumping playhead)
    pub fn reset_state(&mut self) {
        self.wide_stage.reset_state();
        for ch in &mut self.overflow_channels {
            ch.reset_state();
        }
    }

    /// Recalculates the coefficients based on current UI parameters.
    /// Call this ONCE per block (or per sample if interpolating).
    pub fn calculate_coefficients(&mut self) {
        if !self.active.get() || self.sample_rate <= 0.0 {
            return;
        }

        let filter_mode = self.filter_type.get();
        if filter_mode.is_off() {
            return;
        }

        let freq = self.freq.get().clamp(20.0, self.sample_rate / 2.1); // Nyquist safety
        let q = self.q.get().max(0.01);
        let gain = self.gain.get();
        let cascades = self.cascades.get();

        // 🚀 THE FIX: Only do the math if the parameters actually changed!
        if self.last_filter_type == filter_mode
            && (self.last_freq - freq).abs() < 0.001
            && (self.last_gain - gain).abs() < 0.001
            && (self.last_q - q).abs() < 0.001
            && (self.last_cascades - cascades).abs() < 0.001
        {
            return; // Skip the expensive math!
        }

        // Update the cache
        self.last_filter_type = filter_mode;
        self.last_freq = freq;
        self.last_gain = gain;
        self.last_q = q;
        self.last_cascades = cascades;

        // Dynamically resize cascades if the param changed
        let active_cascades = cascades.max(1.0) as usize;
        if self.wide_stage.stages.len() != active_cascades {
            self.resize_cascades(active_cascades);
        }

        self.coeff = filter_mode.get_coefficients(freq, q, gain, self.sample_rate);
    }

    /// Process a single multi-channel frame of audio.
    /// Channels 0-3 are processed simultaneously via f32x4 SIMD.
    /// Channels 4+ fall back to the scalar path (rare in practice).
    #[inline(always)]
    pub fn process_frame(&mut self, frame: &mut [f32]) {
        if !self.active.get() || self.filter_type.get().is_off() {
            return;
        }

        let n = (self.num_of_channels as usize).min(frame.len());
        if n == 0 {
            return;
        }

        // ======================================
        // SIMD path: pack up to 4 channels into f32x4, process, unpack
        // ======================================
        let wide_n = n.min(4);
        let simd_input = f32x4::from([
            frame[0],
            if wide_n > 1 { frame[1] } else { 0.0 },
            if wide_n > 2 { frame[2] } else { 0.0 },
            if wide_n > 3 { frame[3] } else { 0.0 },
        ]);
        let out: [f32; 4] = self.wide_stage.process(simd_input, &self.coeff).into();
        for i in 0..wide_n {
            frame[i] = out[i];
        }

        // ======================================
        // Scalar fallback: channels 4+ (e.g. 5.1, 7.1 surround)
        // ======================================
        for (ch_idx, scalar_ch) in self.overflow_channels.iter_mut().enumerate() {
            let frame_idx = ch_idx + 4;
            if frame_idx < frame.len() {
                frame[frame_idx] = scalar_ch.process(frame[frame_idx], &self.coeff);
            }
        }
    }

    /// Returns the magnitude response in dB at the given frequency.
    /// Reads the filter's own cached coefficients, cascade count, and active state.
    ///
    /// - Returns 0.0 dB if the filter is inactive or the filter type is `Off`.
    /// - Relies on `calculate_coefficients()` having been called beforehand to
    ///   ensure `self.coeff` reflects the current parameter values.
    pub fn magnitude_db_at(&self, freq: f32) -> f32 {
        if !self.active.get() || self.filter_type.get().is_off() {
            return 0.0;
        }

        let num_stages = self.cascades.get().max(1.0) as usize;
        self.coeff
            .magnitude_db_at(freq, self.sample_rate, num_stages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biquad_coefficient_math() {
        let filter_mode = BiquadFilterType::LowPass;
        let freq = 1000.0;
        let q = 0.707;
        let gain = 0.0;
        let sample_rate = 44100.0;

        let coeffs = filter_mode.get_coefficients(freq, q, gain, sample_rate);

        // Ensure coefficients are calculated and not NaNs
        assert!(!coeffs.b0.is_nan());
        assert!(!coeffs.b1.is_nan());
        assert!(!coeffs.b2.is_nan());
        assert!(!coeffs.a1.is_nan());
        assert!(!coeffs.a2.is_nan());

        // For a lowpass filter, response at DC (freq -> 0) should be around 0 dB (gain of 1.0)
        let mag_db_dc = coeffs.magnitude_db_at(10.0, sample_rate, 1);
        assert!(mag_db_dc.abs() < 0.1); // ~0 dB at low frequency

        // Response above cutoff should be lower
        let mag_db_high = coeffs.magnitude_db_at(10000.0, sample_rate, 1);
        assert!(mag_db_high < -10.0); // significantly attenuated
    }

    #[test]
    fn test_biquad_simd_and_scalar_fallback() {
        let mut filter = BiquadFilter::<BiquadFilterType>::default();
        // Set to 6 channels to test SIMD (4 channels) + Scalar (2 channels)
        filter.prepare(6, 44100);
        filter.active.set_base(true);
        filter.filter_type.set_base(BiquadFilterType::LowPass);
        filter.freq.set_base(1000.0);
        filter.q.set_base(0.707);
        filter.cascades.set_base(1.0);

        filter.calculate_coefficients();

        let mut frame = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        filter.process_frame(&mut frame);

        // Output should not be 1.0 anymore since it's filtered
        // Also it should not be NaN
        for sample in frame.iter() {
            assert!(!sample.is_nan());
            assert!(*sample != 1.0);
        }

        // Since input is same for all channels, and all channels use same filter,
        // the SIMD channels and Scalar channels should have the exact same output.
        let val_simd = frame[0];
        let val_scalar = frame[4];
        assert!((val_simd - val_scalar).abs() < 1e-5);
    }

    #[test]
    fn test_biquad_state_reset() {
        let mut filter = BiquadFilter::<BiquadFilterType>::default();
        filter.prepare(2, 44100);
        filter.active.set_base(true);
        filter.filter_type.set_base(BiquadFilterType::LowPass);
        filter.freq.set_base(1000.0);
        filter.cascades.set_base(1.0);

        filter.calculate_coefficients();

        // Feed an impulse to change state
        let mut frame = [1.0, 1.0];
        filter.process_frame(&mut frame);

        // State should now be non-zero
        let x1: [f32; 4] = filter.wide_stage.stages[0].x1.into();
        assert!(x1[0] != 0.0);

        // Reset state
        filter.reset_state();

        // State should be zero
        let x1_reset: [f32; 4] = filter.wide_stage.stages[0].x1.into();
        let y1_reset: [f32; 4] = filter.wide_stage.stages[0].y1.into();
        assert_eq!(x1_reset[0], 0.0);
        assert_eq!(y1_reset[0], 0.0);
    }
}
