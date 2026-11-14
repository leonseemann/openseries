//! Output safety stage: soft-clips peaks so summed channels never exceed full scale.

/// Below this absolute level samples pass unchanged.
const KNEE: f32 = 0.9;
/// Remaining headroom above the knee.
const HEADROOM: f32 = 1.0 - KNEE;

/// Soft-clips one sample into `-1.0..=1.0` (identity below [`KNEE`]).
#[must_use]
pub fn soft_clip(sample: f32) -> f32 {
    let magnitude = sample.abs();
    if magnitude <= KNEE {
        return sample;
    }
    let shaped = KNEE + HEADROOM * ((magnitude - KNEE) / HEADROOM).tanh();
    shaped.copysign(sample)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_below_knee_and_bounded_above() {
        assert!((soft_clip(0.5) - 0.5).abs() < f32::EPSILON);
        assert!((soft_clip(-0.9) + 0.9).abs() < f32::EPSILON);
        for x in [1.0_f32, 2.0, 10.0, 1e6] {
            assert!(soft_clip(x) <= 1.0 && soft_clip(x) > KNEE);
            assert!((soft_clip(-x) + soft_clip(x)).abs() < f32::EPSILON);
        }
    }
}
