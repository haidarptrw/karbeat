// ============================================================================
// ENVELOPE
// ============================================================================

use karbeat_macros::AutoParams;
use karbeat_plugin_types::parameter::Param;

/// ADSR envelope settings
#[derive(Clone, Debug, AutoParams)]
pub struct EnvelopeSettings {
    pub attack: Param<f32>,  // Seconds
    pub decay: Param<f32>,   // Seconds
    pub sustain: Param<f32>, // 0.0 to 1.0
    pub release: Param<f32>, // Seconds
}

impl EnvelopeSettings {
    /// Create a new standard ADSR envelope building block.
    pub fn new(id_start: u32, group: &str) -> Self {
        Self {
            // ID, Name, Group, Default, Min, Max, Step
            attack: Param::new_float(id_start, "Attack", group, 0.01, 0.001, 10.0, 0.001),
            decay: Param::new_float(id_start + 1, "Decay", group, 0.2, 0.001, 10.0, 0.001),
            sustain: Param::new_float(id_start + 2, "Sustain", group, 0.7, 0.0, 1.0, 0.01),
            release: Param::new_float(id_start + 3, "Release", group, 0.5, 0.001, 10.0, 0.001),
        }
    }
}

impl Default for EnvelopeSettings {
    fn default() -> Self {
        Self::new(100, "Envelope")
    }
}

// ============================================================================
// ADVANCED ENVELOPE (DAHDSR + TENSION)
// ============================================================================

/// Advanced DAHDSR envelope settings with adjustable curve tensions
#[derive(Clone, Debug, AutoParams)]
pub struct AdvancedEnvelopeSettings {
    // Time parameters (Seconds)
    pub delay: Param<f32>,   // Time before attack starts
    pub attack: Param<f32>,  // Time to reach peak level
    pub hold: Param<f32>,    // Time to stay at peak level before decaying
    pub decay: Param<f32>,   // Time to reach sustain level
    pub release: Param<f32>, // Time to drop from sustain to 0.0

    // Level parameters (0.0 to 1.0)
    pub peak_level: Param<f32>, // Allows attacks that don't reach full 1.0 volume
    pub sustain: Param<f32>,

    // Curve Tension parameters (-1.0 to 1.0)
    // -1.0 = Logarithmic (Fast start, slow end)
    //  0.0 = Linear
    //  1.0 = Exponential (Slow start, fast end - great for punchy plucks)
    pub attack_tension: Param<f32>,
    pub decay_tension: Param<f32>,
    pub release_tension: Param<f32>,
}

impl AdvancedEnvelopeSettings {
    /// Create a new advanced DAHDSR envelope building block.
    pub fn new(id_start: u32, group: &str) -> Self {
        Self {
            delay: Param::new_float(id_start, "Delay", group, 0.0, 0.0, 5.0, 0.01),
            attack: Param::new_float(id_start + 1, "Attack", group, 0.01, 0.001, 10.0, 0.001),
            hold: Param::new_float(id_start + 2, "Hold", group, 0.0, 0.0, 5.0, 0.01),
            decay: Param::new_float(id_start + 3, "Decay", group, 0.2, 0.001, 10.0, 0.001),
            release: Param::new_float(id_start + 4, "Release", group, 0.5, 0.001, 10.0, 0.001),
            
            peak_level: Param::new_float(id_start + 5, "Peak Level", group, 1.0, 0.0, 1.0, 0.01),
            sustain: Param::new_float(id_start + 6, "Sustain", group, 0.7, 0.0, 1.0, 0.01),
            
            attack_tension: Param::new_float(id_start + 7, "Attack Tension", group, 0.0, -1.0, 1.0, 0.01),
            decay_tension: Param::new_float(id_start + 8, "Decay Tension", group, 0.5, -1.0, 1.0, 0.01),
            release_tension: Param::new_float(id_start + 9, "Release Tension", group, 0.5, -1.0, 1.0, 0.01),
        }
    }

    /// Morphs a linear progress value (0.0 to 1.0) into a curved value based on tension.
    /// Tension range: -1.0 (Logarithmic) to 1.0 (Exponential).
    #[inline(always)]
    pub fn apply_tension(progress: f32, tension: f32) -> f32 {
        // Fast path for strictly linear configurations to save CPU
        if tension.abs() < 0.001 {
            return progress;
        }

        // Scale tension to a useful mathematical range for the exponential function.
        // A factor of 10.0 provides a dramatic but musical curve range.
        let k = tension * 10.0;
        
        // The standard exponential mapping formula
        ( (progress * k).exp() - 1.0 ) / ( k.exp() - 1.0 )
    }
}

impl Default for AdvancedEnvelopeSettings {
    fn default() -> Self {
        Self::new(200, "Advanced Envelope")
    }
}

/// Envelope stage for voice processing state machine
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum EnvelopeStage {
    #[default]
    Idle,
    Delay,
    Attack,
    Hold,
    Decay,
    Sustain,
    Release,
}