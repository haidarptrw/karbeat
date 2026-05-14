//! src/plugin/generator/karbeatzer_v2.rs

use std::{any::Any, f32::consts::PI};

use hashbrown::HashMap;
use karbeat_dsp::prelude::*;
use karbeat_macros::karbeat_plugin;
use karbeat_plugin_api::prelude::*;
use karbeat_plugin_types::*;

#[derive(Clone)]
#[karbeat_plugin]
pub struct KarbeatzerV2 {
    // Component Building Blocks
    #[nested(prefix = "osc")]
    pub oscillators: [Oscillator; 3],

    #[nested(prefix = "amp_env/")]
    pub amp_envelope: EnvelopeSettings,

    #[nested(prefix = "filter/")]
    pub filter: BiquadFilter<SimpleFilterMode>,

    // Local Parameters
    #[param(
        id = "drive",
        name = "Drive",
        group = "Master",
        min = 0.0,
        max = 1.0,
        default = 0.0,
        step = 0.01
    )]
    pub drive: f32,

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

    // Internal Decoupled State (Ignored by Param UI)
    pub active_voices: Vec<SynthVoice>,
    pub sample_rate: f32,
    pub channels: usize,

    #[serde(skip)]
    pub scratch_buffer: Vec<f32>,
}

impl Default for KarbeatzerV2 {
    fn default() -> Self {
        let mut engine = Self::base_default();

        engine.oscillators = [
            Oscillator::new("Oscillator 1"),
            Oscillator::new("Oscillator 2"),
            Oscillator::new("Oscillator 3"),
        ];

        // Setup some nice default starting values
        engine.oscillators[0]
            .waveform
            .set_base(Waveform::Saw as usize as f32);
        engine.oscillators[0].mix.set_base(1.0);

        engine.oscillators[1]
            .waveform
            .set_base(Waveform::Square as usize as f32);
        engine.oscillators[1].detune.set_base(0.1);
        engine.oscillators[1].mix.set_base(0.5);

        engine.oscillators[2]
            .waveform
            .set_base(Waveform::Sine as usize as f32);
        engine.oscillators[2].detune.set_base(-12.0);
        engine.oscillators[2].mix.set_base(0.3);

        // Init internal state
        engine.active_voices = Vec::with_capacity(16);
        engine.sample_rate = 48000.0;
        engine.channels = 2;

        engine.scratch_buffer = vec![0.0; 4096];

        engine
    }
}

impl KarbeatzerV2 {
    /// Mixes a block of audio for a single voice
    fn generate_voice_block(
        oscillators: &[Oscillator; 3],
        amp_envelope: &EnvelopeSettings,
        sample_rate: f32,
        voice: &mut SynthVoice,
        buffer: &mut [f32],
    ) {
        let block_size = buffer.len();
        let base_freq = 440.0 * (2.0_f32).powf(((voice.note as f32) - 69.0) / 12.0);
        // let dt = 1.0 / sample_rate;

        // Extract oscillator parameters
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

        // Render block
        for frame in 0..block_size {
            let env_level = voice.adsr.process(amp_envelope);

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
                    Waveform::Sine => (phase * 2.0 * PI as f64).sin() as f32,
                    Waveform::Saw => {
                        let naive = (2.0 * phase - 1.0) as f32;
                        naive - Oscillator::poly_blep(phase, dt_inc as f64) as f32
                    }
                    Waveform::Square => {
                        let naive = if phase < pws[i] as f64 { 1.0 } else { -1.0 };
                        let mut blep = Oscillator::poly_blep(phase, dt_inc as f64) as f32;
                        let phase2 = (phase + 1.0 - pws[i] as f64) % 1.0;
                        blep -= Oscillator::poly_blep(phase2, dt_inc as f64) as f32;
                        naive - blep
                    }
                    Waveform::Triangle => (4.0 * (phase - 0.5).abs() - 1.0) as f32,
                    Waveform::Noise => fastrand::f32() * 2.0 - 1.0,
                };

                sample_accum += osc_out * mixes[i];

                voice.phase[i] += dt_inc as f64;
                if voice.phase[i] >= 1.0 {
                    voice.phase[i] -= 1.0;
                }
            }

            buffer[frame] = sample_accum * current_gain;
        }
    }

    fn handle_side_effects(&mut self) {
        // Safe to call every time parameters change
        self.filter.calculate_coefficients();
    }
}

// ============================================================================
// DIRECT GENERATOR IMPLEMENTATION
// ============================================================================

impl AudioPlugin for KarbeatzerV2 {
    fn name(&self) -> &str {
        "Karbeatzer V2"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize, _max_buffer_size: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels;
        self.filter.prepare(channels as u8, sample_rate as u32);
        self.filter.calculate_coefficients();
    }

    fn reset(&mut self) {
        self.active_voices.clear();
        self.filter.reset_state();
    }

    fn process(&mut self, output_buffer: &mut [f32], context: &ProcessContext) {
        output_buffer.fill(0.0);

        let current_drive = self.drive.get();
        let master_gain = self.gain.get();
        let total_frames = output_buffer.len() / self.channels;
        let midi_events = context.midi_events;

        let mut current_frame = 0;
        let mut event_idx = 0;

        // Ensure our persistent scratch buffer is large enough for this block
        if self.scratch_buffer.len() < total_frames {
            self.scratch_buffer.resize(total_frames, 0.0);
        }

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

                // Destructure `self` to separate the mutable voices from the immutable components!
                let KarbeatzerV2 {
                    active_voices,
                    oscillators,
                    amp_envelope,
                    sample_rate,
                    scratch_buffer,
                    ..
                } = self;

                let scratch_slice = &mut scratch_buffer[0..block_len];

                // 1. Render all active voices
                for voice in active_voices.iter_mut() {
                    if !voice.is_active {
                        continue;
                    }

                    Self::generate_voice_block(
                        oscillators,
                        amp_envelope,
                        *sample_rate,
                        voice,
                        scratch_slice,
                    );

                    // Mix mono voice into stereo output buffer
                    for (i, &sample) in scratch_slice.iter().enumerate() {
                        for ch in 0..self.channels {
                            out_slice[i * self.channels + ch] += sample;
                        }
                    }
                }

                // 2. Apply global filter
                for chunk in out_slice.chunks_exact_mut(self.channels) {
                    self.filter.process_frame(chunk);
                }

                // 3. Apply drive
                if current_drive > 0.0 {
                    let drive_amt = 1.0 + current_drive * 4.0;
                    for sample in out_slice.iter_mut() {
                        *sample = (*sample * drive_amt).tanh();
                    }
                }

                // 4. Apply Master Gain
                for sample in out_slice.iter_mut() {
                    *sample *= master_gain;
                }
            }

            // Process MIDI
            while event_idx < midi_events.len() && midi_events[event_idx].sample_offset == end_frame
            {
                match midi_events[event_idx].data {
                    MidiMessage::NoteOn { key, velocity } => {
                        if velocity > 0 {
                            self.active_voices.push(SynthVoice::new(
                                key,
                                velocity,
                                self.sample_rate,
                                self.oscillators.len(),
                            ));
                        } else {
                            for v in self.active_voices.iter_mut().filter(|v| v.note == key) {
                                v.release();
                            }
                        }
                    }
                    MidiMessage::NoteOff { key } => {
                        for v in self.active_voices.iter_mut().filter(|v| v.note == key) {
                            v.release();
                        }
                    }
                    _ => {}
                }
                event_idx += 1;
            }
            current_frame = end_frame;
        }

        // Cleanup dead voices
        self.active_voices.retain(|v| v.is_active);
    }

    // --- Dynamic Hashed Routing! ---

    fn set_parameter(&mut self, id: u32, value: f32) {
        if self.auto_set_parameter(karbeat_utils::hash::FNV_OFFSET, id, value) {
            self.handle_side_effects();
        }
    }

    fn get_parameter(&self, id: u32) -> f32 {
        self.auto_get_parameter(karbeat_utils::hash::FNV_OFFSET, id)
            .unwrap_or(0.0)
    }

    fn apply_automation(&mut self, id: u32, value: f32) {
        if self.auto_apply_automation(karbeat_utils::hash::FNV_OFFSET, id, value) {
            self.handle_side_effects();
        }
    }

    fn clear_automation(&mut self, id: u32) {
        if self.auto_clear_automation(karbeat_utils::hash::FNV_OFFSET, id) {
            self.handle_side_effects();
        }
    }

    fn default_parameters(&self) -> HashMap<u32, f32> {
        self.auto_get_parameter_specs(karbeat_utils::hash::FNV_OFFSET, "")
            .into_iter()
            .map(|spec| (spec.id, spec.default_value))
            .collect()
    }

    fn get_parameter_specs(&self) -> Vec<ParameterSpec> {
        self.auto_get_parameter_specs(karbeat_utils::hash::FNV_OFFSET, "")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn static_parameter_specs() -> Vec<ParameterSpec>
    where
        Self: Sized,
    {
        Self::default().auto_get_parameter_specs(karbeat_utils::hash::FNV_OFFSET, "")
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

impl AudioPluginBuilder for KarbeatzerV2 {
    fn build() -> Self {
        Self::default()
    }
}

impl Manifestable for KarbeatzerV2 {
    fn build_manifest() -> PluginManifest {
        PluginManifest {
            id: 0,
            name: "Karbeatzer V2".to_owned(),
            internal_type: "KarbeatzerV2".to_owned(),
            is_synth: true,
            parameters: Self::default()
                .auto_get_parameter_specs(karbeat_utils::hash::FNV_OFFSET, ""),
        }
    }
}
