// src/plugin/generator/karbeatzer.rs

use crate::core::plugin::{KarbeatGenerator, MidiEvent, MidiMessage};
use std::f32::consts::PI;

/// **Karbeatzer**, a synthesizer with 3 oscillator and minimal configuration for a modern synthesizer
pub struct Karbeatzer {
    sample_rate: u32,
    oscillators: [Oscillator; 3],
    active_voices: Vec<Voice>,
    gain: f32,
}

#[derive(Clone, Copy)]
struct Oscillator {
    waveform: Waveform,
    detune: f32, // In semitones
    mix: f32,    // 0.0 to 1.0
}

#[derive(Clone, Copy, PartialEq)]
enum Waveform {
    Sine = 0,
    Saw = 1,
    Square = 2,
    Triangle = 3,
}

impl From<f32> for Waveform {
    fn from(v: f32) -> Self {
        match v as u32 {
            0 => Waveform::Sine,
            1 => Waveform::Saw,
            2 => Waveform::Square,
            _ => Waveform::Triangle,
        }
    }
}

struct Voice {
    note: u8,
    velocity: u8,
    phase: [f32; 3], // Phase for each oscillator
    is_active: bool,
    // Simple envelope state could be added here
}

impl Karbeatzer {
    pub fn new() -> Self {
        Self {
            sample_rate: 48000,
            oscillators: [
                Oscillator { waveform: Waveform::Saw, detune: 0.0, mix: 1.0 },
                Oscillator { waveform: Waveform::Square, detune: 0.1, mix: 0.5 },
                Oscillator { waveform: Waveform::Sine, detune: -12.0, mix: 0.3 },
            ],
            active_voices: Vec::with_capacity(16),
            gain: 0.5,
        }
    }

    fn generate_sample(oscillators: &[Oscillator], sample_rate: u32, voice: &mut Voice) -> f32 {
        let mut sample = 0.0;
        let base_freq = 440.0 * 2.0_f32.powf((voice.note as f32 - 69.0) / 12.0);

        for (i, osc) in oscillators.iter().enumerate() {
            // Apply detune
            let freq = base_freq * 2.0_f32.powf(osc.detune / 12.0);
            let phase_inc = freq / sample_rate as f32;

            let osc_out = match osc.waveform {
                Waveform::Sine => (voice.phase[i] * 2.0 * PI).sin(),
                Waveform::Saw => 2.0 * voice.phase[i] - 1.0,
                Waveform::Square => if voice.phase[i] < 0.5 { 1.0 } else { -1.0 },
                Waveform::Triangle => 4.0 * (voice.phase[i] - 0.5).abs() - 1.0,
            };

            sample += osc_out * osc.mix;

            // Advance phase
            voice.phase[i] += phase_inc;
            if voice.phase[i] >= 1.0 {
                voice.phase[i] -= 1.0;
            }
        }

        sample * (voice.velocity as f32 / 127.0)
    }
}

impl KarbeatGenerator for Karbeatzer {
    fn name(&self) -> &str {
        "Karbeatzer"
    }

    fn prepare(&mut self, sample_rate: f32, _max_buffer_size: usize) {
        self.sample_rate = sample_rate as u32;
    }

    fn reset(&mut self) {
        self.active_voices.clear();
    }

    fn process(&mut self, output_buffer: &mut [f32], midi_events: &[MidiEvent]) {
        // Clear buffer first
        output_buffer.fill(0.0);
        
        let frames = output_buffer.len() / 2; // Stereo
        let mut event_idx = 0;

        for frame in 0..frames {
            // 1. Process MIDI Events for this exact sample frame
            while event_idx < midi_events.len() && midi_events[event_idx].sample_offset == frame {
                match midi_events[event_idx].data {
                    MidiMessage::NoteOn { key, velocity } => {
                        if velocity > 0 {
                            self.active_voices.push(Voice {
                                note: key,
                                velocity,
                                phase: [0.0; 3],
                                is_active: true,
                            });
                        } else {
                            // Note off (velocity 0)
                            self.active_voices.retain(|v| v.note != key);
                        }
                    },
                    MidiMessage::NoteOff { key } => {
                         self.active_voices.retain(|v| v.note != key);
                    },
                    _ => {}
                }
                event_idx += 1;
            }

            // 2. Synthesize
            let mut mix_l = 0.0;
            let mut mix_r = 0.0;

            for voice in self.active_voices.iter_mut() {
                let sample = Karbeatzer::generate_sample(&self.oscillators, self.sample_rate,  voice);
                // Simple mono-to-stereo for now
                mix_l += sample;
                mix_r += sample;
            }

            // 3. Write Output
            output_buffer[frame * 2] = mix_l * self.gain;
            output_buffer[frame * 2 + 1] = mix_r * self.gain;
        }
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        match id {
            // Global Gain
            0 => self.gain = value,
            
            // Osc 1
            10 => self.oscillators[0].waveform = Waveform::from(value),
            11 => self.oscillators[0].detune = value,
            12 => self.oscillators[0].mix = value,

            // Osc 2
            20 => self.oscillators[1].waveform = Waveform::from(value),
            21 => self.oscillators[1].detune = value,
            22 => self.oscillators[1].mix = value,

            // Osc 3
            30 => self.oscillators[2].waveform = Waveform::from(value),
            31 => self.oscillators[2].detune = value,
            32 => self.oscillators[2].mix = value,

            _ => {}
        }
    }

    fn get_parameter(&self, id: u32) -> f32 {
        match id {
            0 => self.gain,
            
            10 => self.oscillators[0].waveform as u32 as f32,
            11 => self.oscillators[0].detune,
            12 => self.oscillators[0].mix,

            20 => self.oscillators[1].waveform as u32 as f32,
            21 => self.oscillators[1].detune,
            22 => self.oscillators[1].mix,

            30 => self.oscillators[2].waveform as u32 as f32,
            31 => self.oscillators[2].detune,
            32 => self.oscillators[2].mix,

            _ => 0.0
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}