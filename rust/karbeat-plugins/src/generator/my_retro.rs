// ====================================================
// MY RETRO SYNTH
// Author: Haidar Wibowo
// ====================================================

use karbeat_dsp::prelude::*;
use karbeat_macros::karbeat_plugin;
use karbeat_plugin_api::prelude::*;

/// A generator/synthesizer that produces a retro-sounding synth sound.
/// It has strictly two oscillators, making it a simple 8-bit retro sound.
#[derive(Clone)]
#[karbeat_plugin]
pub struct MyRetro {
    // Nested building blocks
    #[nested(prefix = "osc")]
    pub oscillators: [Oscillator; 2],

    #[nested(prefix = "amp_env/")]
    pub amp_envelope: EnvelopeSettings,

    // Local Parameters
    #[param(
        id = "bitcrush",
        name = "Resolution",
        group = "Bitcrush",
        min = 2.0,
        max = 256.0,
        default = 16.0,
        step = 1.0
    )]
    pub bitcrush_resolution: f32,

    #[param(
        id = "gain",
        name = "Gain",
        group = "Master",
        min = 0.0,
        max = 1.0,
        default = 0.8,
        step = 0.01
    )]
    pub gain: f32,

    #[param(
        id = "base_freq",
        name = "Base frequency",
        group = "Master",
        min = 432.0,
        max = 448.0,
        default = 440.0,
        step = 1.0
    )]
    pub base_freq: f32,

    // Internal Decoupled State
    pub active_voices: Vec<SynthVoice>,
    pub sample_rate: f32,
    pub channels: usize,
}

impl Default for MyRetro {
    fn default() -> Self {
        let mut engine = Self::base_default();

        // Override specific nested logic for the two oscillators safely
        let mut osc1 = Oscillator::new("Oscillator 1");
        osc1.waveform.set_base(Waveform::Square);
        osc1.mix.set_base(1.0);

        let mut osc2 = Oscillator::new("Oscillator 2");
        osc2.waveform.set_base(Waveform::Square);
        osc2.detune.set_base(-12.0);
        osc2.mix.set_base(0.8);

        engine.oscillators = [osc1, osc2];

        // Init internal state
        engine.active_voices = Vec::with_capacity(16);
        engine.sample_rate = 48000.0;
        engine.channels = 2;

        engine
    }
}

impl MyRetro {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pure associated function (no `&self`).
    /// Takes only the specific slices and primitives it needs!
    pub fn generate_voice_block(
        oscillators: &[Oscillator; 2],
        amp_envelope: &EnvelopeSettings,
        crush_steps: f32,
        sample_rate: f32,
        channels: usize,
        voice: &mut SynthVoice,
        buffer: &mut [f32],
        base_freq: f32,
    ) {
        buffer.fill(0.0);

        let base_freq = base_freq as f64 * (2.0_f64).powf(((voice.note as f64) - 69.0) / 12.0);

        for frame in buffer.chunks_exact_mut(channels) {
            if !voice.is_active {
                continue;
            }

            let env_level = voice.adsr.process(amp_envelope);

            if voice.adsr.state == EnvelopeState::Idle {
                voice.is_active = false;
                continue;
            }

            let velocity_gain = (voice.velocity as f32) / 127.0;
            let current_gain = velocity_gain * env_level;

            // Temporary buffer to hold the output of the oscillators BEFORE bitcrush
            let mut temp_sample = 0.0;

            for (i, osc) in oscillators.iter().enumerate() {
                let mut phase = voice.phase[i];
                let mut filter_state = voice.filter_state[i];

                // Extract 1 sample frame
                let mut osc_output = [0.0; 2];
                osc.output_wave(
                    &mut osc_output,
                    sample_rate as u32,
                    2,
                    base_freq,
                    &mut phase,
                    &mut filter_state,
                );

                voice.phase[i] = phase;

                // Mix the raw, uncrushed oscillators together
                temp_sample += osc_output[0];
            }

            // Apply the bitcrush ONCE to the cleanly summed signal,
            // and apply the envelope GAIN *AFTER* the bitcrush!
            let crushed_sample = (temp_sample * crush_steps).round() / crush_steps;
            let final_sample = crushed_sample * current_gain;

            for ch in frame.iter_mut() {
                *ch = final_sample;
            }
        }
    }
}

// ============================================================================
// DIRECT GENERATOR IMPLEMENTATION
// ============================================================================

#[karbeat_macros::auto_param]
impl AudioPlugin for MyRetro {
    fn name(&self) -> &str {
        "My Retro"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize, _max_buffer_size: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels;
    }

    fn reset(&mut self) {
        self.active_voices.clear();
    }

    fn process(&mut self, output_buffer: &mut [f32], context: &ProcessContext) {
        output_buffer.fill(0.0);

        let midi_events = context.midi_events;

        // Extract primitives outside the loop
        let master_gain = self.gain.get();
        let crush_steps = self.bitcrush_resolution.get().max(2.0);
        let total_frames = output_buffer.len() / self.channels;

        let mut current_frame = 0;
        let mut event_idx = 0;

        let mut scratch = vec![0.0; total_frames * self.channels];

        while current_frame < total_frames {
            let next_event_frame = if event_idx < midi_events.len() {
                midi_events[event_idx].sample_offset
            } else {
                total_frames
            };

            let end_frame = next_event_frame.min(total_frames);
            let block_len = end_frame - current_frame;

            if block_len > 0 {
                let out_slice =
                    &mut output_buffer[current_frame * self.channels..end_frame * self.channels];

                // Destructure `self` to separate mutable voices from immutable parameters
                let MyRetro {
                    active_voices,
                    oscillators,
                    amp_envelope,
                    sample_rate,
                    channels,
                    ..
                } = self;

                for voice in active_voices.iter_mut() {
                    if !voice.is_active {
                        continue;
                    }

                    let scratch_slice = &mut scratch[0..block_len * *channels];

                    Self::generate_voice_block(
                        oscillators,
                        amp_envelope,
                        crush_steps,
                        *sample_rate,
                        *channels,
                        voice,
                        scratch_slice,
                        self.base_freq.get(),
                    );

                    // Mix the voice scratch buffer into the main output
                    for (i, &sample) in scratch_slice.iter().enumerate() {
                        out_slice[i] += sample;
                    }
                }

                // Apply Master Synth Gain
                for sample in out_slice.iter_mut() {
                    *sample *= master_gain;
                }
            }

            // Handle MIDI events at this exact frame
            while event_idx < midi_events.len() && midi_events[event_idx].sample_offset == end_frame
            {
                match midi_events[event_idx].data {
                    MidiMessage::NoteOn { key, velocity } => {
                        if velocity > 0 {
                            let mut voice = SynthVoice::new(
                                key,
                                velocity,
                                self.sample_rate,
                                self.oscillators.len(),
                            );
                            for (i, osc) in self.oscillators.iter().enumerate() {
                                voice.phase[i] = osc.phase_offset.get() as f64;
                                voice.filter_state[i] = 0.0;
                            }
                            self.active_voices.push(voice);
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

        self.active_voices.retain(|v| v.is_active);
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Instrument
    }

    fn latency_samples(&self) -> u32 {
        0
    }

    fn tail_samples(&self) -> u32 {
        let release_ms = self.amp_envelope.release_ms.get();
        let tail_in_samples = (release_ms / 1000.0) * self.sample_rate;

        // Add a tiny safety buffer (e.g., 64 samples) to guarantee the
        // envelope has definitively hit absolute 0.0 before the engine culls it.
        (tail_in_samples as u32) + 64
    }

    fn execute_custom_command(
        &mut self,
        _command: &str,
        _payload: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        None
    }
}

// ============================================================================
// REGISTRY BUILDER & MANIFEST
// ============================================================================

impl AudioPluginBuilder for MyRetro {
    fn build() -> Self {
        Self::new()
    }
}

impl Manifestable for MyRetro {
    fn build_manifest() -> PluginManifest {
        PluginManifest {
            id: 1,
            name: "My Retro".to_owned(),
            internal_type: "MyRetro".to_owned(),
            is_synth: true,
            parameters: Self::static_parameter_specs(),
        }
    }
}
