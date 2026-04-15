// ====================================================
// MY RETRO SYNTH
// Author: Haidar Wibowo
// ====================================================

use std::collections::HashMap;

use karbeat_dsp::prelude::*;
use karbeat_macros::karbeat_plugin;
use karbeat_plugin_api::prelude::*;
use karbeat_plugin_types::{parameter::AutoParams, *};

/// A generator/synthesizer that produces a retro-sounding synth sound.
/// It only has strictly two oscillators and only
/// available as monophonic voice for each oscillator, making it
/// a simple 8-bit retro sound
#[derive(Clone)]
#[karbeat_plugin]
pub struct MyRetroEngine {
    #[nested]
    pub oscillators: [Oscillator; 2],

    #[param(id = 30, name = "Resolution", group = "Bitcrush", min = 2.0, max = 256.0, default = 16.0)]
    pub bitcrush_resolution: f32, // Re-written by macro to Param<f32>
}

impl Default for MyRetroEngine {
    fn default() -> Self {
        // Get the perfectly initialized parameters from the macro
        let mut engine = Self::base_default();

        // Override specific nested logic for the two oscillators safely
        let mut osc1 = Oscillator::new(10, "Oscillator 1");
        osc1.waveform.set_base(Waveform::Square.to_index() as f32);
        osc1.mix.set_base(1.0);

        let mut osc2 = Oscillator::new(20, "Oscillator 2");
        osc2.waveform.set_base(Waveform::Square.to_index() as f32);
        osc2.detune.set_base(-12.0);
        osc2.mix.set_base(0.8);

        engine.oscillators = [osc1, osc2];

        engine
    }
}

impl MyRetroEngine {
    pub fn generate_voice_block(
        &self,
        sample_rate: f32,
        channels: u8,
        amp_envelope: &EnvelopeSettings,
        voice: &mut SynthVoice,
        buffer: &mut [f32],
    ) {
        buffer.fill(0.0);

        let base_freq = 440.0 * (2.0_f64).powf(((voice.note as f64) - 69.0) / 12.0);
        let dt = 1.0 / sample_rate;
        let crush_steps = self.bitcrush_resolution.get().max(2.0);

        for frame in buffer.chunks_exact_mut(channels as usize) {
            let env_level = voice.advance_envelope(dt, amp_envelope);

            if !voice.is_active {
                for ch in frame.iter_mut() {
                    *ch = 0.0;
                }
                continue;
            }

            let velocity_gain = (voice.velocity as f32) / 127.0;
            let current_gain = velocity_gain * env_level;
            
            // We need a temporary buffer to hold the output of the oscillators
            // BEFORE we apply the bitcrusher
            let mut temp_sample = 0.0;

            for (i, osc) in self.oscillators.iter().enumerate() {
                if i >= voice.phase.len() {
                    voice.phase.push(0.0);
                }

                let mut phase = voice.phase[i] as f64;
                
                // We create a tiny 1-sample mono buffer to extract the exact value
                // of the oscillator at this exact phase
                let mut osc_output = [0.0; 2];
                osc.output_wave(&mut osc_output, sample_rate as u32, 2, base_freq, &mut phase);
                
                voice.phase[i] = phase as f32;
                
                // Mix the raw, uncrushed oscillators together
                temp_sample += osc_output[0];
            }

            // Apply the bitcrush ONCE to the cleanly summed signal, 
            // and apply the envelope GAIN *AFTER* the bitcrush!
            // (If you apply gain before bitcrush, the envelope decay will sound zipper-y and broken)
            
            let crushed_sample = (temp_sample * crush_steps).round() / crush_steps;
            let final_sample = crushed_sample * current_gain;

            for ch in frame.iter_mut() {
                *ch = final_sample;
            }
        }
    }
}

impl RawSynthEngine for MyRetroEngine {
    fn name() -> &'static str {
        "MyRetro"
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
            // Determine block boundaries based on next MIDI event
            let next_event_frame = if event_idx < midi_events.len() {
                midi_events[event_idx].sample_offset as usize
            } else {
                total_frames
            };

            let end_frame = next_event_frame.min(total_frames);
            let block_len = end_frame - current_frame;

            if block_len > 0 {
                let out_slice = &mut output_buffer[current_frame * 2..end_frame * 2];

                // Process each active voice
                for voice in base.active_voices.iter_mut() {
                    if !voice.is_active {
                        continue;
                    }

                    // Resize scratch buffer to match exact frame length * 2 (stereo)
                    if base.voice_buffer.len() < block_len * 2 {
                        base.voice_buffer.resize(block_len * 2, 0.0);
                    }

                    let scratch = &mut base.voice_buffer[0..block_len * 2];

                    self.generate_voice_block(
                        base.sample_rate,
                        2, // Stereo
                        &base.amp_envelope,
                        voice,
                        scratch,
                    );

                    // Mix the voice scratch buffer into the main output
                    for (i, &sample) in scratch.iter().enumerate() {
                        out_slice[i] += sample;
                    }
                }

                // Apply Master Synth Gain
                for sample in out_slice.iter_mut() {
                    *sample *= base.gain;
                }
            }

            // Handle MIDI events at this exact frame
            while event_idx < midi_events.len()
                && (midi_events[event_idx].sample_offset as usize) == end_frame
            {
                match midi_events[event_idx].data {
                    MidiMessage::NoteOn { key, velocity } => {
                        if velocity > 0 {
                            let mut voice = SynthVoice::new(key, velocity, self.oscillators.len());
                            for (i, osc) in self.oscillators.iter().enumerate() {
                                voice.phase[i] = osc.phase_offset.get() as f32; // Assuming your PhaseTracker holds f32
                            }
                            base.active_voices.push(voice);
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

        // Garbage collect dead voices
        base.cleanup_voices();
    }

    // ========================================================================
    // PARAMETER ROUTING (Magic via Macros)
    // ========================================================================

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
        // Dynamically build the map directly from the spec list so you never have to hardcode defaults!
        for spec in Self::default().auto_get_parameter_specs() {
            map.insert(spec.id, spec.default_value);
        }
        map
    }
}

/// A generator/synthesizer that produces a retro-sounding synth sound.
/// It only has strictly two oscillators and only
/// available as monophonic voice for each oscillator, making it
/// a simple 8-bit retro sound
pub type MyRetro = RawSynthWrapper<MyRetroEngine>;