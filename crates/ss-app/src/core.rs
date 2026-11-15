//! `AppCore`: the handle the UI, tray and hotkeys use (`docs/architecture/app.md` §1).
//!
//! It owns the configuration, routes changes to the service threads, saves the config debounced
//! and hands out state snapshots. It is `Send + Sync`, so tray and hotkey handlers can use it
//! while the window is hidden.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};

use ss_device::{FeatureId, FeatureKind, Value};
use ss_store::{Config, Store};

use crate::audio::service::{self, AudioMsg};
use crate::audio::supervisor::{AudioBackend, AudioStatus};
use crate::crackle::{BurstDetector, CrackleStatus};
use crate::devices::{self, DeviceMsg, DevicesState};
use crate::notify::Notifier;
use crate::update::{UpdateState, Updater};

/// Quiet time before a changed config is written (slider drags write once).
const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);
/// How often device presence and the headset link are checked (gate, "headset off" fallback).
const LINK_POLL: Duration = Duration::from_millis(500);

/// Everything the UI needs to render one frame.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// Current configuration.
    pub config: Config,
    /// Audio services.
    pub audio: AudioStatus,
    /// Connected devices (ADR 0008).
    pub devices: DevicesState,
    /// Problem with the config file at start-up (shown once in the UI).
    pub config_warning: Option<String>,
    /// Problem while saving the config.
    pub save_error: Option<String>,
    /// Automatic crackling fix.
    pub crackle: CrackleStatus,
    /// ChatMix position, −1 (game) … +1 (chat); follows the dial and the slider.
    pub chatmix_position: f32,
    /// OpenSeries may act: a supported device is connected (or development mode without the
    /// device manager). When `false` everything is paused (`app.md` §8).
    pub active: bool,
    /// In-app updates (ADR 0006).
    pub update: UpdateState,
}

/// Shared application core.
pub struct AppCore {
    config: Mutex<Config>,
    store: Store,
    config_warning: Option<String>,
    save_error: Arc<Mutex<Option<String>>>,
    audio_status: Arc<Mutex<AudioStatus>>,
    devices_state: Arc<Mutex<DevicesState>>,
    audio_tx: Sender<AudioMsg>,
    device_tx: Sender<DeviceMsg>,
    save_tx: Sender<Config>,
    threads: Mutex<Vec<JoinHandle<()>>>,
    /// The first `service_threads` handles are the audio thread and the device manager.
    service_threads: usize,
    /// Software ChatMix position for hotkeys, −1 (game) … +1 (chat).
    chatmix_position: Arc<Mutex<f32>>,
    crackle_status: Arc<Mutex<CrackleStatus>>,
    crackle_watch: Mutex<Option<crate::platform::UnderflowWatch>>,
    notify: Notifier,
    /// `false` in development runs without the device manager: the device gate is skipped.
    gated: bool,
    /// The update checker, attached once after start ([`AppCore::attach_updater`]).
    updater: OnceLock<Updater>,
}

impl AppCore {
    /// Loads the config and starts the audio and saver threads, and the device manager unless
    /// `with_headset` is `false` (development runs that must not talk to any device).
    ///
    /// # Errors
    /// If a thread cannot be spawned.
    pub fn start<B, F>(
        store: Store,
        notify: &Notifier,
        with_headset: bool,
        make_backend: F,
    ) -> std::io::Result<Arc<Self>>
    where
        B: AudioBackend,
        F: FnOnce() -> B + Send + 'static,
    {
        let loaded = store.load();
        let config = loaded.config;
        let audio_status = Arc::new(Mutex::new(AudioStatus::default()));
        let devices_state = Arc::new(Mutex::new(DevicesState::default()));
        let save_error = Arc::new(Mutex::new(None));

        let (audio_tx, audio_rx) = mpsc::channel();
        let (device_tx, device_rx) = mpsc::channel();
        let (chatmix_tx, chatmix_rx) = mpsc::channel();
        let (save_tx, save_rx) = mpsc::channel::<Config>();

        let mut threads = vec![service::spawn(
            audio_rx,
            config.clone(),
            Arc::clone(&audio_status),
            Arc::clone(notify),
            make_backend,
        )?];
        if with_headset {
            threads.push(devices::spawn(
                device_rx,
                Arc::clone(&devices_state),
                chatmix_tx,
                Arc::clone(notify),
                config.audiomix.enabled,
                config.devices.experimental,
            )?);
        } else {
            drop((device_rx, chatmix_tx));
        }
        let service_threads = threads.len();

        if with_headset {
            threads.push(spawn_link_forward(
                Arc::clone(&devices_state),
                audio_tx.clone(),
            )?);
        } else {
            // Development runs without the device manager skip the device gate (§8).
            let _ = audio_tx.send(AudioMsg::DevicePresent(true));
        }

        // Dial events go from the headset thread to the audio thread; the software slider
        // follows the dial.
        let chatmix_position = Arc::new(Mutex::new(0.0_f32));
        let forward_position = Arc::clone(&chatmix_position);
        let forward_notify = Arc::clone(notify);
        let forward_tx = audio_tx.clone();
        threads.push(
            std::thread::Builder::new()
                .name("chatmix-forward".to_owned())
                .spawn(move || {
                    for balance in chatmix_rx {
                        *lock(&forward_position) = ss_dsp::slider_from_balance(balance);
                        forward_notify();
                        if forward_tx.send(AudioMsg::ChatMix(balance)).is_err() {
                            return;
                        }
                    }
                })?,
        );

        threads.push(spawn_saver(
            store.clone(),
            save_rx,
            Arc::clone(&save_error),
            Arc::clone(notify),
        )?);

        let core = Arc::new(Self {
            config: Mutex::new(config),
            store,
            config_warning: loaded.warning,
            save_error,
            audio_status,
            devices_state,
            audio_tx,
            device_tx,
            save_tx,
            threads: Mutex::new(threads),
            service_threads,
            chatmix_position,
            crackle_status: Arc::new(Mutex::new(CrackleStatus::default())),
            crackle_watch: Mutex::new(None),
            notify: Arc::clone(notify),
            gated: with_headset,
            updater: OnceLock::new(),
        });
        if core.config().app.auto_fix_crackling {
            core.apply_crackle_watch(true);
        }
        Ok(core)
    }

    /// Location of the configuration file.
    #[must_use]
    pub fn config_path(&self) -> &std::path::Path {
        self.store.path()
    }

    /// A copy of the current configuration.
    #[must_use]
    pub fn config(&self) -> Config {
        lock(&self.config).clone()
    }

    /// Changes the configuration, applies it to the services and schedules a save.
    pub fn update_config(&self, change: impl FnOnce(&mut Config)) {
        let (new, before) = {
            let mut config = lock(&self.config);
            let before = config.clone();
            change(&mut config);
            (config.clone(), before)
        };
        let audiomix_toggled = new.audiomix.enabled != before.audiomix.enabled;
        let autostart_toggled = new.app.autostart != before.app.autostart;
        if new.app.auto_fix_crackling != before.app.auto_fix_crackling {
            self.apply_crackle_watch(new.app.auto_fix_crackling);
        }
        if autostart_toggled {
            *lock(&self.save_error) = crate::platform::set_autostart(new.app.autostart)
                .err()
                .map(|e| format!("autostart: {e}"));
        }
        if audiomix_toggled {
            let _ = self
                .device_tx
                .send(DeviceMsg::HostMixer(new.audiomix.enabled));
        }
        if new.devices.experimental != before.devices.experimental {
            let _ = self
                .device_tx
                .send(DeviceMsg::Experimental(new.devices.experimental));
        }
        let _ = self.audio_tx.send(AudioMsg::Config(Box::new(new.clone())));
        let _ = self.save_tx.send(new);
    }

    /// Starts or stops the automatic crackling fix.
    fn apply_crackle_watch(&self, enabled: bool) {
        let mut watch = lock(&self.crackle_watch);
        *watch = None; // stop a running watch first
        let mut status = lock(&self.crackle_status);
        status.active = false;
        status.error = None;
        if !enabled {
            return;
        }
        let device_tx = self.device_tx.clone();
        let gate_state = Arc::clone(&self.devices_state);
        let gated = self.gated;
        let crackle_status = Arc::clone(&self.crackle_status);
        let detector = Mutex::new(BurstDetector::default());
        let notify = Arc::clone(&self.notify);
        let started = crate::platform::watch_usb_underflows(move || {
            // Never reset USB devices while no supported device is connected (§8).
            let present = !gated || lock(&gate_state).any();
            if present && lock(&detector).record(Instant::now()) {
                let _ = device_tx.send(DeviceMsg::ReplugUsb { key: None });
                let mut status = lock(&crackle_status);
                status.fixes = status.fixes.saturating_add(1);
                status.last_fix = Some(SystemTime::now());
                drop(status);
                notify();
            }
        });
        match started {
            Ok(handle) => {
                *watch = Some(handle);
                status.active = true;
            }
            Err(error) => status.error = Some(error),
        }
    }

    /// Moves the software ChatMix by `step` (hotkeys) and tells the base station.
    pub fn nudge_chatmix(&self, step: f32) {
        let position = *lock(&self.chatmix_position) + step;
        self.set_chatmix(position);
        self.sync_chatmix_to_headset();
    }

    /// Sets the software ChatMix (−1 = only game … 0 = centre … +1 = only chat). Turning the
    /// headset dial overrides it, and the slider follows the dial.
    pub fn set_chatmix(&self, position: f32) {
        let position = position.clamp(-1.0, 1.0);
        *lock(&self.chatmix_position) = position;
        let balance = ss_dsp::balance_from_slider(position);
        let _ = self.audio_tx.send(AudioMsg::ChatMix(balance));
    }

    /// Sends the current Game/Chat position to the device's balance dial, so its display and
    /// dial continue from there. Only while AudioMix owns the dial; call it when a change is finished
    /// (slider released), not for every drag step.
    pub fn sync_chatmix_to_headset(&self) {
        if !lock(&self.config).audiomix.enabled {
            return;
        }
        let balance = ss_dsp::balance_from_slider(*lock(&self.chatmix_position));
        self.set_feature(
            None,
            FeatureId::of(FeatureKind::BalanceDial),
            Value::Balance(balance),
        );
    }

    /// Resets a device's USB connection (software re-plug) and reconnects; `key: None` = the
    /// first device that supports it.
    pub fn reconnect_usb(&self, key: Option<&str>) {
        let _ = self.device_tx.send(DeviceMsg::ReplugUsb {
            key: key.map(str::to_owned),
        });
    }

    /// Writes a device setting; `key: None` = the first device that has the feature.
    pub fn set_feature(&self, key: Option<&str>, id: FeatureId, value: Value) {
        let _ = self.device_tx.send(DeviceMsg::Set {
            key: key.map(str::to_owned),
            id,
            value,
        });
    }

    /// Connects the update checker (it needs the finished core, so it starts afterwards).
    pub fn attach_updater(&self, updater: Updater) {
        let _ = self.updater.set(updater);
    }

    /// Looks for a new release now.
    pub fn check_for_updates(&self) {
        if let Some(updater) = self.updater.get() {
            updater.check_now();
        }
    }

    /// Downloads, verifies and installs the offered update, then quits (ADR 0006 §4).
    pub fn install_update(&self) {
        if let Some(updater) = self.updater.get() {
            updater.install();
        }
    }

    /// Whether OpenSeries may act right now (a supported device connected, `app.md` §8).
    #[must_use]
    pub fn active(&self) -> bool {
        !self.gated || lock(&self.devices_state).any()
    }

    /// Everything the UI renders.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        // Copy the device state in its own statement: a guard temporary inside the struct
        // literal would live until its end and `active` would deadlock on the same mutex.
        let devices = lock(&self.devices_state).clone();
        let active = !self.gated || devices.any();
        Snapshot {
            config: self.config(),
            audio: lock(&self.audio_status).clone(),
            devices,
            config_warning: self.config_warning.clone(),
            save_error: lock(&self.save_error).clone(),
            crackle: lock(&self.crackle_status).clone(),
            chatmix_position: *lock(&self.chatmix_position),
            active,
            update: self
                .updater
                .get()
                .map_or(UpdateState::Unsupported, Updater::state),
        }
    }

    /// Stops the services (restoring the base station) and writes pending config changes.
    pub fn shutdown(&self) {
        *lock(&self.crackle_watch) = None;
        let _ = self.device_tx.send(DeviceMsg::Shutdown);
        let _ = self.audio_tx.send(AudioMsg::Shutdown);
        let threads = std::mem::take(&mut *lock(&self.threads));
        for thread in threads.into_iter().take(self.service_threads) {
            // Only the service threads are joined; the forwarder and the saver end when their
            // senders are dropped together with `self`.
            let _ = thread.join();
        }
        // Write the final state synchronously: the debounced saver may not run again.
        if let Err(error) = self.store.save(&self.config()) {
            *lock(&self.save_error) = Some(error.to_string());
        }
    }
}

/// Device present/absent (gate, `app.md` §8) and the audio device's wireless link (device
/// fallback, SON-28) → audio thread. Polling keeps the device manager unaware of the audio side.
/// Ends once the audio thread is gone.
fn spawn_link_forward(
    state: Arc<Mutex<DevicesState>>,
    audio_tx: Sender<AudioMsg>,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("device-link-forward".to_owned())
        .spawn(move || {
            let mut last = None;
            let mut last_present = None;
            let mut last_endpoints: Option<Vec<String>> = None;
            loop {
                let (present, linked, endpoints) = {
                    let state = lock(&state);
                    let audio_device = state.with(FeatureKind::AudioEndpoint);
                    (
                        state.any(),
                        audio_device.and_then(devices::DeviceSnapshot::linked),
                        audio_device.map(endpoint_names).unwrap_or_default(),
                    )
                };
                if last_endpoints.as_ref() != Some(&endpoints) {
                    last_endpoints = Some(endpoints.clone());
                    if audio_tx
                        .send(AudioMsg::HeadsetEndpoints(endpoints))
                        .is_err()
                    {
                        return;
                    }
                }
                if last_present != Some(present) {
                    last_present = Some(present);
                    if audio_tx.send(AudioMsg::DevicePresent(present)).is_err() {
                        return;
                    }
                }
                if let Some(now) = linked {
                    if last != Some(now) {
                        last = Some(now);
                        if audio_tx.send(AudioMsg::HeadsetLinked(now)).is_err() {
                            return;
                        }
                    }
                }
                std::thread::sleep(LINK_POLL);
            }
        })
}

/// Name parts of a device's audio endpoints (outputs and inputs, without duplicates).
fn endpoint_names(device: &devices::DeviceSnapshot) -> Vec<String> {
    let Some(ss_device::Descriptor::AudioEndpoint { outputs, inputs }) = device
        .feature(FeatureKind::AudioEndpoint)
        .map(|f| &f.descriptor)
    else {
        return Vec::new();
    };
    let mut names: Vec<String> = outputs
        .iter()
        .chain(*inputs)
        .map(|n| (*n).to_owned())
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// Writes config changes once the user pauses (slider drags write once); ends with the core.
fn spawn_saver(
    store: Store,
    save_rx: mpsc::Receiver<Config>,
    error_slot: Arc<Mutex<Option<String>>>,
    notify: Notifier,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("config-saver".to_owned())
        .spawn(move || {
            while let Ok(mut latest) = save_rx.recv() {
                // Keep taking newer versions until the user pauses.
                loop {
                    match save_rx.recv_timeout(SAVE_DEBOUNCE) {
                        Ok(newer) => latest = newer,
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => {
                            let _ = store.save(&latest);
                            return;
                        }
                    }
                }
                let result = store.save(&latest).err().map(|e| e.to_string());
                *lock(&error_slot) = result;
                notify();
            }
        })
}

/// Locks a mutex, continuing with the data if another thread panicked while holding it.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
