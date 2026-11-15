//! Desktop notifications (CORE-15, NTU-02; `docs/architecture/app.md` §2).
//!
//! [`AlertTracker`] turns successive state snapshots into notable changes (pure, unit-tested);
//! [`spawn`] polls the app state on a background thread, so alerts also appear while the window
//! is hidden, and shows them as OS notifications when enabled in the settings.

use std::sync::Weak;
use std::time::Duration;

use ss_core::battery::ChargeState;

use crate::audio::supervisor::ServiceStatus;
use crate::core::{AppCore, Snapshot};

/// How often the state is checked.
const POLL_INTERVAL: Duration = Duration::from_secs(1);
/// Headset battery at or below this level (percent) while discharging triggers a warning.
pub const LOW_BATTERY_PERCENT: u8 = 25;
/// The low-battery warning re-arms once the level is back at or above this (or charging).
const REARM_BATTERY_PERCENT: u8 = 50;
/// Title of every notification.
const APP_NAME: &str = "StealSeries";

/// Something worth a notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alert {
    /// The headset linked to the base station.
    HeadsetConnected,
    /// The headset is off or out of range.
    HeadsetDisconnected,
    /// The base station vanished (unplugged, or its input switched to another computer).
    BaseStationGone,
    /// The base station is back.
    BaseStationBack,
    /// Headset battery low.
    BatteryLow(u8),
    /// An audio service failed to start.
    ServiceFailed {
        /// "AudioMix" or "Virtual microphone".
        service: &'static str,
        /// The reason.
        message: String,
    },
}

impl Alert {
    /// Notification text.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::HeadsetConnected => "Headset connected".to_owned(),
            Self::HeadsetDisconnected => "Headset disconnected (off or out of range)".to_owned(),
            Self::BaseStationGone => {
                "Base station disconnected — unplugged or switched to another computer".to_owned()
            }
            Self::BaseStationBack => "Base station connected".to_owned(),
            Self::BatteryLow(percent) => format!("Headset battery low (≈{percent} %)"),
            Self::ServiceFailed { service, message } => format!("{service} stopped: {message}"),
        }
    }
}

/// The parts of the state alerts depend on.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Observed {
    base_station: bool,
    /// `None` until the base station reported a status.
    headset_linked: Option<bool>,
    battery: Option<u8>,
    charging: bool,
    mix: ServiceStatus,
    mic: ServiceStatus,
}

impl Observed {
    fn of(snapshot: &Snapshot) -> Self {
        let status = snapshot.headset.status;
        Self {
            base_station: snapshot.headset.base_station,
            headset_linked: status.map(|s| s.connected),
            battery: status
                .filter(|s| s.connected)
                .and_then(|s| s.headset_battery)
                .map(ss_core::battery::BatteryLevel::percent),
            charging: status.is_some_and(|s| {
                matches!(
                    s.charge_state,
                    ChargeState::Charging | ChargeState::PluggedNotCharging
                )
            }),
            mix: snapshot.audio.mix.clone(),
            mic: snapshot.audio.mic.clone(),
        }
    }
}

/// Remembers the previous state and reports changes.
#[derive(Debug, Default)]
pub struct AlertTracker {
    last: Option<Observed>,
    battery_warned: bool,
}

impl AlertTracker {
    /// Alerts for the change from the previous snapshot to `snapshot`. The first snapshot only
    /// sets the baseline (no alerts at start-up).
    pub fn observe(&mut self, snapshot: &Snapshot) -> Vec<Alert> {
        let now = Observed::of(snapshot);
        let Some(last) = self.last.replace(now.clone()) else {
            self.battery_warned = now.battery.is_some_and(|b| b <= LOW_BATTERY_PERCENT);
            return Vec::new();
        };
        let mut alerts = Vec::new();
        match (last.base_station, now.base_station) {
            (true, false) => alerts.push(Alert::BaseStationGone),
            (false, true) => alerts.push(Alert::BaseStationBack),
            _ => {}
        }
        // Link changes only count while the base station stays connected (its own alert
        // covers the rest).
        if last.base_station && now.base_station {
            match (last.headset_linked, now.headset_linked) {
                (Some(false), Some(true)) => alerts.push(Alert::HeadsetConnected),
                (Some(true), Some(false)) => alerts.push(Alert::HeadsetDisconnected),
                _ => {}
            }
        }
        self.battery(&now, &mut alerts);
        for (service, before, after) in [
            ("AudioMix", &last.mix, &now.mix),
            ("Virtual microphone", &last.mic, &now.mic),
        ] {
            if let ServiceStatus::Failed(message) = after {
                if before != after {
                    alerts.push(Alert::ServiceFailed {
                        service,
                        message: message.clone(),
                    });
                }
            }
        }
        alerts
    }

    fn battery(&mut self, now: &Observed, alerts: &mut Vec<Alert>) {
        let Some(percent) = now.battery else {
            return;
        };
        if now.charging || percent >= REARM_BATTERY_PERCENT {
            self.battery_warned = false;
        } else if percent <= LOW_BATTERY_PERCENT && !self.battery_warned {
            self.battery_warned = true;
            alerts.push(Alert::BatteryLow(percent));
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
    use super::*;
    use ss_core::battery::BatteryLevel;
    use ss_nova_pro::Status;

    fn snapshot(base: bool, linked: Option<bool>, battery: u8, charging: bool) -> Snapshot {
        let mut snapshot = Snapshot::default();
        snapshot.headset.base_station = base;
        snapshot.headset.status = linked.map(|connected| Status {
            connected,
            headset_battery: BatteryLevel::from_percent(battery).ok(),
            charge_state: if charging {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            },
            spare_battery: None,
            noise_control: None,
            transparency: None,
            mic_muted: false,
            auto_off: None,
            mic_led: None,
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
    fn link_and_base_station_changes() {
        let mut tracker = AlertTracker::default();
        tracker.observe(&snapshot(true, Some(true), 100, false));
        assert_eq!(
            tracker.observe(&snapshot(true, Some(false), 100, false)),
            vec![Alert::HeadsetDisconnected]
        );
        assert_eq!(
            tracker.observe(&snapshot(true, Some(true), 100, false)),
            vec![Alert::HeadsetConnected]
        );
        assert_eq!(
            tracker.observe(&snapshot(false, None, 100, false)),
            vec![Alert::BaseStationGone]
        );
        assert_eq!(
            tracker.observe(&snapshot(true, Some(true), 100, false)),
            vec![Alert::BaseStationBack]
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
        assert!(matches!(low.as_slice(), [Alert::BatteryLow(_)]));
        assert_eq!(
            tracker.observe(&snapshot(true, Some(true), 12, false)),
            Vec::<Alert>::new()
        );
        tracker.observe(&snapshot(true, Some(true), 12, true)); // charging re-arms
        let again = tracker.observe(&snapshot(true, Some(true), 12, false));
        assert!(matches!(again.as_slice(), [Alert::BatteryLow(_)]));
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
}
