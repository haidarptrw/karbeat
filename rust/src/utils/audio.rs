/// Audio utility functions for signal processing
///
/// Provides decibel-to-linear and linear-to-decibel conversions
/// following standard audio engineering conventions.

/// Convert decibels to linear gain.
///
/// - 0 dB = 1.0 (unity gain)
/// - -6 dB ≈ 0.5
/// - -∞ dB = 0.0 (silence)
/// - +6 dB ≈ 2.0
#[inline]
pub fn db_to_linear(db: f32) -> f32 {
    if db <= -80.0 {
        // Treat very low dB as silence to avoid denormals
        0.0
    } else {
        10.0_f32.powf(db / 20.0)
    }
}

/// Convert linear gain to decibels.
///
/// - 1.0 = 0 dB
/// - 0.5 ≈ -6 dB  
/// - 0.0 = -∞ dB
#[inline]
pub fn linear_to_db(linear: f32) -> f32 {
    if linear <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * linear.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_to_linear_unity() {
        assert!((db_to_linear(0.0) - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_db_to_linear_half() {
        // -6.02 dB = 0.5 linear
        assert!((db_to_linear(-6.02) - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_db_to_linear_silence() {
        assert_eq!(db_to_linear(-100.0), 0.0);
    }

    #[test]
    fn test_linear_to_db_unity() {
        assert!((linear_to_db(1.0) - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_linear_to_db_silence() {
        assert!(linear_to_db(0.0).is_infinite());
    }
}
