//! The feature catalogue (`devices.md` §3): what a device can do.
//!
//! A feature has a **kind** (its meaning: noise control, sidetone, …) and a **descriptor** (its
//! shape: a level range, a list of choices, a toggle, an EQ curve, …). The UI picks a widget by
//! shape, so a new device with other presets or ranges needs no app change; a new *kind* only
//! needs a catalogue entry here (and a dedicated widget only if no existing shape fits).

use ss_core::battery::{BatteryLevel, ChargeState};
use ss_core::headset::ChatMixBalance;

/// The meaning of a feature. New kinds are added at the end; the app shows unknown kinds by
/// their descriptor shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum FeatureKind {
    /// Battery levels of one or more slots plus the charge state.
    Battery,
    /// Wireless link between the device and its base station / dongle.
    WirelessLink,
    /// OS audio endpoints of this device (used by AudioMix and the mic chain).
    AudioEndpoint,
    /// Noise control mode (off, transparency, noise cancelling, …).
    NoiseControl,
    /// Strength of the transparency (hear-through) mode.
    Transparency,
    /// Sidetone (own voice in the headphones).
    Sidetone,
    /// Hardware microphone volume.
    MicVolume,
    /// Microphone mute state.
    MicMute,
    /// Brightness of the mic-mute LED.
    MicLedBrightness,
    /// Brightness of a display (e.g. the base station OLED).
    DisplayBrightness,
    /// On-device equalizer curve.
    Equalizer,
    /// On-device equalizer preset.
    EqPreset,
    /// Hardware Game/Chat balance dial.
    BalanceDial,
    /// Inactivity auto-off timer.
    AutoOff,
    /// Firmware versions (read-only).
    Firmware,
    /// Software re-plug of the USB connection (handled by the OS layer of the app).
    UsbReplug,
}

/// Feature kind plus an index, so a device can have several of one kind (lighting zones, DPI
/// stages, batteries of separate parts …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FeatureId {
    /// What the feature means.
    pub kind: FeatureKind,
    /// Which one of this kind (usually 0).
    pub index: u8,
}

impl FeatureId {
    /// The first (usually only) feature of `kind`.
    #[must_use]
    pub const fn of(kind: FeatureKind) -> Self {
        Self { kind, index: 0 }
    }
}

/// An inclusive integer range with optional labels per step (e.g. sidetone Off/Low/Medium/High).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelRange {
    /// Lowest value.
    pub min: u8,
    /// Highest value.
    pub max: u8,
    /// One label per value from `min` to `max`, if the steps have names.
    pub labels: Option<&'static [&'static str]>,
}

impl LevelRange {
    /// A plain numeric range.
    #[must_use]
    pub const fn new(min: u8, max: u8) -> Self {
        Self {
            min,
            max,
            labels: None,
        }
    }

    /// The label of `value`, if the steps are named.
    #[must_use]
    pub fn label(&self, value: u8) -> Option<&'static str> {
        let labels = self.labels?;
        labels
            .get(usize::from(value.checked_sub(self.min)?))
            .copied()
    }
}

/// One option of a choice feature. `key` is stable (stored in configs and presets), `label` is
/// shown to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceOption {
    /// Stable identifier, e.g. `"anc"`.
    pub key: &'static str,
    /// User-facing text, e.g. `"Noise cancelling"`.
    pub label: &'static str,
}

/// A battery slot of a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BatterySlot {
    /// The battery that powers the device.
    Main,
    /// A spare battery (e.g. in the Nova Pro base station).
    Spare,
}

/// The shape and limits of a feature.
#[derive(Debug, Clone, PartialEq)]
pub enum Descriptor {
    /// An integer level ([`Value::Level`]).
    Level(LevelRange),
    /// One of several options ([`Value::Choice`]).
    Choice(&'static [ChoiceOption]),
    /// On/off ([`Value::Bool`]).
    Toggle,
    /// Battery slots ([`Value::Battery`]).
    Battery {
        /// The slots this device reports, in display order.
        slots: &'static [BatterySlot],
    },
    /// An EQ curve ([`Value::Curve`]).
    Equalizer {
        /// Number of bands.
        bands: u8,
        /// Centre frequency per band in Hz, if known.
        centre_hz: Option<&'static [f32]>,
        /// Lowest gain in dB.
        min_db: f32,
        /// Highest gain in dB.
        max_db: f32,
        /// Gain resolution in dB.
        step_db: f32,
    },
    /// A Game/Chat balance ([`Value::Balance`]).
    Balance {
        /// The app can take the dial over (host mixer role, [`crate::Role::HostMixer`]).
        host_mixer: bool,
    },
    /// OS audio endpoint names (no value).
    AudioEndpoint {
        /// Substrings of the output endpoint names.
        outputs: &'static [&'static str],
        /// Substrings of the input endpoint names.
        inputs: &'static [&'static str],
    },
    /// Part/version pairs ([`Value::Versions`]).
    Versions,
    /// Something the app triggers itself (no value).
    Action,
}

/// One feature of a device.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// Kind and index.
    pub id: FeatureId,
    /// Shape and limits.
    pub descriptor: Descriptor,
    /// The app may change it (otherwise read-only).
    pub writable: bool,
}

/// A feature value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// For [`Descriptor::Level`].
    Level(u8),
    /// For [`Descriptor::Choice`]: the option key.
    Choice(&'static str),
    /// For [`Descriptor::Toggle`].
    Bool(bool),
    /// For [`Descriptor::Battery`]: one level per slot (same order) and the charge state.
    Battery {
        /// Level per slot; `None` if the slot is empty or unknown.
        levels: Vec<Option<BatteryLevel>>,
        /// Charging state of the main battery.
        charge: ChargeState,
    },
    /// For [`Descriptor::Equalizer`]: gain per band in dB.
    Curve(Vec<f32>),
    /// For [`Descriptor::Balance`].
    Balance(ChatMixBalance),
    /// For [`Descriptor::Versions`]: (part, version).
    Versions(Vec<(String, String)>),
}

/// Why a value does not fit a feature.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValueError {
    /// The feature cannot be changed.
    #[error("{0:?} is read-only")]
    ReadOnly(FeatureId),
    /// The value has the wrong shape for the feature.
    #[error("wrong kind of value for {0:?}")]
    WrongShape(FeatureId),
    /// The value is outside the feature's limits.
    #[error("value out of range for {0:?}")]
    OutOfRange(FeatureId),
}

impl Feature {
    /// Checks that `value` may be written to this feature (shape and limits).
    ///
    /// # Errors
    /// [`ValueError`] if the feature is read-only or the value does not fit.
    pub fn check(&self, value: &Value) -> Result<(), ValueError> {
        let id = self.id;
        if !self.writable {
            return Err(ValueError::ReadOnly(id));
        }
        let fits = match (&self.descriptor, value) {
            (Descriptor::Level(range), Value::Level(level)) => {
                Some((range.min..=range.max).contains(level))
            }
            (Descriptor::Choice(options), Value::Choice(key)) => {
                Some(options.iter().any(|o| o.key == *key))
            }
            (Descriptor::Toggle, Value::Bool(_))
            | (Descriptor::Balance { .. }, Value::Balance(_)) => Some(true),
            (
                Descriptor::Equalizer {
                    bands,
                    min_db,
                    max_db,
                    ..
                },
                Value::Curve(gains),
            ) => Some(
                gains.len() == usize::from(*bands)
                    && gains
                        .iter()
                        .all(|g| g.is_finite() && (*min_db..=*max_db).contains(g)),
            ),
            _ => None,
        };
        match fits {
            Some(true) => Ok(()),
            Some(false) => Err(ValueError::OutOfRange(id)),
            None => Err(ValueError::WrongShape(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIDETONE: LevelRange = LevelRange {
        min: 0,
        max: 3,
        labels: Some(&["Off", "Low", "Medium", "High"]),
    };
    const MODES: &[ChoiceOption] = &[
        ChoiceOption {
            key: "off",
            label: "Off",
        },
        ChoiceOption {
            key: "anc",
            label: "Noise cancelling",
        },
    ];

    fn feature(kind: FeatureKind, descriptor: Descriptor) -> Feature {
        Feature {
            id: FeatureId::of(kind),
            descriptor,
            writable: true,
        }
    }

    #[test]
    fn levels_are_checked_against_the_range() {
        let f = feature(FeatureKind::Sidetone, Descriptor::Level(SIDETONE));
        assert_eq!(f.check(&Value::Level(3)), Ok(()));
        assert!(matches!(
            f.check(&Value::Level(4)),
            Err(ValueError::OutOfRange(_))
        ));
        assert!(matches!(
            f.check(&Value::Bool(true)),
            Err(ValueError::WrongShape(_))
        ));
        assert_eq!(SIDETONE.label(2), Some("Medium"));
        assert_eq!(SIDETONE.label(9), None);
    }

    #[test]
    fn choices_need_a_known_key() {
        let f = feature(FeatureKind::NoiseControl, Descriptor::Choice(MODES));
        assert_eq!(f.check(&Value::Choice("anc")), Ok(()));
        assert!(f.check(&Value::Choice("turbo")).is_err());
    }

    #[test]
    fn curves_need_the_band_count_and_range() {
        let f = feature(
            FeatureKind::Equalizer,
            Descriptor::Equalizer {
                bands: 3,
                centre_hz: None,
                min_db: -10.0,
                max_db: 10.0,
                step_db: 0.5,
            },
        );
        assert_eq!(f.check(&Value::Curve(vec![0.0, 5.0, -10.0])), Ok(()));
        assert!(f.check(&Value::Curve(vec![0.0, 5.0])).is_err());
        assert!(f.check(&Value::Curve(vec![0.0, 11.0, 0.0])).is_err());
        assert!(f.check(&Value::Curve(vec![0.0, f32::NAN, 0.0])).is_err());
    }

    #[test]
    fn read_only_features_refuse_writes() {
        let mut f = feature(FeatureKind::MicMute, Descriptor::Toggle);
        f.writable = false;
        assert!(matches!(
            f.check(&Value::Bool(true)),
            Err(ValueError::ReadOnly(_))
        ));
    }
}
