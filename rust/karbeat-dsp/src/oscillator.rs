use dasp::Frame;
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

// Import your universal parameter types and macros
use karbeat_macros::{karbeat_plugin, EnumParam};

// ============================================================================
// WAVEFORM ENUM
// ============================================================================

#[derive(Clone, Debug, Copy, PartialEq, Default, EnumParam, Deserialize, Serialize)]
#[repr(usize)]
pub enum Waveform {
    #[default]
    Sine = 0,
    Saw = 1,
    Square = 2,
    Triangle = 3,
    Noise = 4,
}

// Ensure the enum can convert back from the host's f32 automation signals
impl From<f32> for Waveform {
    fn from(v: f32) -> Self {
        match v.round() as i32 {
            0 => Waveform::Sine,
            1 => Waveform::Saw,
            2 => Waveform::Square,
            3 => Waveform::Triangle,
            4 => Waveform::Noise,
            _ => Waveform::Sine,
        }
    }
}

// ============================================================================
// OSCILLATOR
// ============================================================================

/// A Oscillator module component. you can use this to add oscillator to
/// your plugin audio plugin
#[derive(Clone, Debug)]
#[karbeat_plugin]
pub struct Oscillator {
    #[param(
        id = "waveform",
        name = "Waveform",
        group = "Oscillator",
        default = 0.0
    )]
    pub waveform: Waveform,

    #[param(id = "detune", name = "Detune", group = "Oscillator", min = -48.0, max = 48.0, default = 0.0, step = 0.2)]
    pub detune: f32,

    #[param(
        id = "phase_offset",
        name = "Phase Offset",
        group = "Oscillator",
        min = 0.0,
        max = 1.0,
        default = 0.0,
        step = 0.01
    )]
    pub phase_offset: f32,

    #[param(
        id = "mix",
        name = "Mix",
        group = "Oscillator",
        min = 0.0,
        max = 1.0,
        default = 1.0,
        step = 0.01
    )]
    pub mix: f32,

    #[param(
        id = "pulse_width",
        name = "Pulse Width",
        group = "Oscillator",
        min = 0.01,
        max = 0.99,
        default = 0.5,
        step = 0.01
    )]
    pub pulse_width: f32,
}

impl Default for Oscillator {
    fn default() -> Self {
        Self::new("Default Osc")
    }
}

impl Oscillator {
    /// Create a new Oscillator building block.
    /// We no longer need `id_start`! The macro handles unique hashing automatically.
    pub fn new(group: &str) -> Self {
        // The macro automatically generates base_default() to init the Params!
        let mut osc = Self::base_default();

        // Dynamically override the UI group string based on where it's used
        let group_string = group.to_string();
        osc.waveform.group = group_string.clone();
        osc.detune.group = group_string.clone();
        osc.phase_offset.group = group_string.clone();
        osc.mix.group = group_string.clone();
        osc.pulse_width.group = group_string;

        osc
    }

    /// Standard audio output using dasp frames
    pub fn output_wave(
        &self,
        out_block: &mut [f32],
        sample_rate: u32,
        channels: u8,
        base_freq: f64,
        current_phase: &mut f64,
    ) {
        let current_mix = self.mix.get();
        if current_mix <= 0.0 || out_block.is_empty() {
            return;
        }

        let current_detune = self.detune.get();
        let current_waveform = self.waveform.get();
        let current_pw = self.pulse_width.get() as f64;

        let actual_freq = base_freq * (2.0_f64).powf((current_detune as f64) / 12.0);
        let phase_inc = actual_freq / (sample_rate as f64);

        // Process frame by frame, dynamically adapting to channel count
        for frame in out_block.chunks_exact_mut(channels as usize) {
            let sample =
                Self::generate_aa_sample(current_waveform, current_pw, *current_phase, phase_inc);

            let final_sample = (sample * (current_mix as f64)) as f32;

            // Apply the sample to every channel in the frame (Mono = 1 loop, Stereo = 2 loops)
            for ch in frame.iter_mut() {
                *ch = ch.add_amp(final_sample);
            }

            *current_phase = (*current_phase + phase_inc).fract();
        }
    }

    /// Frequency Modulation (FM) output using dasp zip iterators
    #[allow(clippy::too_many_arguments)]
    pub fn output_wave_fm(
        &self,
        out_block: &mut [f32],
        mod_buffer: &[f32],
        fm_depth: f64,
        sample_rate: u32,
        channels: u8,
        base_freq: f64,
        current_phase: &mut f64,
    ) {
        let current_mix = self.mix.get();
        if current_mix <= 0.0 || out_block.is_empty() {
            return;
        }

        let current_detune = self.detune.get();
        let current_waveform = self.waveform.get();
        let current_pw = self.pulse_width.get() as f64;

        let actual_freq = base_freq * (2.0_f64).powf((current_detune as f64) / 12.0);
        let phase_inc = actual_freq / (sample_rate as f64);

        let channels_usize = channels as usize;

        // Zip the audio buffer chunks with the modulation buffer chunks
        let out_frames = out_block.chunks_exact_mut(channels_usize);
        let mod_frames = mod_buffer.chunks_exact(channels_usize);

        for (out_frame, mod_frame) in out_frames.zip(mod_frames) {
            // Assume the modulation signal is mono-compatible (take the first channel)
            let modulation = (mod_frame[0] as f64) * fm_depth;
            let modulated_phase = (*current_phase + modulation).rem_euclid(1.0);

            let sample =
                Self::generate_aa_sample(current_waveform, current_pw, modulated_phase, phase_inc);

            let final_sample = (sample * (current_mix as f64)) as f32;

            // Apply to all channels in the frame
            for ch in out_frame.iter_mut() {
                *ch = ch.add_amp(final_sample);
            }

            *current_phase = (*current_phase + phase_inc).fract();
        }
    }

    /// Generates a sample and applies Anti-Aliasing ONLY where mathematically required
    #[inline(always)]
    fn generate_aa_sample(waveform: Waveform, pulse_width: f64, phase: f64, phase_inc: f64) -> f64 {
        match waveform {
            // Continuous waves: No PolyBLEP needed!
            Waveform::Sine => (phase * TAU).sin(),
            Waveform::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
            Waveform::Noise => fastrand::f64() * 2.0 - 1.0,

            // Discontinuous waves: Require PolyBLEP
            Waveform::Saw => {
                let naive = 2.0 * phase - 1.0;
                // A rising saw jumps DOWN at phase 0, so we subtract the residual
                naive - Self::poly_blep(phase, phase_inc)
            }
            Waveform::Square => {
                let naive = if phase < pulse_width { 1.0 } else { -1.0 };

                // Jump 1: Upwards at phase 0
                let blep_up = Self::poly_blep(phase, phase_inc);

                // Jump 2: Downwards at phase = pulse_width
                let shifted_phase = (phase + 1.0 - pulse_width).fract();
                let blep_down = Self::poly_blep(shifted_phase, phase_inc);

                naive + blep_up - blep_down
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
}
