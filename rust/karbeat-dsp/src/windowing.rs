use std::f32::consts::PI;
use wide::f32x4;

/// Collection of Windowing function
/// for Digital Signal Processing
pub enum Windowing {
    /// Hann Window apply the windowing function:
    /// ```tex
    /// w[i] = 0.5 * (1 - \cos(\frac{2*\pi*i}{N - 1} ))
    /// ```
    ///
    /// where i is the sample index,
    /// and N is the window size
    Hann,
}

/// A trait applied to slice types so they can process chunks of audio natively.
pub trait WindowableSlice {
    fn apply_hann(&mut self);
}

/// A trait applied to a single sample/frame.
pub trait WindowableSample: Sized {
    fn apply_hann(self, index: usize, window_size: usize) -> Self;
}

fn hann_weight(index: usize, window_size: usize) -> f32 {
    if window_size <= 1 {
        return 1.0;
    }

    let n_minus_1 = (window_size - 1) as f32;
    let phase = 2.0 * PI * (index as f32) / n_minus_1;
    0.5 * (1.0 - phase.cos())
}

impl WindowableSample for f32 {
    #[inline(always)]
    fn apply_hann(self, index: usize, window_size: usize) -> Self {
        self * hann_weight(index, window_size)
    }
}

impl WindowableSample for [f32; 2] {
    #[inline(always)]
    fn apply_hann(self, index: usize, window_size: usize) -> Self {
        let w = hann_weight(index, window_size);
        [self[0] * w, self[1] * w]
    }
}

// ============================================================================
// DYNAMIC (ON-THE-FLY) MONO IMPLEMENTATION (1 Frame = 1 Float)
// ============================================================================
impl WindowableSlice for [f32] {
    fn apply_hann(&mut self) {
        let num_samples = self.len();
        if num_samples <= 1 {
            return;
        }

        let n_minus_1 = (num_samples - 1) as f32;
        let phase_step = 2.0 * PI / n_minus_1;

        let phase_step_v = f32x4::splat(phase_step);
        let offsets = f32x4::new([0.0, 1.0, 2.0, 3.0]);

        let mut iter = self.chunks_exact_mut(4);
        let mut i = 0;

        for chunk in iter.by_ref() {
            let base_phase = f32x4::splat(i as f32) + offsets;
            let phase = base_phase * phase_step_v;

            let mult = f32x4::splat(0.5) * (f32x4::splat(1.0) - phase.cos());

            let mut v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
            v *= mult;
            chunk.copy_from_slice(&v.to_array());

            i += 4;
        }

        for sample in iter.into_remainder() {
            let phase = 2.0 * PI * (i as f32) / n_minus_1;
            *sample *= 0.5 * (1.0 - phase.cos());
            i += 1;
        }
    }
}

// ============================================================================
// DYNAMIC (ON-THE-FLY) STEREO IMPLEMENTATION (1 Frame = 2 Floats)
// ============================================================================
impl WindowableSlice for [[f32; 2]] {
    fn apply_hann(&mut self) {
        let num_samples = self.len();
        if num_samples <= 1 {
            return;
        }

        let n_minus_1 = (num_samples - 1) as f32;
        let phase_step = 2.0 * PI / n_minus_1;

        let phase_step_v = f32x4::splat(phase_step);
        let offsets = f32x4::new([0.0, 0.0, 1.0, 1.0]);

        let mut iter = self.chunks_exact_mut(2);
        let mut i = 0;

        for chunk in iter.by_ref() {
            let base_phase = f32x4::splat(i as f32) + offsets;
            let phase = base_phase * phase_step_v;

            let mult = f32x4::splat(0.5) * (f32x4::splat(1.0) - phase.cos());

            let mut v = f32x4::new([chunk[0][0], chunk[0][1], chunk[1][0], chunk[1][1]]);
            v *= mult;

            let arr = v.to_array();
            chunk[0][0] = arr[0];
            chunk[0][1] = arr[1];
            chunk[1][0] = arr[2];
            chunk[1][1] = arr[3];

            i += 2;
        }

        for frame in iter.into_remainder() {
            let phase = 2.0 * PI * (i as f32) / n_minus_1;
            let mult = 0.5 * (1.0 - phase.cos());
            frame[0] *= mult;
            frame[1] *= mult;
            i += 1;
        }
    }
}

impl Windowing {
    /// Applies the selected window to a mutable slice of audio data natively using SIMD.
    #[inline(always)]
    pub fn apply<T: ?Sized>(&self, buffer: &mut T)
    where
        T: WindowableSlice,
    {
        match self {
            Windowing::Hann => buffer.apply_hann(),
        }
    }

    /// Applies the selected window to one sample/frame.
    #[inline(always)]
    pub fn apply_single_sample<T>(&self, sample: T, index: usize, window_size: usize) -> T
    where
        T: WindowableSample,
    {
        match self {
            Windowing::Hann => sample.apply_hann(index, window_size),
        }
    }

    /// Generates a pre-computed array of weights for blazing fast application
    pub fn precompute(&self, size: usize) -> PrecomputedWindow {
        match self {
            Windowing::Hann => PrecomputedWindow::new_hann(size),
        }
    }
}

// ============================================================================
// PRE-COMPUTED WINDOWING (Lookup Table + SIMD Multiplication)
// ============================================================================

/// Holds a cached array of window weights to avoid evaluating `cos()` in tight loops.
#[derive(Clone, Debug)]
pub struct PrecomputedWindow {
    pub weights: Box<[f32]>,
}

impl PrecomputedWindow {
    pub fn new_hann(size: usize) -> Self {
        let mut weights = Vec::with_capacity(size);
        if size <= 1 {
            weights.resize(size, 1.0);
            return Self {
                weights: weights.into_boxed_slice(),
            };
        }

        let n_minus_1 = (size - 1) as f32;
        for i in 0..size {
            let phase = 2.0 * PI * (i as f32) / n_minus_1;
            weights.push(0.5 * (1.0 - phase.cos()));
        }

        Self {
            weights: weights.into_boxed_slice(),
        }
    }

    #[inline(always)]
    pub fn apply<T: ?Sized>(&self, buffer: &mut T)
    where
        T: ApplyPrecomputedWindow,
    {
        buffer.apply_precomputed(&self.weights);
    }
}

/// A trait applied to slice types so they can multiply against pre-computed weights.
pub trait ApplyPrecomputedWindow {
    fn apply_precomputed(&mut self, weights: &[f32]);
}

impl ApplyPrecomputedWindow for [f32] {
    #[inline(always)]
    fn apply_precomputed(&mut self, weights: &[f32]) {
        assert_eq!(
            self.len(),
            weights.len(),
            "Buffer and window size must match"
        );

        let mut iter_buf = self.chunks_exact_mut(4);
        let mut iter_wt = weights.chunks_exact(4);

        for (chunk, wt) in iter_buf.by_ref().zip(iter_wt.by_ref()) {
            let mut v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let w = f32x4::new([wt[0], wt[1], wt[2], wt[3]]);

            v *= w;
            chunk.copy_from_slice(&v.to_array());
        }

        for (sample, wt) in iter_buf
            .into_remainder()
            .iter_mut()
            .zip(iter_wt.remainder())
        {
            *sample *= *wt;
        }
    }
}

impl ApplyPrecomputedWindow for [[f32; 2]] {
    #[inline(always)]
    fn apply_precomputed(&mut self, weights: &[f32]) {
        assert_eq!(
            self.len(),
            weights.len(),
            "Buffer and window size must match"
        );

        let mut iter_buf = self.chunks_exact_mut(2);
        let mut iter_wt = weights.chunks_exact(2);

        for (chunk, wt) in iter_buf.by_ref().zip(iter_wt.by_ref()) {
            let mut v = f32x4::new([chunk[0][0], chunk[0][1], chunk[1][0], chunk[1][1]]);
            // Duplicate the weights for L and R channels: [w0, w0, w1, w1]
            let w = f32x4::new([wt[0], wt[0], wt[1], wt[1]]);

            v *= w;

            let arr = v.to_array();
            chunk[0][0] = arr[0];
            chunk[0][1] = arr[1];
            chunk[1][0] = arr[2];
            chunk[1][1] = arr[3];
        }

        for (frame, wt) in iter_buf
            .into_remainder()
            .iter_mut()
            .zip(iter_wt.remainder())
        {
            frame[0] *= *wt;
            frame[1] *= *wt;
        }
    }
}
