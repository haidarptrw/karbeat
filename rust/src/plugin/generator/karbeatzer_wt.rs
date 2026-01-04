use crate::{core::project::plugin::KarbeatGenerator, plugin::wrapper::{RawSynthEngine, SynthWrapper}};

#[allow(dead_code)]

#[derive(Clone, Copy)]
struct Oscillator {
    waveform: Waveform,
    detune: f32,      // In semitones
    mix: f32,         // 0.0 to 1.0
    pulse_width: f32, // 0.0 to 1.0 (For Pulse/Square)
}

#[derive(Clone, Copy, PartialEq)]
enum Waveform {
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


/// Wavetable Synthesizer with modern algorithm that reduce the
/// anti-aliasing effect
#[derive(Clone)]
pub struct KarbeatzerWTEngine {
    oscillators: [Oscillator; 3],
}

impl Default for KarbeatzerWTEngine {
    fn default() -> Self {
        Self {
            oscillators: [
                Oscillator {
                    waveform: Waveform::Sine,
                    detune: 0.0,
                    mix: 1.0,
                    pulse_width: 0.5,
                },
                Oscillator {
                    waveform: Waveform::Square,
                    detune: 0.1,
                    mix: 0.5,
                    pulse_width: 0.5,
                },
                Oscillator {
                    waveform: Waveform::Sine,
                    detune: -12.0,
                    mix: 0.3,
                    pulse_width: 0.5,
                },
            ],
        }
    }
}

impl RawSynthEngine for KarbeatzerWTEngine {
    fn process(&mut self, base: &mut crate::plugin::synth_base::SynthBase, output: &mut [f32], midi: &[crate::core::project::plugin::MidiEvent]) {
        todo!()
    }

    fn set_custom_parameter(&mut self, id: u32, value: f32) {
        todo!()
    }

    fn get_custom_parameter(&self, id: u32) -> Option<f32> {
        todo!()
    }

    fn custom_default_parameters() -> std::collections::HashMap<u32, f32>
    where
        Self: Sized {
        todo!()
    }

    fn name() -> &'static str
    where
        Self: Sized {
        todo!()
    }
}

pub type KarbeatzerWaveTable = SynthWrapper<KarbeatzerWTEngine>;

pub fn create_karbeatzer_wt(sample_rate: Option<f32>) -> KarbeatzerWaveTable {
    SynthWrapper::new(KarbeatzerWTEngine::default(), sample_rate.unwrap_or(48000.0))
}