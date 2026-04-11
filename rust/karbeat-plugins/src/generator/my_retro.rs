// ====================================================
// MY RETRO SYNTH
// Author: Haidar Wibowo
// ====================================================

use std::collections::HashMap;

use karbeat_dsp::prelude::*;
use karbeat_plugin_api::prelude::*;
use karbeat_plugin_types::*;
use smallvec::{smallvec, SmallVec};

/// A generator/synthesizer that produces a retro-sounding synth sound.
/// it only has strictly two oscillator and only
/// available as monophonic voice for each oscillator, making it
/// a simple 8-bit retro sound
#[derive(Clone)]
pub struct MyRetroEngine {
    pub oscillators: SmallVec<[Oscillator; 2]>,
    pub bitcrush_resolution: Param<f32>,
}

impl Default for MyRetroEngine {
    fn default() -> Self {
        let mut osc1 = Oscillator::new(10, "Oscillator 1");
        osc1.waveform.set_base(Waveform::Square.to_index() as f32);
        osc1.mix.set_base(1.0);

        let mut osc2 = Oscillator::new(20, "Oscillator 2");
        osc2.waveform.set_base(Waveform::Square.to_index() as f32);
        osc2.detune.set_base(-12.0);
        osc2.mix.set_base(0.8);

        Self {
            oscillators: smallvec![osc1, osc2],
            bitcrush_resolution: Param::new_float(30, "Resolution", "Bitcrush", 16.0, 2.0, 256.0),
        }
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
        // Clear the scratch buffer before DASP `add_amp` accumulates into it
        buffer.fill(0.0);

        let base_freq = 440.0 * (2.0_f64).powf(((voice.note as f64) - 69.0) / 12.0);
        let dt = 1.0 / sample_rate;

        // Render both oscillators directly into the buffer
        for (i, osc) in self.oscillators.iter().enumerate() {
            // Ensure the voice has enough phase trackers
            if i >= voice.phase.len() {
                voice.phase.push(0.0);
            }

            let mut phase = voice.phase[i] as f64;
            osc.output_wave(buffer, sample_rate as u32, channels, base_freq, &mut phase);
        }

        // Apply ADSR, Velocity, and 8-Bit Crush frame-by-frame
        let velocity_gain = (voice.velocity as f32) / 127.0;
        let crush_steps = self.bitcrush_resolution.get().max(2.0);

        for frame in buffer.chunks_exact_mut(channels as usize) {
            let env_level = voice.advance_envelope(dt, amp_envelope);

            // If envelope finished, mark voice dead and stop processing this frame
            if !voice.is_active {
                for ch in frame.iter_mut() {
                    *ch = 0.0;
                }
                continue;
            }

            let current_gain = velocity_gain * env_level;

            for ch in frame.iter_mut() {
                // Apply gain
                let mut sample = *ch * current_gain;

                // Apply Retro Bitcrushing (Quantize amplitude)
                sample = (sample * crush_steps).round() / crush_steps;

                *ch = sample;
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
                                voice.phase[i] = osc.phase_offset.get();
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

    fn set_custom_parameter(&mut self, id: u32, value: f32) {
        if self.bitcrush_resolution.id == id {
            self.bitcrush_resolution.set_base(value);
            return;
        }

        // Route to the child oscillators
        for osc in &mut self.oscillators {
            if osc.waveform.id == id {
                osc.waveform.set_base(value);
                return;
            }
            if osc.detune.id == id {
                osc.detune.set_base(value);
                return;
            }
            if osc.phase_offset.id == id {
                osc.phase_offset.set_base(value);
                return;
            }
            if osc.mix.id == id {
                osc.mix.set_base(value);
                return;
            }
            if osc.pulse_width.id == id {
                osc.pulse_width.set_base(value);
                return;
            }
        }
    }

    fn get_custom_parameter(&self, id: u32) -> Option<f32> {
        if self.bitcrush_resolution.id == id {
            return Some(self.bitcrush_resolution.get_base().to_f32());
        }

        for osc in &self.oscillators {
            if osc.waveform.id == id {
                return Some(osc.waveform.get_base().to_f32());
            }
            if osc.detune.id == id {
                return Some(osc.detune.get_base().to_f32());
            }
            if osc.phase_offset.id == id {
                return Some(osc.phase_offset.get_base().to_f32());
            }
            if osc.mix.id == id {
                return Some(osc.mix.get_base().to_f32());
            }
            if osc.pulse_width.id == id {
                return Some(osc.pulse_width.get_base().to_f32());
            }
        }
        None
    }

    fn custom_default_parameters() -> HashMap<u32, f32> {
        let mut map = HashMap::new();
        let default_engine = Self::default();

        map.insert(
            default_engine.bitcrush_resolution.id,
            default_engine.bitcrush_resolution.get_base().to_f32(),
        );

        for osc in &default_engine.oscillators {
            map.insert(osc.waveform.id, osc.waveform.get_base().to_f32());
            map.insert(osc.detune.id, osc.detune.get_base().to_f32());
            map.insert(osc.phase_offset.id, osc.phase_offset.get_base().to_f32());
            map.insert(osc.mix.id, osc.mix.get_base().to_f32());
            map.insert(osc.pulse_width.id, osc.pulse_width.get_base().to_f32());
        }
        map
    }

    fn get_parameter_specs(&self) -> Vec<PluginParameter> {
        let mut specs = Vec::new();

        for osc in &self.oscillators {
            specs.push(osc.waveform.to_spec());
            specs.push(osc.detune.to_spec());
            specs.push(osc.phase_offset.to_spec());
            specs.push(osc.mix.to_spec());
            specs.push(osc.pulse_width.to_spec());
        }

        specs.push(self.bitcrush_resolution.to_spec());
        specs
    }
    
    fn apply_automation(&mut self, id: u32, value: f32) {
        todo!()
    }
    
    fn clear_automation(&mut self, id: u32) {
        todo!()
    }
}

/// A generator/synthesizer that produces a retro-sounding synth sound.
/// it only has strictly two oscillator and only
/// available as monophonic voice for each oscillator, making it
/// a simple 8-bit retro sound
pub type MyRetro = RawSynthWrapper<MyRetroEngine>;

pub fn create_my_retro_synth(sample_rate: f32, channels: u16) -> MyRetro {
    RawSynthWrapper::new(MyRetroEngine::default(), sample_rate, channels as usize)
}
