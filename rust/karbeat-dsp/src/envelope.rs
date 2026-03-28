// ============================================================================
// ENVELOPE
// ============================================================================

/// ADSR envelope settings
#[derive(Clone, Copy, Debug)]
pub struct EnvelopeSettings {
    pub attack: f32,  // Seconds
    pub decay: f32,   // Seconds
    pub sustain: f32, // 0.0 to 1.0
    pub release: f32, // Seconds
}

impl Default for EnvelopeSettings {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.2,
            sustain: 0.7,
            release: 0.5,
        }
    }
}

// ============================================================================
// ADVANCED ENVELOPE (DAHDSR + TENSION)
// ============================================================================

/// Advanced DAHDSR envelope settings with adjustable curve tensions
#[derive(Clone, Copy, Debug)]
pub struct AdvancedEnvelopeSettings {
    // Time parameters (Seconds)
    pub delay: f32,   // Time before attack starts
    pub attack: f32,  // Time to reach peak level
    pub hold: f32,    // Time to stay at peak level before decaying
    pub decay: f32,   // Time to reach sustain level
    pub release: f32, // Time to drop from sustain to 0.0

    // Level parameters (0.0 to 1.0)
    pub peak_level: f32, // Allows attacks that don't reach full 1.0 volume
    pub sustain: f32,

    // Curve Tension parameters (-1.0 to 1.0)
    // -1.0 = Logarithmic (Fast start, slow end)
    //  0.0 = Linear
    //  1.0 = Exponential (Slow start, fast end - great for punchy plucks)
    pub attack_tension: f32,
    pub decay_tension: f32,
    pub release_tension: f32,
}

impl Default for AdvancedEnvelopeSettings {
    fn default() -> Self {
        Self {
            delay: 0.0,
            attack: 0.01,
            hold: 0.0,
            decay: 0.2,
            release: 0.5,
            peak_level: 1.0,
            sustain: 0.7,
            attack_tension: 0.0,  // Linear attack
            decay_tension: 0.5,   // Slightly exponential decay for "punch"
            release_tension: 0.5, // Slightly exponential release
        }
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

impl AdvancedEnvelopeSettings {
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
