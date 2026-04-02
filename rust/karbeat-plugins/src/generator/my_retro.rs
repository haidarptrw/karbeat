// ====================================================
// MY RETRO GENERATOR
// Author: Haidar Wibowo
// ====================================================

use std::{ collections::HashMap, default };

use karbeat_dsp::prelude::*;
use karbeat_plugin_api::prelude::*;
use smallvec::{ SmallVec, smallvec };

/// A generator/synthesizer that produces a retro-sounding synth sound.
/// it only has strictly two oscillator and only
/// available as monophonic for each oscillator, make it
/// a simple 8-bit retro sound
#[derive(Clone)]
pub struct MyRetroEngine {
    pub oscillators: SmallVec<[Oscillator; 2]>,
    pub bitcrush_resolution: f32,
}

impl Default for MyRetroEngine {
    fn default() -> Self {
        let osc1 = OscillatorBuilder::default()
            .waveform(Waveform::Square)
            .detune(0.0)
            .mix(1.0)
            .pulse_width(0.5)
            .phase_offset(0.0)
            .build()
            .unwrap(); // This unwrap is safe

        let osc2 = OscillatorBuilder::default()
            .waveform(Waveform::Square)
            .detune(-12.0)
            .mix(0.8)
            .pulse_width(0.5)
            .phase_offset(0.0)
            .build()
            .unwrap();

        Self {
            oscillators: smallvec![osc1, osc2],
            bitcrush_resolution: 16.0,
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
        buffer: &mut [f32]
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
        let crush_steps = self.bitcrush_resolution.max(2.0);

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
        midi_events: &[MidiEvent]
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
                        scratch
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
            while
                event_idx < midi_events.len() &&
                (midi_events[event_idx].sample_offset as usize) == end_frame
            {
                match midi_events[event_idx].data {
                    MidiMessage::NoteOn { key, velocity } => {
                        if velocity > 0 {
                            base.active_voices.push(
                                SynthVoice::new(key, velocity, self.oscillators.len())
                            );
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
        match id {
            10 => {
                self.oscillators[0].waveform = Waveform::from(value);
            }
            11 => {
                self.oscillators[0].detune = value;
            }
            12 => {
                self.oscillators[0].mix = value;
            }
            13 => {
                self.oscillators[0].pulse_width = value.clamp(0.01, 0.99);
            }

            20 => {
                self.oscillators[1].waveform = Waveform::from(value);
            }
            21 => {
                self.oscillators[1].detune = value;
            }
            22 => {
                self.oscillators[1].mix = value;
            }
            23 => {
                self.oscillators[1].pulse_width = value.clamp(0.01, 0.99);
            }

            30 => {
                self.bitcrush_resolution = value.clamp(2.0, 256.0);
            }
            _ => {}
        }
    }

    fn get_custom_parameter(&self, id: u32) -> Option<f32> {
        match id {
            10 => Some(self.oscillators[0].waveform as u32 as f32),
            11 => Some(self.oscillators[0].detune),
            12 => Some(self.oscillators[0].mix),
            13 => Some(self.oscillators[0].pulse_width),

            20 => Some(self.oscillators[1].waveform as u32 as f32),
            21 => Some(self.oscillators[1].detune),
            22 => Some(self.oscillators[1].mix),
            23 => Some(self.oscillators[1].pulse_width),

            30 => Some(self.bitcrush_resolution),
            _ => None,
        }
    }

    fn custom_default_parameters() -> HashMap<u32, f32> {
        let mut map = HashMap::new();
        map.insert(10, 2.0); // Osc 1: Square
        map.insert(11, 0.0);
        map.insert(12, 1.0);
        map.insert(13, 0.5);

        map.insert(20, 3.0); // Osc 2: Triangle
        map.insert(21, -12.0);
        map.insert(22, 0.8);
        map.insert(23, 0.5);

        map.insert(30, 16.0); // Bitcrush
        map
    }

    fn get_parameter_specs(&self) -> Vec<PluginParameter> {
        let waveforms = vec![
            "Sine".into(),
            "Saw".into(),
            "Square".into(),
            "Triangle".into(),
            "Noise".into()
        ];

        vec![
            PluginParameter::new_choice(
                10,
                "Waveform",
                "Oscillator 1",
                self.oscillators[0].waveform as u32,
                waveforms.clone(),
                2
            ),
            PluginParameter::new_float(
                11,
                "Detune",
                "Oscillator 1",
                self.oscillators[0].detune,
                -24.0,
                24.0,
                0.0
            ),
            PluginParameter::new_float(
                12,
                "Mix",
                "Oscillator 1",
                self.oscillators[0].mix,
                0.0,
                1.0,
                1.0
            ),
            PluginParameter::new_float(
                13,
                "Pulse Width",
                "Oscillator 1",
                self.oscillators[0].pulse_width,
                0.01,
                0.99,
                0.5
            ),

            PluginParameter::new_choice(
                20,
                "Waveform",
                "Oscillator 2",
                self.oscillators[1].waveform as u32,
                waveforms,
                3
            ),
            PluginParameter::new_float(
                21,
                "Detune",
                "Oscillator 2",
                self.oscillators[1].detune,
                -24.0,
                24.0,
                -12.0
            ),
            PluginParameter::new_float(
                22,
                "Mix",
                "Oscillator 2",
                self.oscillators[1].mix,
                0.0,
                1.0,
                0.8
            ),
            PluginParameter::new_float(
                23,
                "Pulse Width",
                "Oscillator 2",
                self.oscillators[1].pulse_width,
                0.01,
                0.99,
                0.5
            ),

            PluginParameter::new_float(
                30,
                "Resolution",
                "Bitcrush",
                self.bitcrush_resolution,
                2.0,
                256.0,
                16.0
            )
        ]
    }
}

pub type MyRetro = RawSynthWrapper<MyRetroEngine>;

pub fn create_my_retro_synth(sample_rate: f32, channels: u16) -> MyRetro {
    RawSynthWrapper::new(MyRetroEngine::default(), sample_rate, channels as usize)
}
