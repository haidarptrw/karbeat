// src/plugin/generator/karbeatzer.rs

use std::{collections::HashMap, f32::consts::PI};

use crate::core::project::plugin::{KarbeatGenerator, MidiEvent, MidiMessage};
use karbeat_macros::karbeat_synth;

// ============================================================================
// SYNTH-SPECIFIC TYPES
// ============================================================================

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

// ============================================================================
// KARBEATZER STRUCT (common fields injected by macro)
// ============================================================================

/// **Karbeatzer**, an enhanced subtractive synthesizer
#[karbeat_synth]
pub struct Karbeatzer {
    oscillators: [Oscillator; 3],
    drive: f32,
}

impl Karbeatzer {
    pub fn new(sample_rate: Option<f32>) -> Self {
        Self {
            oscillators: [
                Oscillator {
                    waveform: Waveform::Saw,
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
            drive: 0.0,

            // Common fields (injected by macro)
            sample_rate: sample_rate.unwrap_or(48000.0),
            active_voices: Vec::with_capacity(16),
            voice_buffer: Vec::with_capacity(512),
            gain: 0.5,
            filter: SynthFilter::default(),
            amp_envelope: AdsrSettings::default(),
        }
    }

    /// Renders a block of audio for a single voice (static to avoid borrow issues).
    fn generate_voice_block(
        oscillators: &[Oscillator; 3],
        sample_rate: f32,
        amp_envelope: &AdsrSettings,
        voice: &mut SynthVoice,
        buffer: &mut [f32],
    ) {
        let block_size = buffer.len();
        let base_freq = 440.0 * 2.0_f32.powf((voice.note as f32 - 69.0) / 12.0);
        let dt = 1.0 / sample_rate;

        // Pre-calculate phase increments
        let mut phase_incs = [0.0; 3];
        for (i, osc) in oscillators.iter().enumerate() {
            let freq = base_freq * 2.0_f32.powf(osc.detune / 12.0);
            phase_incs[i] = freq / sample_rate;
        }

        for frame in 0..block_size {
            // Advance envelope
            let env_level = voice.advance_envelope(dt, amp_envelope);

            if !voice.is_active {
                buffer[frame] = 0.0;
                continue;
            }

            let velocity_gain = voice.velocity as f32 / 127.0;
            let current_gain = velocity_gain * env_level;

            let mut sample_accum = 0.0;

            // Oscillator summation
            for (i, osc) in oscillators.iter().enumerate() {
                let phase = voice.phase[i];

                let osc_out = match osc.waveform {
                    Waveform::Sine => (phase * 2.0 * PI).sin(),
                    Waveform::Saw => 2.0 * phase - 1.0,
                    Waveform::Square => {
                        if phase < osc.pulse_width {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    Waveform::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
                    Waveform::Noise => fastrand::f32() * 2.0 - 1.0,
                };

                sample_accum += osc_out * osc.mix;

                // Advance phase
                voice.phase[i] += phase_incs[i];
                if voice.phase[i] >= 1.0 {
                    voice.phase[i] -= 1.0;
                }
            }

            buffer[frame] = sample_accum * current_gain;
        }
    }
}

impl KarbeatGenerator for Karbeatzer {
    fn name(&self) -> &str {
        "Karbeatzer"
    }

    fn prepare(&mut self, sample_rate: f32, max_buffer_size: usize) {
        self.sample_rate = sample_rate;
        if self.voice_buffer.len() < max_buffer_size {
            self.voice_buffer.resize(max_buffer_size, 0.0);
        }
    }

    fn reset(&mut self) {
        self.base_reset();
    }

    fn process(&mut self, output_buffer: &mut [f32], midi_events: &[MidiEvent]) {
        output_buffer.fill(0.0);

        let total_frames = output_buffer.len() / 2;
        let mut current_frame = 0;
        let mut event_idx = 0;

        while current_frame < total_frames {
            let next_event_frame = if event_idx < midi_events.len() {
                midi_events[event_idx].sample_offset as usize
            } else {
                total_frames
            };

            let end_frame = next_event_frame.min(total_frames);
            let block_len = end_frame - current_frame;

            if block_len > 0 {
                let out_slice = &mut output_buffer[current_frame * 2..end_frame * 2];

                for voice in self.active_voices.iter_mut() {
                    if !voice.is_active {
                        continue;
                    }

                    let scratch = &mut self.voice_buffer[0..block_len];
                    Karbeatzer::generate_voice_block(
                        &self.oscillators,
                        self.sample_rate,
                        &self.amp_envelope,
                        voice,
                        scratch,
                    );

                    // Mix mono voice into stereo output
                    for (i, &sample) in scratch.iter().enumerate() {
                        out_slice[i * 2] += sample; // L
                        out_slice[i * 2 + 1] += sample; // R
                    }
                }

                // Apply filter
                self.filter.process(out_slice, self.sample_rate);

                // Apply drive
                if self.drive > 0.0 {
                    let drive_amt = 1.0 + self.drive * 4.0;
                    for sample in out_slice.iter_mut() {
                        *sample = (*sample * drive_amt).tanh();
                    }
                }

                // Apply gain
                for sample in out_slice.iter_mut() {
                    *sample *= self.gain;
                }
            }

            // Process MIDI events
            while event_idx < midi_events.len()
                && midi_events[event_idx].sample_offset as usize == end_frame
            {
                match midi_events[event_idx].data {
                    MidiMessage::NoteOn { key, velocity } => {
                        if velocity > 0 {
                            self.active_voices.push(SynthVoice::new(key, velocity, 3));
                        } else {
                            for v in self.active_voices.iter_mut() {
                                if v.note == key && v.is_active {
                                    v.release();
                                }
                            }
                        }
                    }
                    MidiMessage::NoteOff { key } => {
                        for v in self.active_voices.iter_mut() {
                            if v.note == key && v.is_active {
                                v.release();
                            }
                        }
                    }
                    _ => {}
                }
                event_idx += 1;
            }

            current_frame = end_frame;
        }

        self.cleanup_voices(); // Use macro-generated helper
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        // Try base parameters first (IDs 0-7)
        if self.base_set_parameter(id, value) {
            return;
        }

        // Synth-specific parameters
        match id {
            // Drive
            8 => self.drive = value.clamp(0.0, 1.0),

            // Osc 1 (IDs 10-13)
            10 => self.oscillators[0].waveform = Waveform::from(value),
            11 => self.oscillators[0].detune = value,
            12 => self.oscillators[0].mix = value,
            13 => self.oscillators[0].pulse_width = value.clamp(0.01, 0.99),

            // Osc 2 (IDs 20-23)
            20 => self.oscillators[1].waveform = Waveform::from(value),
            21 => self.oscillators[1].detune = value,
            22 => self.oscillators[1].mix = value,
            23 => self.oscillators[1].pulse_width = value.clamp(0.01, 0.99),

            // Osc 3 (IDs 30-33)
            30 => self.oscillators[2].waveform = Waveform::from(value),
            31 => self.oscillators[2].detune = value,
            32 => self.oscillators[2].mix = value,
            33 => self.oscillators[2].pulse_width = value.clamp(0.01, 0.99),

            _ => {}
        }
    }

    fn get_parameter(&self, id: u32) -> f32 {
        // Try base parameters first
        if let Some(v) = self.base_get_parameter(id) {
            return v;
        }

        // Synth-specific parameters
        match id {
            8 => self.drive,

            10 => self.oscillators[0].waveform as u32 as f32,
            11 => self.oscillators[0].detune,
            12 => self.oscillators[0].mix,
            13 => self.oscillators[0].pulse_width,

            20 => self.oscillators[1].waveform as u32 as f32,
            21 => self.oscillators[1].detune,
            22 => self.oscillators[1].mix,
            23 => self.oscillators[1].pulse_width,

            30 => self.oscillators[2].waveform as u32 as f32,
            31 => self.oscillators[2].detune,
            32 => self.oscillators[2].mix,
            33 => self.oscillators[2].pulse_width,

            _ => 0.0,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn default_parameters(&self) -> HashMap<u32, f32> {
        let mut map = Self::base_default_parameters(); // Use macro-generated helper

        // Synth-specific defaults
        map.insert(8, 0.0); // Drive

        // Osc 1
        map.insert(10, 1.0); // Saw
        map.insert(11, 0.0); // Detune
        map.insert(12, 1.0); // Mix
        map.insert(13, 0.5); // PW

        // Osc 2
        map.insert(20, 2.0); // Square
        map.insert(21, 0.1); // Detune
        map.insert(22, 0.5); // Mix
        map.insert(23, 0.5); // PW

        // Osc 3
        map.insert(30, 0.0); // Sine
        map.insert(31, -12.0); // Detune
        map.insert(32, 0.3); // Mix
        map.insert(33, 0.5); // PW

        map
    }
}
