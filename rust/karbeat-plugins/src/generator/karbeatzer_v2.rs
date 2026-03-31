// src/plugin/generator/karbeatzer_v2.rs

use std::{collections::HashMap, f32::consts::PI};

use karbeat_dsp::envelope::EnvelopeSettings;
use karbeat_plugin_api::prelude::*;

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
// KARBEATZER ENGINE (core synthesis logic)
// ============================================================================

/// The core Karbeatzer synthesis engine.
/// Contains only synth-specific fields like oscillators and drive.
/// The shared state (voices, filter, envelope) lives in SynthBase.
#[derive(Clone)]
pub struct KarbeatzerEngine {
    oscillators: [Oscillator; 3],
    drive: f32,
}

impl Default for KarbeatzerEngine {
    fn default() -> Self {
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
        }
    }
}

impl KarbeatzerEngine {
    /// Renders a block of audio for a single voice
    fn generate_voice_block(
        oscillators: &[Oscillator; 3],
        sample_rate: f32,
        amp_envelope: &EnvelopeSettings,
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

impl RawSynthEngine for KarbeatzerEngine {
    fn name() -> &'static str {
        "Karbeatzer"
    }

    fn process(
        &mut self,
        base: &mut StandardSynthBase,
        output_buffer: &mut [f32],
        midi_events: &[MidiEvent],
    ) {
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

                for voice in base.active_voices.iter_mut() {
                    if !voice.is_active {
                        continue;
                    }

                    // Ensure voice buffer is large enough
                    if base.voice_buffer.len() < block_len {
                        base.voice_buffer.resize(block_len, 0.0);
                    }

                    let scratch = &mut base.voice_buffer[0..block_len];
                    Self::generate_voice_block(
                        &self.oscillators,
                        base.sample_rate,
                        &base.amp_envelope,
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
                base.filter.process(out_slice, base.sample_rate);

                // Apply drive
                if self.drive > 0.0 {
                    let drive_amt = 1.0 + self.drive * 4.0;
                    for sample in out_slice.iter_mut() {
                        *sample = (*sample * drive_amt).tanh();
                    }
                }

                // Apply gain
                for sample in out_slice.iter_mut() {
                    *sample *= base.gain;
                }
            }

            // Process MIDI events
            while event_idx < midi_events.len()
                && midi_events[event_idx].sample_offset as usize == end_frame
            {
                match midi_events[event_idx].data {
                    MidiMessage::NoteOn { key, velocity } => {
                        if velocity > 0 {
                            base.active_voices.push(SynthVoice::new(key, velocity, 3));
                        } else {
                            for v in base.active_voices.iter_mut() {
                                if v.note == key && v.is_active {
                                    v.release();
                                }
                            }
                        }
                    }
                    MidiMessage::NoteOff { key } => {
                        for v in base.active_voices.iter_mut() {
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

        base.cleanup_voices();
    }

    fn set_custom_parameter(&mut self, id: u32, value: f32) {
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

    fn get_custom_parameter(&self, id: u32) -> Option<f32> {
        match id {
            8 => Some(self.drive),

            10 => Some(self.oscillators[0].waveform as u32 as f32),
            11 => Some(self.oscillators[0].detune),
            12 => Some(self.oscillators[0].mix),
            13 => Some(self.oscillators[0].pulse_width),

            20 => Some(self.oscillators[1].waveform as u32 as f32),
            21 => Some(self.oscillators[1].detune),
            22 => Some(self.oscillators[1].mix),
            23 => Some(self.oscillators[1].pulse_width),

            30 => Some(self.oscillators[2].waveform as u32 as f32),
            31 => Some(self.oscillators[2].detune),
            32 => Some(self.oscillators[2].mix),
            33 => Some(self.oscillators[2].pulse_width),

            _ => None,
        }
    }

    fn custom_default_parameters() -> HashMap<u32, f32> {
        let mut map = HashMap::new();

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

    fn get_parameter_specs(&self) -> Vec<karbeat_plugin_api::wrapper::PluginParameter> {
        use karbeat_plugin_api::wrapper::PluginParameter;

        let waveform_choices = vec![
            "Sine".into(),
            "Saw".into(),
            "Square".into(),
            "Triangle".into(),
            "Noise".into(),
        ];

        vec![
            // Drive
            PluginParameter::new_float(8, "Drive", "Master", self.drive, 0.0, 1.0, 0.0),
            // Osc 1
            PluginParameter::new_choice(
                10,
                "Waveform",
                "Oscillator 1",
                self.oscillators[0].waveform as u32,
                waveform_choices.clone(),
                1, // Saw default
            ),
            PluginParameter::new_float(
                11,
                "Detune",
                "Oscillator 1",
                self.oscillators[0].detune,
                -24.0,
                24.0,
                0.0,
            ),
            PluginParameter::new_float(
                12,
                "Mix",
                "Oscillator 1",
                self.oscillators[0].mix,
                0.0,
                1.0,
                1.0,
            ),
            PluginParameter::new_float(
                13,
                "Pulse Width",
                "Oscillator 1",
                self.oscillators[0].pulse_width,
                0.01,
                0.99,
                0.5,
            ),
            // Osc 2
            PluginParameter::new_choice(
                20,
                "Waveform",
                "Oscillator 2",
                self.oscillators[1].waveform as u32,
                waveform_choices.clone(),
                2, // Square default
            ),
            PluginParameter::new_float(
                21,
                "Detune",
                "Oscillator 2",
                self.oscillators[1].detune,
                -24.0,
                24.0,
                0.1,
            ),
            PluginParameter::new_float(
                22,
                "Mix",
                "Oscillator 2",
                self.oscillators[1].mix,
                0.0,
                1.0,
                0.5,
            ),
            PluginParameter::new_float(
                23,
                "Pulse Width",
                "Oscillator 2",
                self.oscillators[1].pulse_width,
                0.01,
                0.99,
                0.5,
            ),
            // Osc 3
            PluginParameter::new_choice(
                30,
                "Waveform",
                "Oscillator 3",
                self.oscillators[2].waveform as u32,
                waveform_choices,
                0, // Sine default
            ),
            PluginParameter::new_float(
                31,
                "Detune",
                "Oscillator 3",
                self.oscillators[2].detune,
                -24.0,
                24.0,
                -12.0,
            ),
            PluginParameter::new_float(
                32,
                "Mix",
                "Oscillator 3",
                self.oscillators[2].mix,
                0.0,
                1.0,
                0.3,
            ),
            PluginParameter::new_float(
                33,
                "Pulse Width",
                "Oscillator 3",
                self.oscillators[2].pulse_width,
                0.01,
                0.99,
                0.5,
            ),
        ]
    }
}

// ============================================================================
// TYPE ALIAS FOR WRAPPED SYNTH
// ============================================================================

/// The full Karbeatzer V2 synth (Subtractive Synthesizer).
pub type KarbeatzerV2 = RawSynthWrapper<KarbeatzerEngine>;

/// Helper to create a new Karbeatzer instance
pub fn create_karbeatzer(sample_rate: Option<f32>, channels: usize) -> KarbeatzerV2 {
    RawSynthWrapper::new(KarbeatzerEngine::default(), sample_rate.unwrap_or(48000.0), channels)
}
