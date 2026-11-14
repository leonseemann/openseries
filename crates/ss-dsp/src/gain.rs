//! Smoothed gain stage (SON-13 volume boost, channel volume, ChatMix).

use crate::level::db_to_linear;
use crate::{Format, Processor};

/// Time constant of gain changes; long enough to avoid zipper noise.
const SMOOTHING_MS: f32 = 10.0;

/// Linear gain with one-pole smoothing towards the target.
#[derive(Debug, Clone)]
pub struct Gain {
    format: Format,
    coefficient: f32,
    current: f32,
    target: f32,
}

impl Gain {
    /// A unity gain stage.
    #[must_use]
    pub fn new(format: Format) -> Self {
        Self {
            format,
            coefficient: format.smoothing_coefficient(SMOOTHING_MS),
            current: 1.0,
            target: 1.0,
        }
    }

    /// Sets the target as a linear factor (negative values are treated as 0).
    pub fn set_linear(&mut self, factor: f32) {
        self.target = factor.max(0.0);
    }

    /// Sets the target in dB.
    pub fn set_db(&mut self, db: f32) {
        self.set_linear(db_to_linear(db));
    }

    /// Jumps to the target immediately (e.g. before the stream starts).
    pub fn snap(&mut self) {
        self.current = self.target;
    }
}

impl Processor for Gain {
    fn process(&mut self, buffer: &mut [f32]) {
        let channels = self.format.channels();
        let (a, target) = (self.coefficient, self.target);
        for frame in buffer.chunks_exact_mut(channels) {
            self.current = target + a * (self.current - target);
            for sample in frame {
                *sample *= self.current;
            }
        }
    }

    fn reset(&mut self) {
        self.snap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{FS, assert_db, gain_at};

    #[test]
    fn reaches_target_and_ramps_smoothly() {
        let format = Format::new(FS, 2).unwrap();
        let mut gain = Gain::new(format);
        gain.set_db(-12.0);
        assert_db(gain_at(&mut gain, 440.0, 2), -12.0, 0.05);

        let mut ramp = Gain::new(format);
        ramp.set_linear(0.0);
        let mut buffer = vec![1.0; 2 * 64];
        ramp.process(&mut buffer);
        let left: Vec<f32> = buffer.iter().step_by(2).copied().collect();
        assert!(left.windows(2).all(|w| w[1] <= w[0]), "monotonic fade");
        assert!(left[0] > 0.9, "no jump on the first sample");
    }
}
