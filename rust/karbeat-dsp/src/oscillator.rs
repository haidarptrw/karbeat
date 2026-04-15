// oscillator.rs (part of karbeat_dsp library)

use std::f64::consts::TAU;
use dasp::{ Frame, slice };

// Import your new universal parameter types
use karbeat_macros::{ EnumParam };
use karbeat_plugin_types::{ AutoParams, ParamType, parameter::{ Param, ParameterSpec } };

// ============================================================================
// OSCILLATOR
// ============================================================================

#[derive(Clone, Debug)]
pub struct Oscillator {
    pub waveform: Param<Waveform>,
    pub detune: Param<f32>,
    pub phase_offset: Param<f32>,
    pub mix: Param<f32>,
    pub pulse_width: Param<f32>,
}

impl Default for Oscillator {
    fn default() -> Self {
        Self::new(0, "Default Osc")
    }
}

impl Oscillator {
    /// Create a new Oscillator building block.
    /// Assigns sequential IDs starting from `id_start` under the specified UI `group`.
    pub fn new(id_start: u32, group: &'static str) -> Self {
        Self {
            waveform: Param::new_enum(id_start, "Waveform", group, Waveform::Sine),
            detune: Param::new_float(id_start + 1, "Detune", group, 0.0, -48.0, 48.0, 0.2),
            phase_offset: Param::new_float(id_start + 2, "Phase Offset", group, 0.0, 0.0, 1.0, 0.01),
            mix: Param::new_float(id_start + 3, "Mix", group, 1.0, 0.0, 1.0, 0.01),
            pulse_width: Param::new_float(id_start + 4, "Pulse Width", group, 0.5, 0.01, 0.99, 0.01),
        }
    }

    /// Standard audio output using dasp frames
    pub fn output_wave(
        &self,
        out_block: &mut [f32],
        sample_rate: u32,
        channels: u8,
        base_freq: f64,
        current_phase: &mut f64
    ) {
        // 1. Fetch block-level parameters ONCE for performance and thread-safety
        let current_mix = self.mix.get();
        if current_mix <= 0.0 || out_block.is_empty() {
            return;
        }

        let current_detune = self.detune.get();
        let current_waveform = self.waveform.get();
        let current_pw = self.pulse_width.get() as f64;

        // 2. Calculate DSP constants
        let actual_freq = base_freq * (2.0_f64).powf((current_detune as f64) / 12.0);
        let phase_inc = actual_freq / (sample_rate as f64);

        // 3. Process the audio
        if channels == 2 {
            if let Some(frames) = slice::from_sample_slice_mut::<&mut [[f32; 2]], f32>(out_block) {
                for frame in frames {
                    let mut sample = Self::generate_raw_sample(
                        current_waveform,
                        current_pw,
                        *current_phase
                    );

                    // Anti-Aliasing
                    sample += Self::poly_blep(*current_phase, phase_inc);

                    let final_sample = (sample * (current_mix as f64)) as f32;

                    frame[0] = frame[0].add_amp(final_sample); // Left
                    frame[1] = frame[1].add_amp(final_sample); // Right

                    *current_phase = (*current_phase + phase_inc).fract();
                }
            }
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
        current_phase: &mut f64
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

        if channels == 2 {
            let out_frames = slice
                ::from_sample_slice_mut::<&mut [[f32; 2]], f32>(out_block)
                .unwrap_or_default();
            let mod_frames = slice
                ::from_sample_slice::<&[[f32; 2]], f32>(mod_buffer)
                .unwrap_or_default();

            for (out_frame, mod_frame) in out_frames.iter_mut().zip(mod_frames.iter()) {
                let modulation = (mod_frame[0] as f64) * fm_depth;
                let modulated_phase = (*current_phase + modulation).rem_euclid(1.0);

                let sample = Self::generate_raw_sample(
                    current_waveform,
                    current_pw,
                    modulated_phase
                );
                let final_sample = (sample * (current_mix as f64)) as f32;

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
    fn generate_raw_sample(waveform: Waveform, pulse_width: f64, phase: f64) -> f64 {
        match waveform {
            Waveform::Sine => (phase * TAU).sin(),
            Waveform::Saw => 2.0 * phase - 1.0,
            Waveform::Square => if phase < pulse_width { 1.0 } else { -1.0 }
            Waveform::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
            Waveform::Noise => fastrand::f64() * 2.0 - 1.0,
        }
    }
}

impl AutoParams for Oscillator {
    fn auto_get_parameter(&self, id: u32) -> Option<f32> {
        if self.waveform.id == id {
            return Some(self.waveform.get_base().to_f32());
        }
        if self.detune.id == id {
            return Some(self.detune.get_base().to_f32());
        }
        if self.phase_offset.id == id {
            return Some(self.phase_offset.get_base().to_f32());
        }
        if self.mix.id == id {
            return Some(self.mix.get_base().to_f32());
        }
        if self.pulse_width.id == id {
            return Some(self.pulse_width.get_base().to_f32());
        }
        None
    }

    fn auto_set_parameter(&mut self, id: u32, value: f32) {
        if self.waveform.id == id {
            self.waveform.set_base(value);
            return;
        }
        if self.detune.id == id {
            self.detune.set_base(value);
            return;
        }
        if self.phase_offset.id == id {
            self.phase_offset.set_base(value);
            return;
        }
        if self.mix.id == id {
            self.mix.set_base(value);
            return;
        }
        if self.pulse_width.id == id {
            self.pulse_width.set_base(value);
            return;
        }
    }

    fn auto_apply_automation(&mut self, id: u32, value: f32) {
        if self.waveform.id == id {
            self.waveform.apply_automation(value);
            return;
        }
        if self.detune.id == id {
            self.detune.apply_automation(value);
            return;
        }
        if self.phase_offset.id == id {
            self.phase_offset.apply_automation(value);
            return;
        }
        if self.mix.id == id {
            self.mix.apply_automation(value);
            return;
        }
        if self.pulse_width.id == id {
            self.pulse_width.apply_automation(value);
            return;
        }
    }

    fn auto_clear_automation(&mut self, id: u32) {
        if self.waveform.id == id {
            self.waveform.clear_automation();
            return;
        }
        if self.detune.id == id {
            self.detune.clear_automation();
            return;
        }
        if self.phase_offset.id == id {
            self.phase_offset.clear_automation();
            return;
        }
        if self.mix.id == id {
            self.mix.clear_automation();
            return;
        }
        if self.pulse_width.id == id {
            self.pulse_width.clear_automation();
            return;
        }
    }

    fn auto_get_parameter_specs(&self) -> Vec<ParameterSpec> {
        vec![
            self.waveform.to_spec(),
            self.detune.to_spec(),
            self.phase_offset.to_spec(),
            self.mix.to_spec(),
            self.pulse_width.to_spec()
        ]
    }

}

// ============================================================================
// WAVEFORM ENUM
// ============================================================================

#[derive(Clone, Debug, Copy, PartialEq, Default, EnumParam)]
#[repr(usize)]
pub enum Waveform {
    #[default]
    Sine = 0,
    Saw = 1,
    Square = 2,
    Triangle = 3,
    Noise = 4,
}
