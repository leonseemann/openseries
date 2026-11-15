//! The Nova Pro on the device platform (ADR 0008, `docs/architecture/devices.md` step 3): maps the
//! protocol driver [`NovaProWireless`] to the device-neutral [`Device`] trait.
//!
//! Behaviour moved here from the app: the ChatMix hand-over is the [`Role::HostMixer`] role, and
//! writing an EQ curve also selects the Custom preset (otherwise the curve is not audible,
//! `docs/hardware/nova-pro-wireless.md`).

use std::time::{Duration, Instant};

use ss_core::headset::{
    AutoOff, DisplayBrightness, EqGain, EqPreset, HARDWARE_EQ_BANDS, HardwareEq, MicLedBrightness,
    MicVolume, NoiseControl, SidetoneLevel, TransparencyLevel,
};
use ss_device::{
    BatterySlot, Bus, Candidate, Change, ChoiceOption, Connection, Descriptor, Device, DeviceClass,
    DeviceError, DeviceInfo, DriverEntry, Feature, FeatureId, FeatureKind, HidMatch, Instance,
    LevelRange, Match, Role, Support, Value,
};

use crate::{Command, DriverError, Event, FirmwareVersions, NovaProWireless, ids};

/// Static description of the Nova Pro family.
pub static INFO: DeviceInfo = DeviceInfo {
    id: "arctis-nova-pro-wireless",
    name: "Arctis Nova Pro Wireless",
    class: DeviceClass::Headset,
    support: Support::Verified,
    hub_name: Some("base station"),
};

/// The base station's control interface (commands on `FFC0`, events on `FF00`, `ids`).
const MATCHES: &[Match] = &[Match::Hid(HidMatch {
    bus: Some(Bus::Usb),
    vendor_id: ids::VENDOR_ID,
    product_ids: &ids::PRODUCT_IDS,
    interface: Some(ids::CONTROL_INTERFACE),
    command_usage_page: ids::COMMAND_USAGE_PAGE,
    event_usage_page: Some(ids::EVENT_USAGE_PAGE),
})];

/// Registry entry of this driver (`ss-devices`).
pub static DRIVER: DriverEntry = DriverEntry {
    info: &INFO,
    matches: MATCHES,
    open,
};

/// Name of the base station's audio endpoints (output and input) on Windows and macOS.
const AUDIO_ENDPOINT: &[&str] = &["Arctis Nova Pro Wireless"];
/// Firmware part label (the four parts are not named, `nova-pro-wireless.md`).
const FIRMWARE_LABEL: &str = "Firmware";

const NOISE_CONTROL: &[ChoiceOption] = &[
    ChoiceOption {
        key: "off",
        label: "Off",
    },
    ChoiceOption {
        key: "transparency",
        label: "Transparency",
    },
    ChoiceOption {
        key: "anc",
        label: "Noise cancelling",
    },
];
const EQ_PRESETS: &[ChoiceOption] = &[
    ChoiceOption {
        key: "flat",
        label: "Flat",
    },
    ChoiceOption {
        key: "bass_boost",
        label: "Bass boost",
    },
    ChoiceOption {
        key: "focus",
        label: "Focus",
    },
    ChoiceOption {
        key: "smiley",
        label: "Smiley",
    },
    ChoiceOption {
        key: "custom",
        label: "Custom",
    },
];
const AUTO_OFF: &[ChoiceOption] = &[
    ChoiceOption {
        key: "never",
        label: "Never",
    },
    ChoiceOption {
        key: "1min",
        label: "1 minute",
    },
    ChoiceOption {
        key: "5min",
        label: "5 minutes",
    },
    ChoiceOption {
        key: "10min",
        label: "10 minutes",
    },
    ChoiceOption {
        key: "15min",
        label: "15 minutes",
    },
    ChoiceOption {
        key: "30min",
        label: "30 minutes",
    },
    ChoiceOption {
        key: "60min",
        label: "60 minutes",
    },
];
const SIDETONE: LevelRange = LevelRange {
    min: 0,
    max: 3,
    labels: Some(&["Off", "Low", "Medium", "High"]),
};
/// The 1–10 levels of transparency, mic volume and the brightness settings.
const TEN_STEPS: LevelRange = LevelRange::new(1, 10);
/// EQ resolution of the base station (`codec::eq_gain_to_byte`).
const EQ_STEP_DB: f32 = 0.5;

#[allow(clippy::unnecessary_wraps)] // signature of `ss_device::driver::OpenFn`
fn open(
    connection: Box<dyn Connection>,
    candidate: &Candidate,
) -> Result<Box<dyn Device>, DeviceError> {
    Ok(Box::new(NovaProDevice::new(
        connection,
        Instance::of(INFO.id, candidate),
    )))
}

const fn id(kind: FeatureKind) -> FeatureId {
    FeatureId::of(kind)
}

fn features() -> Vec<Feature> {
    let feature = |kind, descriptor, writable| Feature {
        id: id(kind),
        descriptor,
        writable,
    };
    vec![
        feature(
            FeatureKind::Battery,
            Descriptor::Battery {
                slots: &[BatterySlot::Main, BatterySlot::Spare],
            },
            false,
        ),
        feature(FeatureKind::WirelessLink, Descriptor::Toggle, false),
        feature(
            FeatureKind::AudioEndpoint,
            Descriptor::AudioEndpoint {
                outputs: AUDIO_ENDPOINT,
                inputs: AUDIO_ENDPOINT,
            },
            false,
        ),
        feature(
            FeatureKind::NoiseControl,
            Descriptor::Choice(NOISE_CONTROL),
            true,
        ),
        feature(
            FeatureKind::Transparency,
            Descriptor::Level(TEN_STEPS),
            true,
        ),
        feature(FeatureKind::Sidetone, Descriptor::Level(SIDETONE), true),
        feature(FeatureKind::MicVolume, Descriptor::Level(TEN_STEPS), true),
        feature(FeatureKind::MicMute, Descriptor::Toggle, false),
        feature(
            FeatureKind::MicLedBrightness,
            Descriptor::Level(TEN_STEPS),
            true,
        ),
        feature(
            FeatureKind::DisplayBrightness,
            Descriptor::Level(TEN_STEPS),
            true,
        ),
        feature(
            FeatureKind::Equalizer,
            Descriptor::Equalizer {
                #[allow(clippy::cast_possible_truncation)] // 10 bands
                bands: HARDWARE_EQ_BANDS as u8,
                centre_hz: None, // Unknown (nova-pro-wireless.md)
                min_db: EqGain::MIN_DB,
                max_db: EqGain::MAX_DB,
                step_db: EQ_STEP_DB,
            },
            true,
        ),
        feature(FeatureKind::EqPreset, Descriptor::Choice(EQ_PRESETS), true),
        feature(
            FeatureKind::BalanceDial,
            Descriptor::Balance { host_mixer: true },
            true,
        ),
        feature(FeatureKind::AutoOff, Descriptor::Choice(AUTO_OFF), true),
        feature(FeatureKind::Firmware, Descriptor::Versions, false),
        feature(FeatureKind::UsbReplug, Descriptor::Action, false),
    ]
}

/// A connected Nova Pro base station.
pub struct NovaProDevice<T: Connection> {
    driver: NovaProWireless<T>,
    instance: Instance,
    features: Vec<Feature>,
    /// Firmware versions (`None` if unknown), read once per connection.
    firmware: Option<FirmwareVersions>,
    firmware_read: bool,
    host_mixer: bool,
}

impl<T: Connection> NovaProDevice<T> {
    /// Wraps an opened connection.
    pub fn new(connection: T, instance: Instance) -> Self {
        Self {
            driver: NovaProWireless::new(connection),
            instance,
            features: features(),
            firmware: None,
            firmware_read: false,
            host_mixer: false,
        }
    }

    fn send(&mut self, command: Command) -> Result<(), DeviceError> {
        self.driver.send(command).map_err(device_error)
    }

    fn commands_for(id: FeatureId, value: &Value) -> Option<Vec<Command>> {
        let command = match (id.kind, value) {
            (FeatureKind::NoiseControl, Value::Choice(key)) => {
                Command::SetNoiseControl(noise_control_from(key)?)
            }
            (FeatureKind::Transparency, Value::Level(level)) => {
                Command::SetTransparency(TransparencyLevel::new(*level).ok()?)
            }
            (FeatureKind::Sidetone, Value::Level(level)) => {
                Command::SetSidetone(sidetone_from(*level)?)
            }
            (FeatureKind::MicVolume, Value::Level(level)) => {
                Command::SetMicVolume(MicVolume::new(*level).ok()?)
            }
            (FeatureKind::MicLedBrightness, Value::Level(level)) => {
                Command::SetMicLedBrightness(MicLedBrightness::new(*level).ok()?)
            }
            (FeatureKind::DisplayBrightness, Value::Level(level)) => {
                Command::SetDisplayBrightness(DisplayBrightness::new(*level).ok()?)
            }
            (FeatureKind::Equalizer, Value::Curve(gains)) => {
                // A curve is only audible with the Custom preset selected.
                return Some(vec![
                    Command::SetCustomEq(hardware_eq(gains)?),
                    Command::SetEqPreset(EqPreset::Custom),
                ]);
            }
            (FeatureKind::EqPreset, Value::Choice(key)) => {
                Command::SetEqPreset(eq_preset_from(key)?)
            }
            (FeatureKind::BalanceDial, Value::Balance(balance)) => Command::SetChatMix(*balance),
            (FeatureKind::AutoOff, Value::Choice(key)) => Command::SetAutoOff(auto_off_from(key)?),
            _ => return None,
        };
        Some(vec![command])
    }
}

impl<T: Connection> Device for NovaProDevice<T> {
    fn info(&self) -> &'static DeviceInfo {
        &INFO
    }

    fn instance(&self) -> &Instance {
        &self.instance
    }

    fn features(&self) -> &[Feature] {
        &self.features
    }

    fn refresh(&mut self) -> Result<Vec<(FeatureId, Value)>, DeviceError> {
        let status = self.driver.status().map_err(device_error)?;
        let settings = self.driver.settings().map_err(device_error)?;
        let display = self.driver.display().map_err(device_error)?;
        if !self.firmware_read {
            // Optional information: a failed read must not break the connection.
            self.firmware = self.driver.firmware().ok().flatten();
            self.firmware_read = true;
        }
        let mut values = vec![
            (
                id(FeatureKind::Battery),
                Value::Battery {
                    levels: vec![status.headset_battery, status.spare_battery],
                    charge: status.charge_state,
                },
            ),
            (id(FeatureKind::WirelessLink), Value::Bool(status.connected)),
            (id(FeatureKind::MicMute), Value::Bool(status.mic_muted)),
        ];
        let mut push = |kind, value: Option<Value>| {
            if let Some(value) = value {
                values.push((id(kind), value));
            }
        };
        push(
            FeatureKind::NoiseControl,
            status
                .noise_control
                .map(|m| Value::Choice(noise_control_key(m))),
        );
        push(
            FeatureKind::Transparency,
            status.transparency.map(|t| Value::Level(t.get())),
        );
        push(
            FeatureKind::AutoOff,
            status.auto_off.map(|a| Value::Choice(auto_off_key(a))),
        );
        push(
            FeatureKind::MicLedBrightness,
            status.mic_led.map(|l| Value::Level(l.get())),
        );
        push(
            FeatureKind::EqPreset,
            settings.eq_preset.map(|p| Value::Choice(eq_preset_key(p))),
        );
        push(
            FeatureKind::Equalizer,
            settings
                .custom_eq
                .map(|eq| Value::Curve(eq.bands.iter().map(|g| g.db()).collect())),
        );
        push(
            FeatureKind::MicVolume,
            settings.mic_volume.map(|v| Value::Level(v.get())),
        );
        push(
            FeatureKind::Sidetone,
            settings.sidetone.map(|s| Value::Level(sidetone_level(s))),
        );
        push(
            FeatureKind::BalanceDial,
            settings.chatmix.map(Value::Balance),
        );
        push(
            FeatureKind::DisplayBrightness,
            display.brightness.map(|b| Value::Level(b.get())),
        );
        push(
            FeatureKind::Firmware,
            self.firmware
                .map(|fw| Value::Versions(vec![(FIRMWARE_LABEL.to_owned(), fw.to_string())])),
        );
        Ok(values)
    }

    fn set(&mut self, id: FeatureId, value: &Value) -> Result<(), DeviceError> {
        self.feature(id)
            .ok_or(DeviceError::Unsupported(id))?
            .check(value)?;
        let commands = Self::commands_for(id, value).ok_or(DeviceError::Unsupported(id))?;
        for command in commands {
            self.send(command)?;
        }
        Ok(())
    }

    fn next_change(&mut self, timeout: Duration) -> Result<Option<Change>, DeviceError> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let Some(event) = self.driver.next_event(remaining).map_err(device_error)? else {
                return Ok(None);
            };
            if let Some(change) = change_of(event) {
                return Ok(Some(change));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
        }
    }

    fn set_role(&mut self, role: Role, active: bool) -> Result<(), DeviceError> {
        match role {
            Role::HostMixer => {
                // `06 49` hands the dial to the host; the mixer LED shows that a host mixes.
                self.send(Command::SetChatMixDial(active))?;
                self.send(Command::SetMixerPresent(active))?;
                self.host_mixer = active;
                Ok(())
            }
        }
    }

    fn shutdown(&mut self) -> Result<(), DeviceError> {
        if self.host_mixer {
            self.set_role(Role::HostMixer, false)?;
        }
        Ok(())
    }
}

/// The device-neutral change behind an unsolicited event (`None` = nothing the app models).
fn change_of(event: Event) -> Option<Change> {
    let value = |kind, value| Some(Change::Value(id(kind), value));
    match event {
        Event::NoiseControl(mode) => value(
            FeatureKind::NoiseControl,
            Value::Choice(noise_control_key(mode)),
        ),
        Event::MicMute(muted) => value(FeatureKind::MicMute, Value::Bool(muted)),
        Event::Sidetone(level) => value(FeatureKind::Sidetone, Value::Level(sidetone_level(level))),
        Event::MicVolume(volume) => value(FeatureKind::MicVolume, Value::Level(volume.get())),
        Event::EqPreset(preset) => {
            value(FeatureKind::EqPreset, Value::Choice(eq_preset_key(preset)))
        }
        Event::ChatMix(balance) => value(FeatureKind::BalanceDial, Value::Balance(balance)),
        Event::AutoOff(auto_off) => {
            value(FeatureKind::AutoOff, Value::Choice(auto_off_key(auto_off)))
        }
        Event::DisplayBrightness(brightness) => value(
            FeatureKind::DisplayBrightness,
            Value::Level(brightness.get()),
        ),
        // Link, battery and full blocks: re-read the complete state.
        Event::Status(_)
        | Event::Settings(_)
        | Event::Display(_)
        | Event::Battery(_)
        | Event::Connection { .. }
        | Event::EqBand { .. } => Some(Change::Refresh),
        Event::Volume(_)
        | Event::Firmware(_)
        | Event::Reply { .. }
        | Event::Unrecognized { .. } => None,
    }
}

fn device_error(error: DriverError) -> DeviceError {
    match error {
        DriverError::Transport(error) => DeviceError::Transport(error),
        other => DeviceError::Protocol(other.to_string()),
    }
}

fn hardware_eq(gains: &[f32]) -> Option<HardwareEq> {
    let mut eq = HardwareEq::FLAT;
    if gains.len() != eq.bands.len() {
        return None;
    }
    for (band, gain) in eq.bands.iter_mut().zip(gains) {
        *band = EqGain::from_db(*gain).ok()?;
    }
    Some(eq)
}

const fn noise_control_key(mode: NoiseControl) -> &'static str {
    match mode {
        NoiseControl::Off => "off",
        NoiseControl::Transparency => "transparency",
        NoiseControl::NoiseCancelling => "anc",
    }
}

fn noise_control_from(key: &str) -> Option<NoiseControl> {
    [
        NoiseControl::Off,
        NoiseControl::Transparency,
        NoiseControl::NoiseCancelling,
    ]
    .into_iter()
    .find(|m| noise_control_key(*m) == key)
}

const fn eq_preset_key(preset: EqPreset) -> &'static str {
    match preset {
        EqPreset::Flat => "flat",
        EqPreset::BassBoost => "bass_boost",
        EqPreset::Focus => "focus",
        EqPreset::Smiley => "smiley",
        EqPreset::Custom => "custom",
    }
}

fn eq_preset_from(key: &str) -> Option<EqPreset> {
    [
        EqPreset::Flat,
        EqPreset::BassBoost,
        EqPreset::Focus,
        EqPreset::Smiley,
        EqPreset::Custom,
    ]
    .into_iter()
    .find(|p| eq_preset_key(*p) == key)
}

const fn auto_off_key(auto_off: AutoOff) -> &'static str {
    match auto_off {
        AutoOff::Never => "never",
        AutoOff::After1Min => "1min",
        AutoOff::After5Min => "5min",
        AutoOff::After10Min => "10min",
        AutoOff::After15Min => "15min",
        AutoOff::After30Min => "30min",
        AutoOff::After60Min => "60min",
    }
}

fn auto_off_from(key: &str) -> Option<AutoOff> {
    [
        AutoOff::Never,
        AutoOff::After1Min,
        AutoOff::After5Min,
        AutoOff::After10Min,
        AutoOff::After15Min,
        AutoOff::After30Min,
        AutoOff::After60Min,
    ]
    .into_iter()
    .find(|a| auto_off_key(*a) == key)
}

const fn sidetone_level(level: SidetoneLevel) -> u8 {
    match level {
        SidetoneLevel::Off => 0,
        SidetoneLevel::Low => 1,
        SidetoneLevel::Medium => 2,
        SidetoneLevel::High => 3,
    }
}

const fn sidetone_from(level: u8) -> Option<SidetoneLevel> {
    match level {
        0 => Some(SidetoneLevel::Off),
        1 => Some(SidetoneLevel::Low),
        2 => Some(SidetoneLevel::Medium),
        3 => Some(SidetoneLevel::High),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss_core::headset::{ChatMixBalance, Percent};

    fn choice_keys(kind: FeatureKind) -> Vec<&'static str> {
        let features = features();
        let Some(Descriptor::Choice(options)) = features
            .iter()
            .find(|f| f.id.kind == kind)
            .map(|f| f.descriptor.clone())
        else {
            panic!("{kind:?} is not a choice");
        };
        options.iter().map(|o| o.key).collect()
    }

    #[test]
    fn every_choice_key_maps_to_a_command() {
        for key in choice_keys(FeatureKind::NoiseControl) {
            assert!(noise_control_from(key).is_some(), "{key}");
        }
        for key in choice_keys(FeatureKind::EqPreset) {
            assert!(eq_preset_from(key).is_some(), "{key}");
        }
        for key in choice_keys(FeatureKind::AutoOff) {
            assert!(auto_off_from(key).is_some(), "{key}");
        }
        for level in SIDETONE.min..=SIDETONE.max {
            assert_eq!(sidetone_from(level).map(sidetone_level), Some(level));
        }
    }

    #[test]
    fn writing_a_curve_also_selects_custom() {
        let commands = NovaProDevice::<Box<dyn Connection>>::commands_for(
            id(FeatureKind::Equalizer),
            &Value::Curve(vec![1.5; 10]),
        )
        .unwrap();
        assert_eq!(commands.len(), 2);
        assert!(matches!(commands[0], Command::SetCustomEq(_)));
        assert_eq!(commands[1], Command::SetEqPreset(EqPreset::Custom));
    }

    #[test]
    fn values_of_the_wrong_kind_are_unsupported() {
        assert!(
            NovaProDevice::<Box<dyn Connection>>::commands_for(
                id(FeatureKind::Sidetone),
                &Value::Bool(true)
            )
            .is_none()
        );
        assert!(
            NovaProDevice::<Box<dyn Connection>>::commands_for(
                id(FeatureKind::Firmware),
                &Value::Level(1)
            )
            .is_none()
        );
    }

    #[test]
    fn events_become_changes() {
        let balance = ChatMixBalance {
            game: Percent::saturating(100),
            chat: Percent::saturating(40),
        };
        assert_eq!(
            change_of(Event::ChatMix(balance)),
            Some(Change::Value(
                id(FeatureKind::BalanceDial),
                Value::Balance(balance)
            ))
        );
        assert_eq!(
            change_of(Event::NoiseControl(NoiseControl::NoiseCancelling)),
            Some(Change::Value(
                id(FeatureKind::NoiseControl),
                Value::Choice("anc")
            ))
        );
        assert_eq!(
            change_of(Event::Connection { connected: false }),
            Some(Change::Refresh)
        );
        assert_eq!(change_of(Event::Reply { opcode: 0x20 }), None);
    }

    #[test]
    fn the_base_station_interface_matches() {
        use ss_device::Collection;
        let candidate = Candidate {
            bus: Bus::Usb,
            vendor_id: ids::VENDOR_ID,
            product_id: 0x12E0,
            release: Some(0x0130),
            serial: None,
            interface: ids::CONTROL_INTERFACE,
            product: Some("Arctis Nova Pro Wireless".to_owned()),
            collections: vec![
                Collection {
                    usage_page: ids::COMMAND_USAGE_PAGE,
                    usage: 1,
                    path: "c".to_owned(),
                },
                Collection {
                    usage_page: ids::EVENT_USAGE_PAGE,
                    usage: 1,
                    path: "e".to_owned(),
                },
            ],
        };
        assert!(DRIVER.rule_for(&candidate).is_some());
        let audio_interface = Candidate {
            interface: 3,
            ..candidate
        };
        assert!(DRIVER.rule_for(&audio_interface).is_none());
    }
}
