//! Second-order IIR sections (RBJ "Audio EQ Cookbook" designs, transposed direct form II).

use std::f32::consts::TAU;

use ss_core::eq::FilterType;

use crate::{Format, MAX_CHANNELS, Processor};

/// Highest design frequency as a fraction of the sample rate (just below Nyquist).
const MAX_FREQUENCY_RATIO: f32 = 0.49;

/// Normalised biquad coefficients (`a0 = 1`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Coefficients {
    /// Pass-through.
    pub const IDENTITY: Self = Self {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };

    /// Designs a filter. `gain_db` is used only by peaking and shelving shapes.
    /// Frequencies above `0.49·fs` are clamped; a non-positive `q` yields [`Self::IDENTITY`].
    #[must_use]
    pub fn design(
        filter: FilterType,
        sample_rate: f32,
        frequency: f32,
        q: f32,
        gain_db: f32,
    ) -> Self {
        if q <= 0.0 || frequency <= 0.0 || filter == FilterType::Bypass {
            return Self::IDENTITY;
        }
        let f0 = frequency.min(sample_rate * MAX_FREQUENCY_RATIO);
        let w0 = TAU * f0 / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a = 10.0_f32.powf(gain_db / 40.0);

        // (b0, b1, b2, a0, a1, a2) per the cookbook.
        let raw = match filter {
            FilterType::Bypass => return Self::IDENTITY,
            FilterType::LowPass => {
                let b = f32::midpoint(1.0, -cos);
                (b, 1.0 - cos, b, 1.0 + alpha, -2.0 * cos, 1.0 - alpha)
            }
            FilterType::HighPass => {
                let b = f32::midpoint(1.0, cos);
                (b, -(1.0 + cos), b, 1.0 + alpha, -2.0 * cos, 1.0 - alpha)
            }
            FilterType::BandPassPeakQ => (
                sin / 2.0,
                0.0,
                -sin / 2.0,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            FilterType::BandPassPeak0Db => {
                (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos, 1.0 - alpha)
            }
            FilterType::Notch => (1.0, -2.0 * cos, 1.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
            FilterType::AllPass => (
                1.0 - alpha,
                -2.0 * cos,
                1.0 + alpha,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            FilterType::Peaking => (
                1.0 + alpha * a,
                -2.0 * cos,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos,
                1.0 - alpha / a,
            ),
            FilterType::LowShelf => {
                let k = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * cos + k),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                    a * ((a + 1.0) - (a - 1.0) * cos - k),
                    (a + 1.0) + (a - 1.0) * cos + k,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                    (a + 1.0) + (a - 1.0) * cos - k,
                )
            }
            FilterType::HighShelf => {
                let k = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cos + k),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                    a * ((a + 1.0) + (a - 1.0) * cos - k),
                    (a + 1.0) - (a - 1.0) * cos + k,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos),
                    (a + 1.0) - (a - 1.0) * cos - k,
                )
            }
        };
        let (b0, b1, b2, a0, a1, a2) = raw;
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

/// One biquad applied to every channel of an interleaved stream.
#[derive(Debug, Clone)]
pub struct Biquad {
    format: Format,
    coefficients: Coefficients,
    /// Transposed-direct-form-II state `(s1, s2)` per channel.
    state: [[f32; 2]; MAX_CHANNELS],
}

impl Biquad {
    /// A pass-through filter for `format`.
    #[must_use]
    pub fn new(format: Format) -> Self {
        Self {
            format,
            coefficients: Coefficients::IDENTITY,
            state: [[0.0; 2]; MAX_CHANNELS],
        }
    }

    /// Replaces the coefficients, keeping the state (no click on parameter changes).
    pub fn set_coefficients(&mut self, coefficients: Coefficients) {
        self.coefficients = coefficients;
    }

    /// Redesigns the filter.
    pub fn design(&mut self, filter: FilterType, frequency: f32, q: f32, gain_db: f32) {
        self.coefficients =
            Coefficients::design(filter, self.format.sample_rate(), frequency, q, gain_db);
    }

    /// Whether the filter currently passes audio unchanged.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.coefficients == Coefficients::IDENTITY
    }
}

impl Processor for Biquad {
    fn process(&mut self, buffer: &mut [f32]) {
        let c = self.coefficients;
        let channels = self.format.channels();
        for frame in buffer.chunks_exact_mut(channels) {
            for (sample, [s1, s2]) in frame.iter_mut().zip(self.state.iter_mut()) {
                let x = *sample;
                let y = c.b0 * x + *s1;
                *s1 = c.b1 * x - c.a1 * y + *s2;
                *s2 = c.b2 * x - c.a2 * y;
                *sample = y;
            }
        }
    }

    fn reset(&mut self) {
        self.state = [[0.0; 2]; MAX_CHANNELS];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{FS, assert_db, gain_at};

    fn filter(kind: FilterType, f0: f32, q: f32, gain_db: f32) -> Biquad {
        let mut bq = Biquad::new(Format::new(FS, 2).unwrap());
        bq.design(kind, f0, q, gain_db);
        bq
    }

    #[test]
    fn peaking_boosts_at_centre_only() {
        let mut bq = filter(FilterType::Peaking, 1_000.0, 1.0, 6.0);
        assert_db(gain_at(&mut bq, 1_000.0, 2), 6.0, 0.3);
        assert_db(gain_at(&mut bq, 100.0, 2), 0.0, 0.5);
        assert_db(gain_at(&mut bq, 10_000.0, 2), 0.0, 0.5);
    }

    #[test]
    fn shelves_affect_their_side() {
        let mut low = filter(FilterType::LowShelf, 200.0, 0.707, -6.0);
        assert_db(gain_at(&mut low, 40.0, 2), -6.0, 0.5);
        assert_db(gain_at(&mut low, 5_000.0, 2), 0.0, 0.3);
        let mut high = filter(FilterType::HighShelf, 4_000.0, 0.707, 6.0);
        assert_db(gain_at(&mut high, 15_000.0, 2), 6.0, 0.6);
        assert_db(gain_at(&mut high, 200.0, 2), 0.0, 0.3);
    }

    #[test]
    fn pass_and_stop_filters() {
        let mut lp = filter(FilterType::LowPass, 1_000.0, 0.707, 0.0);
        assert_db(gain_at(&mut lp, 100.0, 2), 0.0, 0.3);
        assert!(gain_at(&mut lp, 10_000.0, 2) < -35.0);
        let mut hp = filter(FilterType::HighPass, 1_000.0, 0.707, 0.0);
        let hp_db = gain_at(&mut hp, 100.0, 2);
        assert!(hp_db < -35.0, "high-pass at 100 Hz: {hp_db} dB");
        let mut notch = filter(FilterType::Notch, 1_000.0, 2.0, 0.0);
        assert!(gain_at(&mut notch, 1_000.0, 2) < -30.0);
        let mut bp = filter(FilterType::BandPassPeak0Db, 1_000.0, 2.0, 0.0);
        assert_db(gain_at(&mut bp, 1_000.0, 2), 0.0, 0.3);
        let mut ap = filter(FilterType::AllPass, 1_000.0, 1.0, 0.0);
        assert_db(gain_at(&mut ap, 3_000.0, 2), 0.0, 0.2);
    }

    #[test]
    fn bypass_and_invalid_parameters_are_identity() {
        assert!(filter(FilterType::Bypass, 1_000.0, 1.0, 6.0).is_identity());
        assert!(filter(FilterType::Peaking, 1_000.0, 0.0, 6.0).is_identity());
    }

    #[test]
    fn frequency_above_nyquist_is_clamped_and_stable() {
        let mut bq = filter(FilterType::Peaking, 30_000.0, 1.0, 6.0);
        assert!(gain_at(&mut bq, 1_000.0, 1).is_finite());
    }
}
