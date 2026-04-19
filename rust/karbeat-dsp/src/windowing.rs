use std::f32::consts::PI;
use wide::f32x4;

pub enum Windowing {
    Hann,
}

/// A trait applied to slice types so they can process chunks of audio natively.
pub trait WindowableSlice {
    fn apply_hann(&mut self);
}

// ============================================================================
// MONO IMPLEMENTATION (1 Frame = 1 Float)
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

        // Process 4 Mono frames per CPU cycle
        for chunk in iter.by_ref() {
            let base_phase = f32x4::splat(i as f32) + offsets;
            let phase = base_phase * phase_step_v;
            
            // Hann formula: 0.5 * (1.0 - cos(phase))
            let mult = f32x4::splat(0.5) * (f32x4::splat(1.0) - phase.cos());

            let mut v = f32x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
            v *= mult;
            chunk.copy_from_slice(&v.to_array());
            
            i += 4;
        }

        // Remainder loop (1 to 3 frames)
        for sample in iter.into_remainder() {
            let phase = 2.0 * PI * (i as f32) / n_minus_1;
            *sample *= 0.5 * (1.0 - phase.cos());
            i += 1;
        }
    }
}

// ============================================================================
// STEREO IMPLEMENTATION (1 Frame = 2 Floats)
// ============================================================================
impl WindowableSlice for [[f32; 2]] {
    fn apply_hann(&mut self) {
        let num_samples = self.len();
        if num_samples <= 1 {
            return;
        }

        let n_min_1 = (num_samples - 1) as f32;
        let phase_step = 2.0 * PI / n_min_1;
        
        let phase_step_v = f32x4::splat(phase_step);
        
        // For stereo, 1 f32x4 holds exactly 2 frames: [L0, R0, L1, R1]
        // Frame 0 needs Phase 0 applied to both L and R.
        // Frame 1 needs Phase 1 applied to both L and R.
        let offsets = f32x4::new([0.0, 0.0, 1.0, 1.0]);

        // Process in chunks of 2 frames (4 floats)
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

        // Remainder loop (0 or 1 stereo frame)
        for frame in iter.into_remainder() {
            let phase = 2.0 * PI * (i as f32) / n_min_1;
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
}