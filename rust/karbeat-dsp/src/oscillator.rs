// oscillator.rs (part of karbeat_dsp library)

use std::f64::consts::TAU;
use dasp::{ Frame, slice };
use derive_builder::Builder;

// ============================================================================
// OSCILLATOR
// ============================================================================

#[derive(Clone, Copy, Builder, Debug)]
#[builder(setter(into))]
pub struct Oscillator {
    #[builder(default = "Waveform::Sine")]
    pub waveform: Waveform,

    // Change these to custom to trigger clamping on input
    #[builder(default = "0.0", setter(custom))]
    pub detune: f32,

    #[builder(default = "0.0", setter(custom))]
    pub phase_offset: f32,

    #[builder(default = "1.0", setter(custom))]
    pub mix: f32, 

    #[builder(default = "0.5", setter(custom))]
    pub pulse_width: f32, 
}

impl OscillatorBuilder {
    pub fn detune(&mut self, value: f32) -> &mut Self {
        self.detune = Some(value.clamp(-48.0, 48.0));
        self
    }

    pub fn phase_offset(&mut self, value: f32) -> &mut Self {
        self.phase_offset = Some(value.clamp(0.0, 1.0));
        self
    }

    pub fn mix(&mut self, value: f32) -> &mut Self {
        self.mix = Some(value.clamp(0.0, 1.0));
        self
    }

    pub fn pulse_width(&mut self, value: f32) -> &mut Self {
        self.pulse_width = Some(value.clamp(0.01, 0.99));
        self
    }
}

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Waveform {
    Sine = 0,
    Saw = 1,
    Square = 2,
    Triangle = 3,
    Noise = 4,
}

impl From<f32> for Waveform {
    fn from(v: f32) -> Self {
        match v as u32 {
            0 => Waveform::Sine,
            1 => Waveform::Saw,
            2 => Waveform::Square,
            3 => Waveform::Triangle,
            _ => Waveform::Noise,
        }
    }
}

impl Oscillator {
    /// Standard audio output using dasp frames
    pub fn output_wave(
        &self,
        out_block: &mut [f32],
        sample_rate: u32,
        channels: u8,
        base_freq: f64,
        current_phase: &mut f64
    ) {
        if self.mix <= 0.0 || out_block.is_empty() {
            return;
        }

        let actual_freq = base_freq * (2.0_f64).powf((self.detune as f64) / 12.0);

        // DASP: Use internal helper for perfect precision phase steps
        let phase_inc = actual_freq / (sample_rate as f64);

        // DASP: Safely cast the flat slice into an array of Stereo Frames
        // This fails gracefully and costs 0 memory allocation
        if channels == 2 {
            if let Some(frames) = slice::from_sample_slice_mut::<&mut [[f32; 2]], f32>(out_block) {
                for frame in frames {
                    let mut sample = self.generate_raw_sample(*current_phase);

                    // Anti-Aliasing
                    sample += Self::poly_blep(*current_phase, phase_inc);

                    let final_sample = (sample * (self.mix as f64)) as f32;

                    // DASP: add_amp perfectly accumulates the audio into the existing buffer
                    frame[0] = frame[0].add_amp(final_sample); // Left
                    frame[1] = frame[1].add_amp(final_sample); // Right

                    // Fast phase wrapping
                    *current_phase = (*current_phase + phase_inc).fract();
                }
            }
        }
    }

    /// Frequency Modulation (FM) output using dasp zip iterators
    pub fn output_wave_fm(
        &self,
        out_block: &mut [f32],
        mod_buffer: &[f32],
        fm_depth: f64,
        sample_rate: u32,
        channels: u8,
        base_freq: f64,
        current_phase: &mut f64
    ) {
        if self.mix <= 0.0 || out_block.is_empty() {
            return;
        }

        let actual_freq = base_freq * (2.0_f64).powf((self.detune as f64) / 12.0);
        let phase_inc = actual_freq / (sample_rate as f64);

        if channels == 2 {
            // DASP: Cast both buffers to frames so we can zip them cleanly
            let out_frames = slice
                ::from_sample_slice_mut::<&mut [[f32; 2]], f32>(out_block)
                .unwrap();
            let mod_frames = slice::from_sample_slice::<&[[f32; 2]], f32>(mod_buffer).unwrap();

            for (out_frame, mod_frame) in out_frames.iter_mut().zip(mod_frames.iter()) {
                // Read the modulator's left channel to warp our phase
                let modulation = (mod_frame[0] as f64) * fm_depth;

                // rem_euclid safely wraps negative phase shifts caused by heavy FM
                let modulated_phase = (*current_phase + modulation).rem_euclid(1.0);

                let sample = self.generate_raw_sample(modulated_phase);
                let final_sample = (sample * (self.mix as f64)) as f32;

                out_frame[0] = out_frame[0].add_amp(final_sample);
                out_frame[1] = out_frame[1].add_amp(final_sample);

                *current_phase = (*current_phase + phase_inc).fract();
            }
        }
    }

    #[inline(always)]
    pub fn poly_blep(phase: f64, phase_inc: f64) -> f64 {
        if phase < phase_inc {
            let t = phase / phase_inc;
            2.0 * t - t * t - 1.0
        } else if phase > 1.0 - phase_inc {
            let t = (phase - 1.0) / phase_inc;
            t * t + 2.0 * t + 1.0
        } else {
            0.0
        }
    }

    /// Pure function to calculate the raw shape based on the current phase
    #[inline(always)]
    fn generate_raw_sample(&self, phase: f64) -> f64 {
        match self.waveform {
            Waveform::Sine => (phase * TAU).sin(),
            Waveform::Saw => 2.0 * phase - 1.0,
            Waveform::Square => if phase < (self.pulse_width as f64) { 1.0 } else { -1.0 }
            Waveform::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
            Waveform::Noise => fastrand::f64() * 2.0 - 1.0,
        }
    }
}
