//! Level conversions.

/// Level treated as silence when converting linear 0 to dB.
pub const SILENCE_DB: f32 = -120.0;

/// Decibels → linear amplitude factor.
#[must_use]
pub fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Linear amplitude → decibels (clamped at [`SILENCE_DB`]).
#[must_use]
pub fn linear_to_db(linear: f32) -> f32 {
    if linear <= 0.0 {
        return SILENCE_DB;
    }
    (20.0 * linear.log10()).max(SILENCE_DB)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_round_trip() {
        assert!((db_to_linear(0.0) - 1.0).abs() < 1e-6);
        assert!((db_to_linear(-6.0206) - 0.5).abs() < 1e-4);
        assert!((linear_to_db(db_to_linear(-17.5)) + 17.5).abs() < 1e-4);
        assert!((linear_to_db(0.0) - SILENCE_DB).abs() < f32::EPSILON);
    }
}
