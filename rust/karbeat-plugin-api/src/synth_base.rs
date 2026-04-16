// src/plugin/synth_base.rs
//
// Shared synthesizer infrastructure for all synth plugins.
// Replaces procedural macro-generated code with simple composition.

use indexmap::IndexMap;
use karbeat_dsp::{envelope::{EnvelopeSettings, EnvelopeStage}, filter::SimpleFilterMode};
use karbeat_plugin_types::ParameterSpec;


// ============================================================================
// FILTER
// ============================================================================

/// State-variable filter for the synthesizer
#[derive(Clone, Copy, Debug)]
pub struct SynthFilter {
    pub cutoff: f32,    // Hz (20.0 - 20000.0)
    pub resonance: f32, // 0.0 to 0.95
    pub mode: SimpleFilterMode,
    // Internal state (Stereo)
    s1_l: f32,
    s2_l: f32,
    s1_r: f32,
    s2_r: f32,
}

impl Default for SynthFilter {
    fn default() -> Self {
        Self {
            cutoff: 2000.0,
            resonance: 0.2,
            mode: SimpleFilterMode::LowPass,
            s1_l: 0.0,
            s2_l: 0.0,
            s1_r: 0.0,
            s2_r: 0.0,
        }
    }
}

impl SynthFilter {
    /// Reset filter state (call on reset/seek)
    pub fn reset(&mut self) {
        self.s1_l = 0.0;
        self.s2_l = 0.0;
        self.s1_r = 0.0;
        self.s2_r = 0.0;
    }

    /// Process a stereo interleaved buffer through the filter
    /// Uses the Chamberlin State Variable Filter algorithm
    pub fn process(&mut self, buffer: &mut [f32], sample_rate: f32) {
        if self.mode == SimpleFilterMode::Off {
            return;
        }

        // Constrain cutoff to slightly below Nyquist to prevent tan() explosion
        let nyquist = sample_rate * 0.499;
        let clamped_cutoff = self.cutoff.clamp(20.0, nyquist);

        // g = tan(PI * cutoff / sample_rate) maps the frequency domain correctly
        let g = (std::f32::consts::PI * clamped_cutoff / sample_rate).tan();

        // Map resonance (0.0 -> 0.95) to damping (k)
        // res 0.0 => k = 2.0 (Butterworth/No resonance)
        // res 0.95 => k = 0.1 (High resonance)
        let k = 2.0 - (2.0 * self.resonance.clamp(0.0, 0.95));

        // Pre-calculate inverse denominator for the loop
        let denom_inv = 1.0 / (1.0 + g * (g + k));

        let frames = buffer.len() / 2;

        for i in 0..frames {
            let l_idx = i * 2;
            let r_idx = i * 2 + 1;

            let in_l = buffer[l_idx];
            let in_r = buffer[r_idx];

            // Left Channel (TPT SVF)
            // Calculate outputs based on current state
            let hp_l = (in_l - (g + k) * self.s1_l - self.s2_l) * denom_inv;
            let bp_l = g * hp_l + self.s1_l;
            let lp_l = g * bp_l + self.s2_l;

            // Update state for next sample
            // Note: TPT state update is 2*sample - old_state, or simply:
            self.s1_l = g * hp_l + bp_l;
            self.s2_l = g * bp_l + lp_l;

            // Right Channel (TPT SVF)
            let hp_r = (in_r - (g + k) * self.s1_r - self.s2_r) * denom_inv;
            let bp_r = g * hp_r + self.s1_r;
            let lp_r = g * bp_r + self.s2_r;

            self.s1_r = g * hp_r + bp_r;
            self.s2_r = g * bp_r + lp_r;

            // Apply output to buffer
            match self.mode {
                SimpleFilterMode::LowPass => {
                    buffer[l_idx] = lp_l;
                    buffer[r_idx] = lp_r;
                }
                SimpleFilterMode::HighPass => {
                    buffer[l_idx] = hp_l;
                    buffer[r_idx] = hp_r;
                }
                SimpleFilterMode::BandPass => {
                    buffer[l_idx] = bp_l;
                    buffer[r_idx] = bp_r;
                }
                SimpleFilterMode::Off => {}
            }
        }
    }
}

// ============================================================================
// VOICE
// ============================================================================

/// Voice state for polyphonic synthesis
#[derive(Clone, Debug)]
pub struct SynthVoice {
    pub note: u8,
    pub velocity: u8,
    pub phase: Vec<f32>, // Phase for each oscillator (flexible count)

    // Envelope State
    pub env_stage: EnvelopeStage,
    pub env_level: f32,
    pub env_timer: f32,
    pub release_start_level: f32,

    pub is_active: bool,
}

impl SynthVoice {
    /// Create a new voice for a note-on event
    pub fn new(note: u8, velocity: u8, num_oscillators: usize) -> Self {
        Self {
            note,
            velocity,
            phase: vec![0.0; num_oscillators],
            env_stage: EnvelopeStage::Attack,
            env_level: 0.0,
            env_timer: 0.0,
            release_start_level: 0.0,
            is_active: true,
        }
    }

    /// Trigger release phase (note-off)
    pub fn release(&mut self) {
        if self.is_active && self.env_stage != EnvelopeStage::Release {
            self.env_stage = EnvelopeStage::Release;
            self.env_timer = 0.0;
            self.release_start_level = self.env_level;
        }
    }

    /// Advance envelope by dt seconds, returns current level
    pub fn advance_envelope(&mut self, dt: f32, settings: &EnvelopeSettings) -> f32 {
        self.env_timer += dt;

        match self.env_stage {
            EnvelopeStage::Attack => {
                let rate = if settings.attack.get() < 0.001 {
                    1000.0
                } else {
                    1.0 / settings.attack.get()
                };
                self.env_level = (self.env_timer * rate).min(1.0);
                if self.env_level >= 1.0 {
                    self.env_level = 1.0;
                    self.env_stage = EnvelopeStage::Decay;
                    self.env_timer = 0.0;
                }
            }
            EnvelopeStage::Decay => {
                let rate = if settings.decay.get() < 0.001 {
                    1000.0
                } else {
                    1.0 / settings.decay.get()
                };
                let progress = (self.env_timer * rate).min(1.0);
                self.env_level = 1.0 - (progress * (1.0 - settings.sustain.get()));
                if progress >= 1.0 {
                    self.env_level = settings.sustain.get();
                    self.env_stage = EnvelopeStage::Sustain;
                }
            }
            EnvelopeStage::Sustain => {
                self.env_level = settings.sustain.get();
            }
            EnvelopeStage::Release => {
                let rate = if settings.release.get() < 0.001 {
                    1000.0
                } else {
                    1.0 / settings.release.get()
                };
                let progress = (self.env_timer * rate).min(1.0);
                self.env_level = self.release_start_level * (1.0 - progress);
                if progress >= 1.0 {
                    self.env_level = 0.0;
                    self.is_active = false;
                    self.env_stage = EnvelopeStage::Idle;
                }
            }
            EnvelopeStage::Idle => {
                self.env_level = 0.0;
                self.is_active = false;
            }
            EnvelopeStage::Delay => {
                // TODO: Implement this
            },
            EnvelopeStage::Hold => {
                // TODO: Implement this
            },
        }

        self.env_level
    }
}

// ============================================================================
// SYNTH BASE (Composition Pattern)
// ============================================================================

/// Common synthesizer state for all synth plugins.
/// Embed this in your synth struct instead of using procedural macros.
///
/// # Example
/// ```rust,ignore
/// pub struct MySynth {
///     pub base: SynthBase,
///     pub oscillators: [Oscillator; 3],  // Synth-specific
/// }
/// ```
#[derive(Clone, Debug)]
pub struct StandardSynthBase {
    pub sample_rate: f32,
    pub channels: usize,
    pub active_voices: Vec<SynthVoice>,
    pub voice_buffer: Vec<f32>,
    pub gain: f32,
    pub filter: SynthFilter,
    pub amp_envelope: EnvelopeSettings,
}

impl Default for StandardSynthBase {
    fn default() -> Self {
        Self::new(48000.0, 2)
    }
}

impl StandardSynthBase {
    /// Create a new SynthBase with the given sample rate
    pub fn new(sample_rate: f32, channels: usize) -> Self {
        Self {
            sample_rate,
            active_voices: Vec::with_capacity(16),
            voice_buffer: Vec::with_capacity(512),
            gain: 0.5,
            filter: SynthFilter::default(),
            amp_envelope: EnvelopeSettings::default(),
            channels,
        }
    }

    /// Prepare for playback
    pub fn prepare(&mut self, sample_rate: f32, channels: usize, max_buffer_size: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels;
        if self.voice_buffer.len() < max_buffer_size {
            self.voice_buffer.resize(max_buffer_size, 0.0);
        }
    }

    /// Reset all state (voices, filter)
    pub fn reset(&mut self) {
        self.active_voices.clear();
        self.filter.reset();
    }

    /// Apply filter and gain to a stereo buffer
    pub fn apply_filter_and_gain(&mut self, buffer: &mut [f32]) {
        self.filter.process(buffer, self.sample_rate);
        for sample in buffer.iter_mut() {
            *sample *= self.gain;
        }
    }

    /// Cleanup inactive voices (call at end of process)
    pub fn cleanup_voices(&mut self) {
        self.active_voices.retain(|v| v.is_active);
    }

    /// Handle base parameters (IDs 0-7). Returns true if parameter was handled.
    ///
    /// Parameter mapping:
    /// - 0: gain (0.0-1.0)
    /// - 1: filter_cutoff (20.0-20000.0 Hz)
    /// - 2: filter_resonance (0.0-0.95)
    /// - 3: filter_mode (0=LP, 1=HP, 2=BP, 3=Off)
    /// - 4: env_attack (0.001-5.0 seconds)
    /// - 5: env_decay (0.001-5.0 seconds)
    /// - 6: env_sustain (0.0-1.0)
    /// - 7: env_release (0.001-10.0 seconds)
    pub fn set_parameter(&mut self, id: u32, value: f32) -> bool {
        match id {
            0 => {
                self.gain = value.clamp(0.0, 1.0);
                true
            }
            1 => {
                self.filter.cutoff = value.clamp(20.0, 20000.0);
                true
            }
            2 => {
                self.filter.resonance = value.clamp(0.0, 0.95);
                true
            }
            3 => {
                self.filter.mode = SimpleFilterMode::from(value);
                true
            }
            4 => {
                self.amp_envelope.attack.set_base(value);
                true
            }
            5 => {
                self.amp_envelope.decay.set_base(value);
                true
            }
            6 => {
                self.amp_envelope.sustain.set_base(value);
                true
            }
            7 => {
                self.amp_envelope.release.set_base(value);
                true
            }
            _ => false,
        }
    }

    /// Get base parameters (IDs 0-7). Returns Some if parameter was handled.
    pub fn get_parameter(&self, id: u32) -> Option<f32> {
        match id {
            0 => Some(self.gain),
            1 => Some(self.filter.cutoff),
            2 => Some(self.filter.resonance),
            3 => Some(self.filter.mode as u32 as f32),
            4 => Some(self.amp_envelope.attack.get()),
            5 => Some(self.amp_envelope.decay.get()),
            6 => Some(self.amp_envelope.sustain.get()),
            7 => Some(self.amp_envelope.release.get()),
            _ => None,
        }
    }

    /// Get default parameter values for base parameters (IDs 0-7)
    pub fn default_parameters() -> IndexMap<u32, f32> {
        let mut map = IndexMap::new();
        map.insert(0, 0.5); // gain
        map.insert(1, 2000.0); // filter_cutoff
        map.insert(2, 0.2); // filter_resonance
        map.insert(3, 0.0); // filter_mode (LowPass)
        map.insert(4, 0.01); // env_attack
        map.insert(5, 0.2); // env_decay
        map.insert(6, 0.7); // env_sustain
        map.insert(7, 0.5); // env_release
        map
    }

    pub fn get_parameter_specs(&self) -> Vec<ParameterSpec> {
        vec![
            ParameterSpec::new_float(0, "Master Gain", "Output", self.gain, 0.0, 1.0, 0.5),
            // Filter
            ParameterSpec::new_float(
                1,
                "Cutoff",
                "Filter",
                self.filter.cutoff,
                20.0,
                20000.0,
                2000.0,
            ),
            ParameterSpec::new_float(
                2,
                "Resonance",
                "Filter",
                self.filter.resonance,
                0.0,
                0.95,
                0.2,
            ),
            ParameterSpec::new_choice(
                3,
                "Mode",
                "Filter",
                self.filter.mode as u32,
                vec![
                    "LowPass".into(),
                    "HighPass".into(),
                    "BandPass".into(),
                    "Off".into(),
                ],
                0,
            ),
            // Envelope
            ParameterSpec::new_float(
                4,
                "Attack",
                "Envelope",
                self.amp_envelope.attack.get(),
                0.001,
                5.0,
                0.01,
            ),
            ParameterSpec::new_float(
                5,
                "Decay",
                "Envelope",
                self.amp_envelope.decay.get(),
                0.001,
                5.0,
                0.2,
            ),
            ParameterSpec::new_float(
                6,
                "Sustain",
                "Envelope",
                self.amp_envelope.sustain.get(),
                0.0,
                1.0,
                0.7,
            ),
            ParameterSpec::new_float(
                7,
                "Release",
                "Envelope",
                self.amp_envelope.release.get(),
                0.001,
                10.0,
                0.5,
            ),
        ]
    }

    pub fn apply_automation(&mut self, id: u32, value: f32) {
        match id {
            0 => self.gain = value.clamp(0.0, 1.0),
            1 => self.filter.cutoff = value.clamp(20.0, 20000.0),
            2 => self.filter.resonance = value.clamp(0.0, 0.95),
            3 => self.filter.mode = SimpleFilterMode::from(value),
            4 => self.amp_envelope.attack.set_base(value),
            5 => self.amp_envelope.decay.set_base(value),
            6 => self.amp_envelope.sustain.set_base(value),
            7 => self.amp_envelope.release.set_base(value),
            _ => {},
        }
    }

    pub fn clear_automation(&mut self, id: u32) {
        // For base parameters, clearing automation means snapping back to the default value.
        // We can reuse the logic from default_parameters().
        if let Some(&default_val) = Self::default_parameters().get(&id) {
            self.set_parameter(id, default_val);
        }
    }

    /// Base parameter IDs reserved by SynthBase (0-7)
    pub const BASE_PARAM_COUNT: u32 = 8;
}
