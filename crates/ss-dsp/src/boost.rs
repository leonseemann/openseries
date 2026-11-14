//! Bass / treble / voice-clarity boosts (SON-11).
//!
//! Corner frequencies are our design choice; the original's are Unknown
//! (`docs/architecture/dsp.md` §1).

use ss_core::eq::FilterType;

use crate::{Biquad, Format, Processor};

/// Low-shelf corner of the bass boost.
const BASS_HZ: f32 = 100.0;
/// High-shelf corner of the treble boost.
const TREBLE_HZ: f32 = 8_000.0;
/// Centre of the voice-clarity bell.
const CLARITY_HZ: f32 = 2_500.0;
/// Shelf slope (Butterworth Q).
const SHELF_Q: f32 = std::f32::consts::FRAC_1_SQRT_2;
/// Bandwidth of the clarity bell.
const CLARITY_Q: f32 = 1.0;
/// Largest boost in dB.
pub const MAX_BOOST_DB: f32 = 12.0;

/// Boost amounts in dB (0 = off), clamped to `0..=MAX_BOOST_DB`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoostSettings {
    /// Low-frequency boost.
    pub bass_db: f32,
    /// High-frequency boost.
    pub treble_db: f32,
    /// Presence boost for voices.
    pub clarity_db: f32,
}

/// The three boost filters in series.
#[derive(Debug, Clone)]
pub struct Boosts {
    bass: Biquad,
    treble: Biquad,
    clarity: Biquad,
}

impl Boosts {
    /// Creates the boosts, all off.
    #[must_use]
    pub fn new(format: Format) -> Self {
        Self {
            bass: Biquad::new(format),
            treble: Biquad::new(format),
            clarity: Biquad::new(format),
        }
    }

    /// Applies new boost amounts.
    pub fn set(&mut self, settings: BoostSettings) {
        Self::configure(
            &mut self.bass,
            FilterType::LowShelf,
            BASS_HZ,
            SHELF_Q,
            settings.bass_db,
        );
        Self::configure(
            &mut self.treble,
            FilterType::HighShelf,
            TREBLE_HZ,
            SHELF_Q,
            settings.treble_db,
        );
        Self::configure(
            &mut self.clarity,
            FilterType::Peaking,
            CLARITY_HZ,
            CLARITY_Q,
            settings.clarity_db,
        );
    }

    fn configure(filter: &mut Biquad, kind: FilterType, hz: f32, q: f32, db: f32) {
        let db = db.clamp(0.0, MAX_BOOST_DB);
        if db == 0.0 {
            filter.set_coefficients(crate::Coefficients::IDENTITY);
        } else {
            filter.design(kind, hz, q, db);
        }
    }

    fn filters(&mut self) -> [&mut Biquad; 3] {
        [&mut self.bass, &mut self.treble, &mut self.clarity]
    }
}

impl Processor for Boosts {
    fn process(&mut self, buffer: &mut [f32]) {
        for filter in self.filters() {
            if !filter.is_identity() {
                filter.process(buffer);
            }
        }
    }

    fn reset(&mut self) {
        self.filters().into_iter().for_each(Processor::reset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{FS, assert_db, gain_at};

    #[test]
    fn each_boost_targets_its_region() {
        let mut boosts = Boosts::new(Format::new(FS, 2).unwrap());
        boosts.set(BoostSettings {
            bass_db: 6.0,
            ..BoostSettings::default()
        });
        assert_db(gain_at(&mut boosts, 30.0, 2), 6.0, 0.6);
        assert_db(gain_at(&mut boosts, 5_000.0, 2), 0.0, 0.3);

        boosts.set(BoostSettings {
            clarity_db: 6.0,
            ..BoostSettings::default()
        });
        assert_db(gain_at(&mut boosts, CLARITY_HZ, 2), 6.0, 0.3);
    }

    #[test]
    fn negative_and_excessive_values_are_clamped() {
        let mut boosts = Boosts::new(Format::new(FS, 1).unwrap());
        boosts.set(BoostSettings {
            treble_db: -6.0,
            ..BoostSettings::default()
        });
        assert_db(gain_at(&mut boosts, 15_000.0, 1), 0.0, 0.01);
        boosts.set(BoostSettings {
            treble_db: 40.0,
            ..BoostSettings::default()
        });
        assert!(gain_at(&mut boosts, 18_000.0, 1) <= MAX_BOOST_DB + 0.5);
    }
}
