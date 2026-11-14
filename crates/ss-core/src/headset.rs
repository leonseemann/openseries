//! Headset settings as validated value objects.
//!
//! Types here describe *what* a user can configure, independent of how a particular
//! device encodes it on the wire (that is the job of a device driver crate).

use crate::RangeError;

/// Defines a newtype over `u8` that only accepts values in `$min..=$max`.
macro_rules! bounded_level {
    ($(#[$meta:meta])* $name:ident, $what:literal, $min:literal..=$max:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u8);

        impl $name {
            /// Smallest accepted value.
            pub const MIN: u8 = $min;
            /// Largest accepted value.
            pub const MAX: u8 = $max;

            /// Creates the value, validating its range.
            ///
            /// # Errors
            /// Returns [`RangeError`] if `value` is outside the accepted range.
            pub fn new(value: u8) -> Result<Self, RangeError> {
                if (Self::MIN..=Self::MAX).contains(&value) {
                    Ok(Self(value))
                } else {
                    Err(RangeError {
                        what: $what,
                        min: f32::from(Self::MIN),
                        max: f32::from(Self::MAX),
                        actual: f32::from(value),
                    })
                }
            }

            /// The raw level.
            #[must_use]
            pub const fn get(self) -> u8 {
                self.0
            }
        }
    };
}

bounded_level!(
    /// Strength of the transparency (hear-through) mode, 1 = weakest.
    TransparencyLevel,
    "transparency level",
    1..=10
);
bounded_level!(
    /// Hardware microphone gain, 1 = quietest.
    MicVolume,
    "microphone volume",
    1..=10
);
bounded_level!(
    /// Brightness of a device display (e.g. the base-station OLED), 1 = dimmest.
    DisplayBrightness,
    "display brightness",
    1..=10
);
bounded_level!(
    /// Brightness of the microphone-mute LED, 1 = dimmest.
    MicLedBrightness,
    "mic LED brightness",
    1..=10
);
bounded_level!(
    /// Percentage used by mixers and dials, 0–100.
    Percent,
    "percentage",
    0..=100
);

/// Noise-control mode of headsets with active noise cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoiseControl {
    /// No processing.
    Off,
    /// Ambient sound is passed through.
    Transparency,
    /// Active noise cancellation.
    NoiseCancelling,
}

/// How much of the user's own voice is fed back into the headphones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SidetoneLevel {
    /// No sidetone.
    Off,
    /// Low sidetone.
    Low,
    /// Medium sidetone.
    Medium,
    /// High sidetone.
    High,
}

/// Inactivity timeout after which the headset switches itself off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AutoOff {
    /// Never switch off automatically.
    Never,
    /// After 1 minute.
    After1Min,
    /// After 5 minutes.
    After5Min,
    /// After 10 minutes.
    After10Min,
    /// After 15 minutes.
    After15Min,
    /// After 30 minutes.
    After30Min,
    /// After 60 minutes.
    After60Min,
}

/// Built-in equalizer presets stored on the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EqPreset {
    /// Neutral response.
    Flat,
    /// Boosted lows.
    BassBoost,
    /// Emphasised mids for voices/footsteps.
    Focus,
    /// "V-shaped" response.
    Smiley,
    /// User-defined bands, see [`HardwareEq`].
    Custom,
}

/// Gain of one equalizer band in decibels.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct EqGain(f32);

impl EqGain {
    /// Lowest supported gain in dB.
    pub const MIN_DB: f32 = -10.0;
    /// Highest supported gain in dB.
    pub const MAX_DB: f32 = 10.0;
    /// Neutral gain.
    pub const ZERO: Self = Self(0.0);

    /// Creates a gain from decibels.
    ///
    /// # Errors
    /// Returns [`RangeError`] if `db` is not finite or outside `MIN_DB..=MAX_DB`.
    pub fn from_db(db: f32) -> Result<Self, RangeError> {
        if db.is_finite() && (Self::MIN_DB..=Self::MAX_DB).contains(&db) {
            Ok(Self(db))
        } else {
            Err(RangeError {
                what: "EQ gain (dB)",
                min: Self::MIN_DB,
                max: Self::MAX_DB,
                actual: db,
            })
        }
    }

    /// The gain in decibels.
    #[must_use]
    pub const fn db(self) -> f32 {
        self.0
    }
}

/// Number of bands of the on-device graphic equalizer.
pub const HARDWARE_EQ_BANDS: usize = 10;

/// Custom curve of the on-device 10-band equalizer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HardwareEq {
    /// Band gains from lowest to highest frequency.
    pub bands: [EqGain; HARDWARE_EQ_BANDS],
}

impl HardwareEq {
    /// A flat curve (all bands at 0 dB).
    pub const FLAT: Self = Self {
        bands: [EqGain::ZERO; HARDWARE_EQ_BANDS],
    };

    /// Returns `true` if every band is at 0 dB.
    #[must_use]
    pub fn is_flat(&self) -> bool {
        self.bands.iter().all(|g| g.db() == 0.0)
    }
}

/// Balance between the game and chat channel as reported by a ChatMix dial.
///
/// Both sides are 100 % at the centre position; turning the dial attenuates one side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChatMixBalance {
    /// Game channel level.
    pub game: Percent,
    /// Chat channel level.
    pub chat: Percent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_level_validates_range() {
        assert!(TransparencyLevel::new(0).is_err());
        assert_eq!(TransparencyLevel::new(1).unwrap().get(), 1);
        assert_eq!(TransparencyLevel::new(10).unwrap().get(), 10);
        assert!(TransparencyLevel::new(11).is_err());
        assert!(Percent::new(0).is_ok());
        assert!(Percent::new(101).is_err());
    }

    #[test]
    fn eq_gain_rejects_out_of_range_and_nan() {
        assert!(EqGain::from_db(-10.0).is_ok());
        assert!(EqGain::from_db(10.0).is_ok());
        assert!(EqGain::from_db(10.5).is_err());
        assert!(EqGain::from_db(f32::NAN).is_err());
    }

    #[test]
    fn flat_eq_is_flat() {
        assert!(HardwareEq::FLAT.is_flat());
        let mut eq = HardwareEq::FLAT;
        eq.bands[3] = EqGain::from_db(2.5).unwrap();
        assert!(!eq.is_flat());
    }
}
