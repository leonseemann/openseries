//! Battery state of wireless devices.

use crate::RangeError;

/// Battery charge in percent (0–100).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BatteryLevel(u8);

impl BatteryLevel {
    /// Largest valid percentage.
    pub const MAX_PERCENT: u8 = 100;

    /// Creates a battery level from a percentage.
    ///
    /// # Errors
    /// Returns [`RangeError`] if `percent` is greater than 100.
    pub fn from_percent(percent: u8) -> Result<Self, RangeError> {
        if percent > Self::MAX_PERCENT {
            return Err(RangeError {
                what: "battery level",
                min: 0.0,
                max: f32::from(Self::MAX_PERCENT),
                actual: f32::from(percent),
            });
        }
        Ok(Self(percent))
    }

    /// The charge in percent.
    #[must_use]
    pub const fn percent(self) -> u8 {
        self.0
    }
}

/// What the power source of a battery is currently doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChargeState {
    /// Running on battery.
    Discharging,
    /// Connected to power and charging.
    Charging,
    /// Connected to power, not charging (usually full).
    PluggedNotCharging,
    /// Device is off or the state could not be determined.
    Unknown,
}

/// Battery report of a headset with a swappable spare battery (e.g. a base station slot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BatteryReport {
    /// Battery currently inside the headset.
    pub headset: BatteryLevel,
    /// Spare battery charging in the base station, if the device has one.
    pub spare: Option<BatteryLevel>,
    /// Charging state of the headset battery.
    pub state: ChargeState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_full_range() {
        assert_eq!(BatteryLevel::from_percent(0).unwrap().percent(), 0);
        assert_eq!(BatteryLevel::from_percent(100).unwrap().percent(), 100);
    }

    #[test]
    fn rejects_above_hundred() {
        assert!(BatteryLevel::from_percent(101).is_err());
    }
}
