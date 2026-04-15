//! src/plugin/generator/karbeatzer_v2.rs

use std::{ collections::HashMap, f32::consts::PI };

use karbeat_dsp::prelude::*;
use karbeat_macros::karbeat_plugin;
use karbeat_plugin_api::{ prelude::* };
use karbeat_plugin_types::*;

// ============================================================================
// KARBEATZER ENGINE (core synthesis logic)
// ============================================================================

/// The core Karbeatzer synthesis engine.
/// Contains only synth-specific fields like oscillators and drive.
/// The shared state (voices, filter, envelope) lives in SynthBase.
#[derive(Clone)]
#[karbeat_plugin]
pub struct KarbeatzerEngine {
    #[nested]
    oscillators: [Oscillator; 3],
    #[param(id = 8, name = "Drive", group = "Master", min = 0.0, max = 1.0, default = 0.0)]
    drive: f32,
}

impl Default for KarbeatzerEngine {
    fn default() -> Self {
        // Get the perfectly initialized parameters from the macro
        let mut engine = Self::base_default();

        let mut osc1 = Oscillator::new(10, "Oscillator 1");
        osc1.waveform.set_base(Waveform::Saw.to_index() as f32);
        osc1.mix.set_base(1.0);

        let mut osc2 = Oscillator::new(20, "Oscillator 2");
        osc2.waveform.set_base(Waveform::Square.to_index() as f32);
        osc2.detune.set_base(0.1);
        osc2.mix.set_base(0.5);

        let mut osc3 = Oscillator::new(30, "Oscillator 3");
        osc3.waveform.set_base(Waveform::Sine.to_index() as f32);
        osc3.detune.set_base(-12.0);
        osc3.mix.set_base(0.3);

        // Override the uninitialized array
        engine.oscillators = [osc1, osc2, osc3];

        engine
    }
}

impl KarbeatzerEngine {
    /// Renders a block of audio for a single voice
    fn generate_voice_block(
        oscillators: &[Oscillator; 3],
        sample_rate: f32,
        amp_envelope: &EnvelopeSettings,
        voice: &mut SynthVoice,
        buffer: &mut [f32]
    ) {
        let block_size = buffer.len();
        let base_freq = 440.0 * (2.0_f32).powf(((voice.note as f32) - 69.0) / 12.0);
        let dt = 1.0 / sample_rate;

        // Pre-calculate phase increments
        let mut phase_incs = [0.0; 3];
        let mut wfs = [Waveform::Sine; 3];
        let mut mixes = [0.0; 3];
        let mut pws = [0.5; 3];

        for (i, osc) in oscillators.iter().enumerate() {
            let detune = osc.detune.get();
            let freq = base_freq * (2.0_f32).powf(detune / 12.0);

            phase_incs[i] = freq / sample_rate;
            wfs[i] = osc.waveform.get();
            mixes[i] = osc.mix.get();
            pws[i] = osc.pulse_width.get();
        }

        for frame in 0..block_size {
            let env_level = voice.advance_envelope(dt, amp_envelope);

            if !voice.is_active {
                buffer[frame] = 0.0;
                continue;
            }

            let velocity_gain = (voice.velocity as f32) / 127.0;
            let current_gain = velocity_gain * env_level;
            let mut sample_accum = 0.0;

            for i in 0..3 {
                let phase = voice.phase[i];
                let dt_inc = phase_incs[i];

                let osc_out = match wfs[i] {
                    Waveform::Sine => (phase * 2.0 * PI).sin(),
                    Waveform::Saw => {
                        let naive = 2.0 * phase - 1.0;
                        // Apply PolyBLEP for anti-aliasing
                        naive - poly_blep(phase, dt_inc)
                    },
                    Waveform::Square => {
                        let naive = if phase < pws[i] { 1.0 } else { -1.0 };
                        // Apply PolyBLEP for anti-aliasing
                        let mut blep = poly_blep(phase, dt_inc);
                        let phase2 = (phase + 1.0 - pws[i]) % 1.0;
                        blep -= poly_blep(phase2, dt_inc);
                        naive - blep
                    }
                    Waveform::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
                    Waveform::Noise => fastrand::f32() * 2.0 - 1.0,
                };

                sample_accum += osc_out * mixes[i];

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
        midi_events: &[MidiEvent]
    ) {
        output_buffer.fill(0.0);

        let current_drive = self.drive.get();

        let total_frames = output_buffer.len() / 2;
        let mut current_frame = 0;
        let mut event_idx = 0;

        while current_frame < total_frames {
            let next_event_frame = if event_idx < midi_events.len() {
                midi_events[event_idx].sample_offset
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
                        scratch
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
                if current_drive > 0.0 {
                    let drive_amt = 1.0 + current_drive * 4.0;
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
            while
                event_idx < midi_events.len() &&
                (midi_events[event_idx].sample_offset as usize) == end_frame
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
        self.auto_set_parameter(id, value);
    }

    fn get_custom_parameter(&self, id: u32) -> Option<f32> {
        self.auto_get_parameter(id)
    }

    fn apply_automation(&mut self, id: u32, value: f32) {
        self.auto_apply_automation(id, value);
    }

    fn clear_automation(&mut self, id: u32) {
        self.auto_clear_automation(id);
    }

    fn get_parameter_specs(&self) -> Vec<ParameterSpec> {
        self.auto_get_parameter_specs()
    }

    fn custom_default_parameters() -> HashMap<u32, f32> where Self: Sized {
        let mut map = HashMap::new();
        let default_engine = Self::default();

        // You can either extract this via a macro helper if you build one later,
        // or just use the specs vector to dynamically construct the defaults!
        for spec in default_engine.auto_get_parameter_specs() {
            map.insert(spec.id, spec.default_value);
        }

        map
    }
}

/// Calculates the Polynomial Band-Limited Step for anti-aliasing.
/// `t` is the current phase (0.0 to 1.0)
/// `dt` is the phase increment per sample
#[inline(always)]
fn poly_blep(mut t: f32, dt: f32) -> f32 {
    if t < dt {
        // At the start of the phase cycle
        t /= dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        // At the end of the phase cycle
        t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

// ============================================================================
// TYPE ALIAS FOR WRAPPED SYNTH
// ============================================================================

/// The full Karbeatzer V2 synth (Subtractive Synthesizer).
pub type KarbeatzerV2 = RawSynthWrapper<KarbeatzerEngine>;
