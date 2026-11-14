//! Parametric equalizer built from [`Biquad`] sections.

use ss_core::eq::{PEQ_BANDS, ParametricEqSettings};

use crate::{Biquad, Format, Processor};

/// Ten-band parametric EQ applied to all channels.
#[derive(Debug, Clone)]
pub struct ParametricEq {
    bands: [Biquad; PEQ_BANDS],
    enabled: bool,
}

impl ParametricEq {
    /// Creates a flat (pass-through) EQ.
    #[must_use]
    pub fn new(format: Format) -> Self {
        Self {
            bands: std::array::from_fn(|_| Biquad::new(format)),
            enabled: false,
        }
    }

    /// Applies new settings. Disabled bands become pass-through; state is kept.
    pub fn set(&mut self, settings: &ParametricEqSettings) {
        self.enabled = settings.enabled;
        for (filter, band) in self.bands.iter_mut().zip(settings.bands.iter()) {
            if band.enabled {
                filter.design(
                    band.filter,
                    band.frequency.get(),
                    band.q.get(),
                    band.gain.get(),
                );
            } else {
                filter.set_coefficients(crate::Coefficients::IDENTITY);
            }
        }
    }
}

impl Processor for ParametricEq {
    fn process(&mut self, buffer: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for band in self.bands.iter_mut().filter(|b| !b.is_identity()) {
            band.process(buffer);
        }
    }

    fn reset(&mut self) {
        self.bands.iter_mut().for_each(Processor::reset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{FS, assert_db, gain_at};
    use ss_core::eq::{BandGain, FilterType, Frequency, QFactor};

    fn eq_with_band(index: usize, gain_db: f32) -> ParametricEqSettings {
        let mut settings = ParametricEqSettings::flat();
        let band = &mut settings.bands[index];
        band.filter = FilterType::Peaking;
        band.frequency = Frequency::new(1_000.0).unwrap();
        band.q = QFactor::new(1.0).unwrap();
        band.gain = BandGain::new(gain_db).unwrap();
        settings
    }

    #[test]
    fn flat_settings_are_transparent() {
        let mut eq = ParametricEq::new(Format::new(FS, 2).unwrap());
        eq.set(&ParametricEqSettings::flat());
        assert_db(gain_at(&mut eq, 1_000.0, 2), 0.0, 0.05);
    }

    #[test]
    fn single_band_applies_gain() {
        let mut eq = ParametricEq::new(Format::new(FS, 2).unwrap());
        eq.set(&eq_with_band(5, -9.0));
        assert_db(gain_at(&mut eq, 1_000.0, 2), -9.0, 0.3);
    }

    #[test]
    fn disabled_eq_or_band_bypasses() {
        let mut eq = ParametricEq::new(Format::new(FS, 1).unwrap());
        let mut settings = eq_with_band(5, 9.0);
        settings.enabled = false;
        eq.set(&settings);
        assert_db(gain_at(&mut eq, 1_000.0, 1), 0.0, 0.01);
        settings.enabled = true;
        settings.bands[5].enabled = false;
        eq.set(&settings);
        assert_db(gain_at(&mut eq, 1_000.0, 1), 0.0, 0.05);
    }
}
