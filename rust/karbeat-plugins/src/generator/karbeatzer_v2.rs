//! src/plugin/generator/karbeatzer_v2.rs

use std::{collections::HashMap, f32::consts::PI};

use karbeat_dsp::prelude::*;
use karbeat_plugin_api::{prelude::*};
use karbeat_plugin_types::*;

// ============================================================================
// KARBEATZER ENGINE (core synthesis logic)
// ============================================================================

/// The core Karbeatzer synthesis engine.
/// Contains only synth-specific fields like oscillators and drive.
/// The shared state (voices, filter, envelope) lives in SynthBase.
#[derive(Clone)]
pub struct KarbeatzerEngine {
    oscillators: [Oscillator; 3],
    drive: Param<f32>,
}

impl Default for KarbeatzerEngine {
    fn default() -> Self {
       // Create baseline oscillators using our new constructor
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

        Self {
            oscillators: [osc1, osc2, osc3],
            drive: Param::new_float(8, "Drive", "Master", 0.0, 0.0, 1.0),
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
        let mut wfs = [Waveform::Sine; 3];
        let mut mixes = [0.0; 3];
        let mut pws = [0.5; 3];
        
        for (i, osc) in oscillators.iter().enumerate() {
            let detune = osc.detune.get();
            let freq = base_freq * 2.0_f32.powf(detune / 12.0);
            
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

            let velocity_gain = voice.velocity as f32 / 127.0;
            let current_gain = velocity_gain * env_level;
            let mut sample_accum = 0.0;

            for i in 0..3 {
                let phase = voice.phase[i];

                let osc_out = match wfs[i] {
                    Waveform::Sine => (phase * 2.0 * PI).sin(),
                    Waveform::Saw => 2.0 * phase - 1.0,
                    Waveform::Square => if phase < pws[i] as f32 { 1.0 } else { -1.0 },
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
        midi_events: &[MidiEvent],
    ) {
        output_buffer.fill(0.0);

        let current_drive = self.drive.get();

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
        if self.drive.id == id {
            self.drive.set_base(value);
            return;
        }

        // route to the child oscillators
        for osc in &mut self.oscillators {
            if osc.waveform.id == id { osc.waveform.set_base(value); return; }
            if osc.detune.id == id { osc.detune.set_base(value); return; }
            if osc.phase_offset.id == id { osc.phase_offset.set_base(value); return; }
            if osc.mix.id == id { osc.mix.set_base(value); return; }
            if osc.pulse_width.id == id { osc.pulse_width.set_base(value); return; }
        }
    }

    fn get_custom_parameter(&self, id: u32) -> Option<f32> {
        if self.drive.id == id { return Some(self.drive.get_base().to_f32()); }
        
        for osc in &self.oscillators {
            if osc.waveform.id == id { return Some(osc.waveform.get_base().to_f32()); }
            if osc.detune.id == id { return Some(osc.detune.get_base().to_f32()); }
            if osc.phase_offset.id == id { return Some(osc.phase_offset.get_base().to_f32()); }
            if osc.mix.id == id { return Some(osc.mix.get_base().to_f32()); }
            if osc.pulse_width.id == id { return Some(osc.pulse_width.get_base().to_f32()); }
        }
        None
    }

    fn custom_default_parameters() -> HashMap<u32, f32> {
        let mut map = HashMap::new();
        let default_engine = Self::default();

        map.insert(default_engine.drive.id, default_engine.drive.get_base().to_f32());

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
        let mut specs = vec![self.drive.to_spec()];
        
        for osc in &self.oscillators {
            specs.push(osc.waveform.to_spec());
            specs.push(osc.detune.to_spec());
            specs.push(osc.phase_offset.to_spec());
            specs.push(osc.mix.to_spec());
            specs.push(osc.pulse_width.to_spec());
        }
        
        specs
    }
    
    fn apply_automation(&mut self, id: u32, value: f32) {
        todo!()
    }
    
    fn clear_automation(&mut self, id: u32) {
        todo!()
    }
}

// ============================================================================
// TYPE ALIAS FOR WRAPPED SYNTH
// ============================================================================

/// The full Karbeatzer V2 synth (Subtractive Synthesizer).
pub type KarbeatzerV2 = RawSynthWrapper<KarbeatzerEngine>;