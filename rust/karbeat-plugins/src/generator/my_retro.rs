// ====================================================
// MY RETRO GENERATOR
// Author: Haidar Wibowo
// ====================================================

use karbeat_plugin_api::traits::{MidiEvent, MidiMessage};
use karbeat_plugin_api::synth_base::{EnvelopeSettings, Oscillator, StandardSynthBase, SynthVoice};
use karbeat_plugin_api::wrapper::{RawSynthEngine, RawSynthWrapper};
use smallvec::SmallVec;

/// A generator that produces a retro-sounding synth sound.
#[derive(Clone)]
pub struct MyRetroEngine {
    oscillators: SmallVec<[Oscillator; 2]>,
}

impl Default for MyRetroEngine {
    fn default() -> Self {
        Self {
            oscillators: SmallVec::new(),
        }
    }
}

impl MyRetroEngine {
    pub fn generate_voice_block(
        oscillators: &[Oscillator; 2],
        sample_rate: f32,
        amp_envelope: &EnvelopeSettings,
        voice: &mut SynthVoice,
        buffer: &mut [f32],
    ) {
        let block_size = buffer.len();
        let base_freq = 440.0 * 2.0_f32.powf((voice.note as f32 - 69.0) / 12.0);
        let dt = 1.0 / sample_rate;
    }
}