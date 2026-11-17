//! Desktop notifications (CORE-15, NTU-02; `docs/architecture/app.md` §2).
//!
//! [`AlertTracker`] turns successive state snapshots into notable changes per device (pure,
//! unit-tested); [`spawn`] polls the app state on a background thread, so alerts also appear while
//! the window is hidden, and shows them as OS notifications when enabled in the settings.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Weak;
use std::time::Duration;

use ss_core::battery::ChargeState;

use crate::audio::supervisor::ServiceStatus;
use crate::core::{AppCore, Snapshot};
use crate::i18n::tr;

/// How often the state is checked.
const POLL_INTERVAL: Duration = Duration::from_secs(1);
/// Battery level at or below this (percent) while discharging triggers a warning.
pub const LOW_BATTERY_PERCENT: u8 = 25;
/// The low-battery warning re-arms once the level is back at or above this (or charging).
const REARM_BATTERY_PERCENT: u8 = 50;
/// Title of every notification.
const APP_NAME: &str = "OpenSeries";

/// Something worth a notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alert {
    /// The wireless part of a device linked (e.g. headset to base station).
    Linked {
        /// Device name.
        device: &'static str,
    },
    /// The wireless part is off or out of range.
    Unlinked {
        /// Device name.
        device: &'static str,
    },
    /// A device vanished (unplugged, or its input switched to another computer).
    Gone {
        /// Device name.
        device: &'static str,
        /// What the computer is plugged into ("base station", "dongle", "device").
        hub: &'static str,
    },
    /// A device is back.
    Back {
        /// Device name.
        device: &'static str,
        /// What the computer is plugged into.
        hub: &'static str,
    },
    /// Battery low.
    BatteryLow {
        /// Device name.
        device: &'static str,
        /// Level in percent.
        percent: u8,
    },
    /// An audio service failed to start.
    ServiceFailed {
        /// Which one.
        service: Service,
        /// The reason.
        message: String,
    },
}

/// An audio service that can fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    AudioMix,
    VirtualMic,
}

impl Service {
    /// The service's name in the UI language.
    fn name(self) -> String {
        match self {
            Self::AudioMix => tr!("alert-service-audiomix"),
            Self::VirtualMic => tr!("alert-service-virtual-mic"),
        }
    }
}

impl Alert {
    /// Notification text in the UI language.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Linked { device } => tr!("alert-linked", device = device.to_string()),
            Self::Unlinked { device } => tr!("alert-unlinked", device = device.to_string()),
            Self::Gone { device, hub } => tr!(
                "alert-gone",
                device = device.to_string(),
                hub = hub.to_string()
            ),
            Self::Back { device, hub } => tr!(
                "alert-back",
                device = device.to_string(),
                hub = hub.to_string()
            ),
            Self::BatteryLow { device, percent } => {
                tr!(
                    "alert-battery-low",
                    device = device.to_string(),
                    percent = u32::from(*percent)
                )
            }
            Self::ServiceFailed { service, message } => tr!(
                "alert-service-failed",
                service = service.name(),
                message = message.as_str()
            ),
        }
    }
}

/// The parts of one device's state alerts depend on.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DeviceObserved {
    name: &'static str,
    hub: &'static str,
    /// `None` for wired devices or before the first read.
    linked: Option<bool>,
    battery: Option<u8>,
    charging: bool,
}

/// The parts of the state alerts depend on.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Observed {
    devices: BTreeMap<String, DeviceObserved>,
    mix: ServiceStatus,
    mic: ServiceStatus,
}

impl Observed {
    fn of(snapshot: &Snapshot) -> Self {
        let devices = snapshot
            .devices
            .devices
            .iter()
            .filter(|d| d.ready)
            .map(|d| {
                let charge = d.batteries().map(|(_, charge)| charge);
                (
                    d.key().to_owned(),
                    DeviceObserved {
                        name: d.info.name,
                        hub: d.hub(),
                        linked: d.linked(),
                        battery: d
                            .main_battery()
                            .map(ss_core::battery::BatteryLevel::percent),
                        charging: matches!(
                            charge,
                            Some(ChargeState::Charging | ChargeState::PluggedNotCharging)
                        ),
                    },
                )
            })
            .collect();
        Self {
            devices,
            mix: snapshot.audio.mix.clone(),
            mic: snapshot.audio.mic.clone(),
        }
    }
}

/// Remembers the previous state and reports changes.
#[derive(Debug, Default)]
pub struct AlertTracker {
    last: Option<Observed>,
    /// Devices whose low-battery warning was already shown.
    battery_warned: BTreeSet<String>,
}

impl AlertTracker {
    /// Alerts for the change from the previous snapshot to `snapshot`. The first snapshot only
    /// sets the baseline (no alerts at start-up).
    pub fn observe(&mut self, snapshot: &Snapshot) -> Vec<Alert> {
        let now = Observed::of(snapshot);
        let Some(last) = self.last.replace(now.clone()) else {
            for (key, device) in &now.devices {
                if device.battery.is_some_and(|b| b <= LOW_BATTERY_PERCENT) {
                    self.battery_warned.insert(key.clone());
                }
            }
            return Vec::new();
        };
        let mut alerts = Vec::new();
        for (key, device) in &last.devices {
            if !now.devices.contains_key(key) {
                alerts.push(Alert::Gone {
                    device: device.name,
                    hub: device.hub,
                });
            }
        }
        for (key, device) in &now.devices {
            let Some(before) = last.devices.get(key) else {
                alerts.push(Alert::Back {
                    device: device.name,
                    hub: device.hub,
                });
                continue;
            };
            // Link changes only count while the device stays connected (its own alert covers
            // the rest).
            match (before.linked, device.linked) {
                (Some(false), Some(true)) => alerts.push(Alert::Linked {
                    device: device.name,
                }),
                (Some(true), Some(false)) => alerts.push(Alert::Unlinked {
                    device: device.name,
                }),
                _ => {}
            }
            self.battery(key, device, &mut alerts);
        }
        for (service, before, after) in [
            (Service::AudioMix, &last.mix, &now.mix),
            (Service::VirtualMic, &last.mic, &now.mic),
        ] {
            if let ServiceStatus::Failed(message) = after
                && before != after
            {
                alerts.push(Alert::ServiceFailed {
                    service,
                    message: message.clone(),
                });
            }
        }
        alerts
    }

    fn battery(&mut self, key: &str, device: &DeviceObserved, alerts: &mut Vec<Alert>) {
        let Some(percent) = device.battery else {
            return;
        };
        if device.charging || percent >= REARM_BATTERY_PERCENT {
            self.battery_warned.remove(key);
        } else if percent <= LOW_BATTERY_PERCENT && self.battery_warned.insert(key.to_owned()) {
            alerts.push(Alert::BatteryLow {
                device: device.name,
                percent,
            });
        }
    }
}

/// Shows one OS notification.
///
/// # Errors
/// A message if the OS refused it.
pub fn show(text: &str) -> Result<(), String> {
    let mut notification = notify_rust::Notification::new();
    crate::platform::prepare_notification(&mut notification);
    notification
        .summary(APP_NAME)
        .body(text)
        .show()
        .and(Ok(())) // the handle type differs per OS
        .map_err(|e| e.to_string())
}

/// Starts the notification thread; it ends when the core is dropped.
///
/// # Errors
/// If the thread cannot be spawned.
pub fn spawn(core: Weak<AppCore>) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("alerts".to_owned())
        .spawn(move || {
            let mut tracker = AlertTracker::default();
            while let Some(core) = core.upgrade() {
                let snapshot = core.snapshot();
                let enabled = snapshot.config.app.notifications.enabled;
                drop(core);
                for alert in tracker.observe(&snapshot) {
                    if enabled {
                        let _ = show(&alert.text());
                    }
                }
                std::thread::sleep(POLL_INTERVAL);
            }
        })
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::devices::DeviceSnapshot;
    use ss_core::battery::BatteryLevel;
    use ss_device::{
        Bus, DeviceClass, DeviceInfo, FeatureId, FeatureKind, Instance, Support, Value,
    };

    static INFO: DeviceInfo = DeviceInfo {
        id: "test",
        name: "Test headset",
        class: DeviceClass::Headset,
        support: Support::Verified,
        hub_name: Some("base station"),
    };
    const NAME: &str = "Test headset";
    const HUB: &str = "base station";

    /// `present = false` → no device; otherwise a device with link state and battery.
    fn snapshot(present: bool, linked: Option<bool>, battery: u8, charging: bool) -> Snapshot {
        let mut snapshot = Snapshot::default();
        if !present {
            return snapshot;
        }
        let mut values = BTreeMap::new();
        if let Some(linked) = linked {
            values.insert(
                FeatureId::of(FeatureKind::WirelessLink),
                Value::Bool(linked),
            );
        }
        values.insert(
            FeatureId::of(FeatureKind::Battery),
            Value::Battery {
                levels: vec![BatteryLevel::from_percent(battery).ok()],
                charge: if charging {
                    ChargeState::Charging
                } else {
                    ChargeState::Discharging
                },
            },
        );
        snapshot.devices.devices.push(DeviceSnapshot {
            info: &INFO,
            instance: Instance {
                key: "test/1".to_owned(),
                bus: Bus::Usb,
                vendor_id: 1,
                product_id: 2,
                release: None,
                serial: None,
            },
            features: Vec::new(),
            values,
            ready: true,
        });
        snapshot
    }

    #[test]
    fn start_up_is_silent() {
        let mut tracker = AlertTracker::default();
        assert_eq!(
            tracker.observe(&snapshot(true, Some(true), 10, false)),
            Vec::<Alert>::new()
        );
        // Already low at start: no warning later either until it re-arms.
        assert_eq!(
            tracker.observe(&snapshot(true, Some(true), 10, false)),
            Vec::<Alert>::new()
        );
    }

    #[test]
    fn link_and_device_changes() {
        let mut tracker = AlertTracker::default();
        tracker.observe(&snapshot(true, Some(true), 100, false));
        assert_eq!(
            tracker.observe(&snapshot(true, Some(false), 100, false)),
            vec![Alert::Unlinked { device: NAME }]
        );
        assert_eq!(
            tracker.observe(&snapshot(true, Some(true), 100, false)),
            vec![Alert::Linked { device: NAME }]
        );
        assert_eq!(
            tracker.observe(&snapshot(false, None, 100, false)),
            vec![Alert::Gone {
                device: NAME,
                hub: HUB
            }]
        );
        assert_eq!(
            tracker.observe(&snapshot(true, Some(true), 100, false)),
            vec![Alert::Back {
                device: NAME,
                hub: HUB
            }]
        );
    }

    #[test]
    fn low_battery_warns_once_and_rearms() {
        let mut tracker = AlertTracker::default();
        tracker.observe(&snapshot(true, Some(true), 50, false));
        assert_eq!(
            tracker.observe(&snapshot(true, Some(true), 37, false)),
            Vec::<Alert>::new()
        );
        let low = tracker.observe(&snapshot(true, Some(true), 25, false));
        assert!(matches!(low.as_slice(), [Alert::BatteryLow { .. }]));
        assert_eq!(
            tracker.observe(&snapshot(true, Some(true), 12, false)),
            Vec::<Alert>::new()
        );
        tracker.observe(&snapshot(true, Some(true), 12, true)); // charging re-arms
        let again = tracker.observe(&snapshot(true, Some(true), 12, false));
        assert!(matches!(again.as_slice(), [Alert::BatteryLow { .. }]));
    }

    #[test]
    fn service_failures_are_reported_once() {
        let mut tracker = AlertTracker::default();
        let mut s = snapshot(true, Some(true), 100, false);
        tracker.observe(&s);
        s.audio.mix = ServiceStatus::Failed("no permission".to_owned());
        assert_eq!(tracker.observe(&s).len(), 1);
        assert_eq!(tracker.observe(&s), Vec::<Alert>::new());
    }

    #[test]
    fn texts_name_the_device() {
        assert_eq!(
            Alert::Gone {
                device: NAME,
                hub: HUB
            }
            .text(),
            "Test headset: base station disconnected — unplugged or switched to another computer"
        );
    }
}
