//! Software parametric equalizer model (Sonar-style, `docs/architecture/dsp.md` §1).

use crate::RangeError;

/// Number of bands of the software parametric EQ.
pub const PEQ_BANDS: usize = 10;

/// Filter shape of one EQ band.
///
/// Discriminants equal Sonar's `FilterType` wire values (`docs/re/sonar.md` §3.2, Observed),
/// so presets can be imported unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FilterType {
    /// Band has no effect.
    Bypass = 0,
    /// Second-order low-pass.
    LowPass = 1,
    /// Second-order high-pass.
    HighPass = 2,
    /// Band-pass, peak gain = Q.
    BandPassPeakQ = 3,
    /// Band-pass, 0 dB peak gain.
    BandPassPeak0Db = 4,
    /// Notch (band-stop).
    Notch = 5,
    /// All-pass (phase only).
    AllPass = 6,
    /// Peaking (bell) filter.
    Peaking = 7,
    /// Low shelf.
    LowShelf = 8,
    /// High shelf.
    HighShelf = 9,
}

impl FilterType {
    /// Parses Sonar's numeric filter type.
    #[must_use]
    pub fn from_sonar_id(id: u8) -> Option<Self> {
        Some(match id {
            0 => Self::Bypass,
            1 => Self::LowPass,
            2 => Self::HighPass,
            3 => Self::BandPassPeakQ,
            4 => Self::BandPassPeak0Db,
            5 => Self::Notch,
            6 => Self::AllPass,
            7 => Self::Peaking,
            8 => Self::LowShelf,
            9 => Self::HighShelf,
            _ => return None,
        })
    }

    /// Sonar's numeric filter type.
    #[must_use]
    pub fn sonar_id(self) -> u8 {
        self as u8
    }

    /// Whether the band's gain parameter affects this filter shape.
    #[must_use]
    pub fn uses_gain(self) -> bool {
        matches!(self, Self::Peaking | Self::LowShelf | Self::HighShelf)
    }
}

/// Defines a validated `f32` newtype with an inclusive range.
macro_rules! bounded_f32 {
    ($(#[$meta:meta])* $name:ident, $what:literal, $min:literal..=$max:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        pub struct $name(f32);

        impl $name {
            /// Smallest accepted value.
            pub const MIN: f32 = $min;
            /// Largest accepted value.
            pub const MAX: f32 = $max;

            /// Creates the value, validating its range.
            ///
            /// # Errors
            /// Returns [`RangeError`] if `value` is not finite or out of range.
            pub fn new(value: f32) -> Result<Self, RangeError> {
                if value.is_finite() && (Self::MIN..=Self::MAX).contains(&value) {
                    Ok(Self(value))
                } else {
                    Err(RangeError { what: $what, min: Self::MIN, max: Self::MAX, actual: value })
                }
            }

            /// The raw value.
            #[must_use]
            pub const fn get(self) -> f32 {
                self.0
            }
        }
    };
}

bounded_f32!(
    /// Centre/corner frequency in Hz.
    Frequency,
    "frequency (Hz)",
    20.0..=20_000.0
);
bounded_f32!(
    /// Band gain in dB (our range choice; Sonar's limit is Unknown).
    BandGain,
    "band gain (dB)",
    -12.0..=12.0
);
bounded_f32!(
    /// Quality factor of a band.
    QFactor,
    "Q factor",
    0.1..=10.0
);

/// One band of the parametric EQ.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeqBand {
    /// Whether the band is active.
    pub enabled: bool,
    /// Filter shape.
    pub filter: FilterType,
    /// Centre/corner frequency.
    pub frequency: Frequency,
    /// Gain (used by peaking and shelving filters).
    pub gain: BandGain,
    /// Quality factor.
    pub q: QFactor,
}

/// Octave-spaced default centre frequencies (ISO 266 series).
const DEFAULT_FREQUENCIES: [f32; PEQ_BANDS] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 16_000.0,
];
/// Q giving roughly one-octave bandwidth.
const DEFAULT_Q: f32 = 1.41;

/// Complete parametric EQ configuration of one channel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParametricEqSettings {
    /// Master switch for the whole EQ.
    pub enabled: bool,
    /// The bands, low to high by default.
    pub bands: [PeqBand; PEQ_BANDS],
}

impl ParametricEqSettings {
    /// A flat EQ with octave-spaced peaking bands at 0 dB.
    #[must_use]
    pub fn flat() -> Self {
        Self {
            enabled: true,
            bands: DEFAULT_FREQUENCIES.map(|f| PeqBand {
                enabled: true,
                filter: FilterType::Peaking,
                frequency: Frequency(f),
                gain: BandGain(0.0),
                q: QFactor(DEFAULT_Q),
            }),
        }
    }
}

impl Default for ParametricEqSettings {
    fn default() -> Self {
        Self::flat()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sonar_ids_round_trip() {
        for id in 0..=9 {
            assert_eq!(FilterType::from_sonar_id(id).unwrap().sonar_id(), id);
        }
        assert!(FilterType::from_sonar_id(10).is_none());
    }

    #[test]
    fn bounded_values_validate() {
        assert!(Frequency::new(19.9).is_err());
        assert!(Frequency::new(1_000.0).is_ok());
        assert!(BandGain::new(12.5).is_err());
        assert!(QFactor::new(f32::INFINITY).is_err());
    }

    #[test]
    fn flat_default_has_ten_octave_bands() {
        let eq = ParametricEqSettings::default();
        assert!(eq.bands.iter().all(|b| b.gain.get() == 0.0));
        assert!((eq.bands[5].frequency.get() - 1_000.0).abs() < f32::EPSILON);
    }
}
