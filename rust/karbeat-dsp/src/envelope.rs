// ============================================================================
// ENVELOPE (envelope.rs)
// ============================================================================

use karbeat_macros::karbeat_plugin;
use serde::{Deserialize, Serialize};

/// A common trait for any envelope settings struct.
/// Allows the AdsrProcessor to process both standard and advanced envelopes.
pub trait Envelope {
    fn attack_ms(&self) -> f32;
    fn decay_ms(&self) -> f32;
    fn sustain_level(&self) -> f32;
    fn release_ms(&self) -> f32;

    // Default implementation returns 1.0, but advanced envelopes can override this
    fn peak_level(&self) -> f32 {
        1.0
    }
}

/// Standard ADSR envelope settings
#[derive(Clone, Debug)]
#[karbeat_plugin]
pub struct EnvelopeSettings {
    #[param(
        id = "attack",
        name = "Attack",
        group = "Envelope",
        min = 1.0,
        max = 10000.0,
        default = 10.0,
        step = 1.0
    )]
    pub attack_ms: f32,

    #[param(
        id = "decay",
        name = "Decay",
        group = "Envelope",
        min = 1.0,
        max = 10000.0,
        default = 200.0,
        step = 1.0
    )]
    pub decay_ms: f32,

    #[param(
        id = "sustain",
        name = "Sustain",
        group = "Envelope",
        min = 0.0,
        max = 1.0,
        default = 0.7,
        step = 0.01
    )]
    pub sustain_level: f32,

    #[param(
        id = "release",
        name = "Release",
        group = "Envelope",
        min = 1.0,
        max = 10000.0,
        default = 500.0,
        step = 1.0
    )]
    pub release_ms: f32,
}

impl Default for EnvelopeSettings {
    fn default() -> Self {
        Self::new("Envelope")
    }
}

impl EnvelopeSettings {
    pub fn new(group: &str) -> Self {
        let mut env = Self::base_default();
        let g = group.to_string();
        env.attack_ms.group = g.clone();
        env.decay_ms.group = g.clone();
        env.sustain_level.group = g.clone();
        env.release_ms.group = g;
        env
    }
}

impl Envelope for EnvelopeSettings {
    fn attack_ms(&self) -> f32 {
        self.attack_ms.get()
    }
    fn decay_ms(&self) -> f32 {
        self.decay_ms.get()
    }
    fn sustain_level(&self) -> f32 {
        self.sustain_level.get()
    }
    fn release_ms(&self) -> f32 {
        self.release_ms.get()
    }
}

/// Advanced DAHDSR envelope settings with adjustable curve tensions
#[derive(Clone, Debug)]
#[karbeat_plugin]
pub struct AdvancedEnvelopeSettings {
    #[param(
        id = "delay",
        name = "Delay",
        group = "Envelope",
        min = 0.0,
        max = 5000.0,
        default = 0.0,
        step = 1.0
    )]
    pub delay_ms: f32,

    #[param(
        id = "attack",
        name = "Attack",
        group = "Envelope",
        min = 1.0,
        max = 10000.0,
        default = 10.0,
        step = 1.0
    )]
    pub attack_ms: f32,

    #[param(
        id = "hold",
        name = "Hold",
        group = "Envelope",
        min = 0.0,
        max = 5000.0,
        default = 0.0,
        step = 1.0
    )]
    pub hold_ms: f32,

    #[param(
        id = "decay",
        name = "Decay",
        group = "Envelope",
        min = 1.0,
        max = 10000.0,
        default = 200.0,
        step = 1.0
    )]
    pub decay_ms: f32,

    #[param(
        id = "release",
        name = "Release",
        group = "Envelope",
        min = 1.0,
        max = 10000.0,
        default = 500.0,
        step = 1.0
    )]
    pub release_ms: f32,

    #[param(
        id = "peak_level",
        name = "Peak Level",
        group = "Envelope",
        min = 0.0,
        max = 1.0,
        default = 1.0,
        step = 0.01
    )]
    pub peak_level: f32,

    #[param(
        id = "sustain",
        name = "Sustain",
        group = "Envelope",
        min = 0.0,
        max = 1.0,
        default = 0.7,
        step = 0.01
    )]
    pub sustain_level: f32,

    #[param(id = "attack_tension", name = "Attack Tension", group = "Envelope", min = -1.0, max = 1.0, default = 0.0, step = 0.01)]
    pub attack_tension: f32,

    #[param(id = "decay_tension", name = "Decay Tension", group = "Envelope", min = -1.0, max = 1.0, default = 0.5, step = 0.01)]
    pub decay_tension: f32,

    #[param(id = "release_tension", name = "Release Tension", group = "Envelope", min = -1.0, max = 1.0, default = 0.5, step = 0.01)]
    pub release_tension: f32,
}

impl AdvancedEnvelopeSettings {
    pub fn new(group: &str) -> Self {
        let mut env = Self::base_default();
        let g = group.to_string();
        env.delay_ms.group = g.clone();
        env.attack_ms.group = g.clone();
        env.hold_ms.group = g.clone();
        env.decay_ms.group = g.clone();
        env.release_ms.group = g.clone();
        env.peak_level.group = g.clone();
        env.sustain_level.group = g.clone();
        env.attack_tension.group = g.clone();
        env.decay_tension.group = g.clone();
        env.release_tension.group = g;
        env
    }

    #[inline(always)]
    pub fn apply_tension(progress: f32, tension: f32) -> f32 {
        if tension.abs() < 0.001 {
            return progress;
        }
        let k = tension * 10.0;
        ((progress * k).exp() - 1.0) / (k.exp() - 1.0)
    }
}

impl Default for AdvancedEnvelopeSettings {
    fn default() -> Self {
        Self::new("Advanced Envelope")
    }
}

impl Envelope for AdvancedEnvelopeSettings {
    fn attack_ms(&self) -> f32 {
        self.attack_ms.get()
    }
    fn decay_ms(&self) -> f32 {
        self.decay_ms.get()
    }
    fn sustain_level(&self) -> f32 {
        self.sustain_level.get()
    }
    fn release_ms(&self) -> f32 {
        self.release_ms.get()
    }
    fn peak_level(&self) -> f32 {
        self.peak_level.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdsrProcessor {
    pub state: EnvelopeState,
    pub current_level: f32,
    phase_time: f32,          // Time spent in the CURRENT phase
    release_start_level: f32, // Captures amplitude when note is released
    sample_rate: f32,
}

impl AdsrProcessor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            state: EnvelopeState::Idle,
            current_level: 0.0,
            phase_time: 0.0,
            release_start_level: 0.0,
            sample_rate,
        }
    }

    pub fn note_on(&mut self) {
        self.state = EnvelopeState::Attack;
        self.phase_time = 0.0;
    }

    pub fn note_off(&mut self) {
        self.state = EnvelopeState::Release;
        self.phase_time = 0.0;
        self.release_start_level = self.current_level;
    }

    pub fn process<T: Envelope>(&mut self, settings: &T) -> f32 {
        let dt = 1.0 / self.sample_rate;
        self.phase_time += dt;

        match self.state {
            EnvelopeState::Idle => {
                self.current_level = 0.0;
            }
            EnvelopeState::Attack => {
                let attack_sec = settings.attack_ms() / 1000.0;
                let peak = settings.peak_level();

                self.current_level = (self.phase_time / attack_sec) * peak;

                if self.phase_time >= attack_sec {
                    self.current_level = peak;
                    self.state = EnvelopeState::Decay;
                    self.phase_time = 0.0;
                }
            }
            EnvelopeState::Decay => {
                let decay_sec = settings.decay_ms() / 1000.0;
                let sustain = settings.sustain_level();
                let peak = settings.peak_level();

                // Interpolate from peak down to sustain_level
                let progress = self.phase_time / decay_sec;
                self.current_level = peak - (peak - sustain) * progress;

                if self.phase_time >= decay_sec {
                    self.current_level = sustain;
                    self.state = EnvelopeState::Sustain;
                    self.phase_time = 0.0;
                }
            }
            EnvelopeState::Sustain => {
                self.current_level = settings.sustain_level();
            }
            EnvelopeState::Release => {
                let release_sec = settings.release_ms() / 1000.0;
                let progress = self.phase_time / release_sec;

                // Fade from the captured note_off level down to 0
                self.current_level = self.release_start_level * (1.0 - progress);

                if self.phase_time >= release_sec {
                    self.current_level = 0.0;
                    self.state = EnvelopeState::Idle;
                    self.phase_time = 0.0;
                }
            }
        }

        self.current_level
    }
}
