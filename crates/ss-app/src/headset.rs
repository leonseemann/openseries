//! Headset thread: the single owner of the Nova Pro HID device (`docs/architecture/app.md` §1).
//!
//! Connects and reconnects on its own, keeps a state snapshot for the UI, executes commands and
//! forwards ChatMix dial movements to the audio side. While AudioMix is enabled it hands the
//! base station's ChatMix to the host (and gives it back when disabled or on quit).

use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ss_core::headset::ChatMixBalance;
use ss_hid::{DeviceSelector, HidapiTransport};
use ss_nova_pro::{Command, Display, Event, NovaProWireless, Settings, Status, ids};

use crate::notify::Notifier;

/// How long one event read blocks before messages are checked again.
const POLL: Duration = Duration::from_millis(100);
/// Delay between connection attempts while the base station is missing.
const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);
/// How often the full status (battery etc.) is re-read.
const STATUS_INTERVAL: Duration = Duration::from_secs(30);

const NOVA_PRO: DeviceSelector<'static> = DeviceSelector {
    vendor_id: ids::VENDOR_ID,
    product_ids: &ids::PRODUCT_IDS,
    interface: ids::CONTROL_INTERFACE,
    command_usage_page: ids::COMMAND_USAGE_PAGE,
    event_usage_page: ids::EVENT_USAGE_PAGE,
};

/// Messages to the headset thread.
#[derive(Debug, Clone, Copy)]
pub enum HeadsetMsg {
    /// Send a setting to the headset.
    Send(Command),
    /// AudioMix master switch changed (controls the ChatMix hand-over).
    AudioMixEnabled(bool),
    /// Reset the base station's USB connection (software re-plug) and reconnect.
    ResetUsb,
    /// Restore the base station and stop.
    Shutdown,
}

/// What the UI shows about the headset.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HeadsetState {
    /// The base station is open.
    pub base_station: bool,
    /// Last status block (link, battery, ANC, mic mute …).
    pub status: Option<Status>,
    /// Last settings block (EQ preset, sidetone, mic volume …).
    pub settings: Option<Settings>,
    /// Last display block.
    pub display: Option<Display>,
    /// Last ChatMix dial position.
    pub chatmix: Option<ChatMixBalance>,
    /// Last error (cleared on success).
    pub error: Option<String>,
}

/// Starts the headset thread.
pub fn spawn(
    rx: Receiver<HeadsetMsg>,
    state: Arc<Mutex<HeadsetState>>,
    chatmix_out: Sender<ChatMixBalance>,
    notify: Notifier,
    audiomix_enabled: bool,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("headset".to_owned())
        .spawn(move || {
            let mut worker = Worker {
                rx,
                state,
                chatmix_out,
                notify,
                device: None,
                takeover_wanted: audiomix_enabled,
                takeover_active: false,
                last_attempt: None,
                last_status: Instant::now(),
            };
            worker.run();
        })
}

struct Worker {
    rx: Receiver<HeadsetMsg>,
    state: Arc<Mutex<HeadsetState>>,
    chatmix_out: Sender<ChatMixBalance>,
    notify: Notifier,
    device: Option<NovaProWireless<HidapiTransport>>,
    takeover_wanted: bool,
    takeover_active: bool,
    last_attempt: Option<Instant>,
    last_status: Instant,
}

impl Worker {
    fn run(&mut self) {
        loop {
            if !self.handle_messages() {
                self.release_takeover();
                return;
            }
            if self.device.is_none() {
                self.try_connect();
                if self.device.is_none() {
                    std::thread::sleep(POLL);
                    continue;
                }
            }
            self.sync_takeover();
            self.read_event();
            if self.last_status.elapsed() >= STATUS_INTERVAL {
                self.refresh_status();
            }
        }
    }

    /// Returns `false` on shutdown.
    fn handle_messages(&mut self) -> bool {
        loop {
            match self.rx.try_recv() {
                Ok(HeadsetMsg::Send(command)) => {
                    // The base station reports button presses, but not changes made by the
                    // host, so read the result back right away.
                    self.send(command);
                    self.refresh_all();
                }
                Ok(HeadsetMsg::ResetUsb) => self.reset_usb(),
                Ok(HeadsetMsg::AudioMixEnabled(enabled)) => self.takeover_wanted = enabled,
                Ok(HeadsetMsg::Shutdown) | Err(TryRecvError::Disconnected) => return false,
                Err(TryRecvError::Empty) => return true,
            }
        }
    }

    fn try_connect(&mut self) {
        if self
            .last_attempt
            .is_some_and(|t| t.elapsed() < RECONNECT_INTERVAL)
        {
            return;
        }
        self.last_attempt = Some(Instant::now());
        let opened = ss_hid::api().and_then(|api| HidapiTransport::open(&api, &NOVA_PRO));
        let Ok(transport) = opened else {
            self.update(|s| {
                s.base_station = false;
                s.status = None;
            });
            return;
        };
        let mut device = NovaProWireless::new(transport);
        let status = device.status().ok();
        let settings = device.settings().ok();
        let display = device.display().ok();
        self.device = Some(device);
        self.takeover_active = false;
        self.last_status = Instant::now();
        if let Some(balance) = settings.and_then(|s| s.chatmix) {
            let _ = self.chatmix_out.send(balance);
        }
        self.update(|s| {
            s.base_station = true;
            s.status = status;
            s.settings = settings;
            s.display = display;
            s.chatmix = settings.and_then(|x| x.chatmix);
            s.error = None;
        });
    }

    fn sync_takeover(&mut self) {
        if self.takeover_wanted == self.takeover_active {
            return;
        }
        let wanted = self.takeover_wanted;
        self.send(Command::SetChatMixDial(wanted));
        self.send(Command::SetMixerPresent(wanted));
        self.takeover_active = wanted;
    }

    fn release_takeover(&mut self) {
        if self.takeover_active {
            self.send(Command::SetChatMixDial(false));
            self.send(Command::SetMixerPresent(false));
            self.takeover_active = false;
        }
    }

    fn send(&mut self, command: Command) {
        let Some(device) = self.device.as_mut() else {
            self.update(|s| s.error = Some("headset base station not connected".to_owned()));
            return;
        };
        if let Err(error) = device.send(command) {
            self.disconnect(&error.to_string());
        }
    }

    fn read_event(&mut self) {
        let Some(device) = self.device.as_mut() else {
            return;
        };
        match device.next_event(POLL) {
            Ok(Some(event)) => self.apply(event),
            Ok(None) => {}
            Err(error) => self.disconnect(&error.to_string()),
        }
    }

    fn refresh_status(&mut self) {
        self.last_status = Instant::now();
        let Some(device) = self.device.as_mut() else {
            return;
        };
        match device.status() {
            Ok(status) => self.update(|s| s.status = Some(status)),
            Err(error) => self.disconnect(&error.to_string()),
        }
    }

    /// Software re-plug of the base station; the reconnect loop picks it up again afterwards.
    fn reset_usb(&mut self) {
        self.release_takeover();
        self.device = None; // close our HID handles before the device disappears
        let result = ss_usb_macos::reenumerate(ids::VENDOR_ID, &ids::PRODUCT_IDS);
        // Give the base station time to re-enumerate before the next connection attempt.
        self.last_attempt = Some(Instant::now());
        self.update(|s| {
            s.base_station = false;
            s.status = None;
            s.error = result.err().map(|e| e.to_string());
        });
    }

    /// Re-reads status, settings and display after a host-side change.
    fn refresh_all(&mut self) {
        self.refresh_status();
        let Some(device) = self.device.as_mut() else {
            return;
        };
        let settings = device.settings();
        let display = device.display();
        match (settings, display) {
            (Ok(settings), Ok(display)) => self.update(|s| {
                s.settings = Some(settings);
                s.display = Some(display);
            }),
            (Err(error), _) | (_, Err(error)) => self.disconnect(&error.to_string()),
        }
    }

    fn disconnect(&mut self, reason: &str) {
        self.device = None;
        self.takeover_active = false;
        self.update(|s| {
            s.base_station = false;
            s.status = None;
            s.error = Some(format!("headset connection lost: {reason}"));
        });
    }

    fn apply(&mut self, event: Event) {
        if let Event::ChatMix(balance) = event {
            let _ = self.chatmix_out.send(balance);
        }
        if matches!(event, Event::Connection { .. } | Event::Battery(_)) {
            // Link or battery changed: re-read the complete status block.
            self.refresh_status();
        }
        self.update(|s| apply_event(s, event));
    }

    fn update(&self, change: impl FnOnce(&mut HeadsetState)) {
        if let Ok(mut state) = self.state.lock() {
            change(&mut state);
        }
        (self.notify)();
    }
}

/// Folds one unsolicited event into the snapshot.
fn apply_event(state: &mut HeadsetState, event: Event) {
    match event {
        Event::Status(status) => state.status = Some(status),
        Event::Settings(settings) => state.settings = Some(settings),
        Event::Display(display) => state.display = Some(display),
        Event::ChatMix(balance) => state.chatmix = Some(balance),
        Event::NoiseControl(mode) => {
            if let Some(status) = state.status.as_mut() {
                status.noise_control = Some(mode);
            }
        }
        Event::MicMute(muted) => {
            if let Some(status) = state.status.as_mut() {
                status.mic_muted = muted;
            }
        }
        Event::Sidetone(level) => {
            if let Some(settings) = state.settings.as_mut() {
                settings.sidetone = Some(level);
            }
        }
        Event::MicVolume(volume) => {
            if let Some(settings) = state.settings.as_mut() {
                settings.mic_volume = Some(volume);
            }
        }
        Event::EqPreset(preset) => {
            if let Some(settings) = state.settings.as_mut() {
                settings.eq_preset = Some(preset);
            }
        }
        Event::AutoOff(auto_off) => {
            if let Some(status) = state.status.as_mut() {
                status.auto_off = Some(auto_off);
            }
        }
        Event::DisplayBrightness(brightness) => {
            if let Some(display) = state.display.as_mut() {
                display.brightness = Some(brightness);
            }
        }
        Event::Battery(_)
        | Event::Volume(_)
        | Event::EqBand { .. }
        | Event::Connection { .. }
        | Event::Reply { .. }
        | Event::Unrecognized { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss_core::headset::NoiseControl;

    #[test]
    fn events_update_the_snapshot_fields() {
        let mut state = HeadsetState::default();
        let balance = ss_dsp::balance_from_slider(0.5);
        apply_event(&mut state, Event::ChatMix(balance));
        assert_eq!(state.chatmix, Some(balance));

        // Without a status block, partial events are ignored instead of inventing one.
        apply_event(&mut state, Event::NoiseControl(NoiseControl::Off));
        assert!(state.status.is_none());
    }
}
