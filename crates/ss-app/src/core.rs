//! `AppCore`: the handle the UI, tray and hotkeys use (`docs/architecture/app.md` §1).
//!
//! It owns the configuration, routes changes to the service threads, saves the config debounced
//! and hands out state snapshots. It is `Send + Sync`, so tray and hotkey handlers can use it
//! while the window is hidden.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};

use ss_nova_pro::Command;
use ss_store::{Config, Store};

use crate::audio::service::{self, AudioMsg};
use crate::audio::supervisor::{AudioBackend, AudioStatus};
use crate::crackle::{BurstDetector, CrackleStatus};
use crate::headset::{self, HeadsetMsg, HeadsetState};
use crate::notify::Notifier;

/// Quiet time before a changed config is written (slider drags write once).
const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);

/// Everything the UI needs to render one frame.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// Current configuration.
    pub config: Config,
    /// Audio services.
    pub audio: AudioStatus,
    /// Headset.
    pub headset: HeadsetState,
    /// Problem with the config file at start-up (shown once in the UI).
    pub config_warning: Option<String>,
    /// Problem while saving the config.
    pub save_error: Option<String>,
    /// Automatic crackling fix.
    pub crackle: CrackleStatus,
    /// ChatMix position, −1 (game) … +1 (chat); follows the dial and the slider.
    pub chatmix_position: f32,
}

/// Shared application core.
pub struct AppCore {
    config: Mutex<Config>,
    store: Store,
    config_warning: Option<String>,
    save_error: Arc<Mutex<Option<String>>>,
    audio_status: Arc<Mutex<AudioStatus>>,
    headset_state: Arc<Mutex<HeadsetState>>,
    audio_tx: Sender<AudioMsg>,
    headset_tx: Sender<HeadsetMsg>,
    save_tx: Sender<Config>,
    threads: Mutex<Vec<JoinHandle<()>>>,
    /// The first `service_threads` handles are the audio (and headset) threads.
    service_threads: usize,
    /// Software ChatMix position for hotkeys, −1 (game) … +1 (chat).
    chatmix_position: Arc<Mutex<f32>>,
    crackle_status: Arc<Mutex<CrackleStatus>>,
    crackle_watch: Mutex<Option<crate::platform::UnderflowWatch>>,
    notify: Notifier,
}

impl AppCore {
    /// Loads the config and starts the audio and saver threads, and the headset thread unless
    /// `with_headset` is `false` (development runs that must not talk to the device).
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
        let headset_state = Arc::new(Mutex::new(HeadsetState::default()));
        let save_error = Arc::new(Mutex::new(None));

        let (audio_tx, audio_rx) = mpsc::channel();
        let (headset_tx, headset_rx) = mpsc::channel();
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
            threads.push(headset::spawn(
                headset_rx,
                Arc::clone(&headset_state),
                chatmix_tx,
                Arc::clone(notify),
                config.audiomix.enabled,
            )?);
        } else {
            drop((headset_rx, chatmix_tx));
        }
        let service_threads = threads.len();

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

        let saver_store = store.clone();
        let error_slot = Arc::clone(&save_error);
        let saver_notify = Arc::clone(notify);
        threads.push(
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
                                    let _ = saver_store.save(&latest);
                                    return;
                                }
                            }
                        }
                        let result = saver_store.save(&latest).err().map(|e| e.to_string());
                        *lock(&error_slot) = result;
                        saver_notify();
                    }
                })?,
        );

        let core = Arc::new(Self {
            config: Mutex::new(config),
            store,
            config_warning: loaded.warning,
            save_error,
            audio_status,
            headset_state,
            audio_tx,
            headset_tx,
            save_tx,
            threads: Mutex::new(threads),
            service_threads,
            chatmix_position,
            crackle_status: Arc::new(Mutex::new(CrackleStatus::default())),
            crackle_watch: Mutex::new(None),
            notify: Arc::clone(notify),
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
                .headset_tx
                .send(HeadsetMsg::AudioMixEnabled(new.audiomix.enabled));
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
        let headset_tx = self.headset_tx.clone();
        let crackle_status = Arc::clone(&self.crackle_status);
        let detector = Mutex::new(BurstDetector::default());
        let notify = Arc::clone(&self.notify);
        let started = crate::platform::watch_usb_underflows(move || {
            if lock(&detector).record(Instant::now()) {
                let _ = headset_tx.send(HeadsetMsg::ResetUsb);
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

    /// Sends the current ChatMix position to the base station (`06 45`), so its display and dial
    /// continue from there. Only while AudioMix owns the dial; call it when a change is finished
    /// (slider released), not for every drag step.
    pub fn sync_chatmix_to_headset(&self) {
        if !lock(&self.config).audiomix.enabled {
            return;
        }
        let balance = ss_dsp::balance_from_slider(*lock(&self.chatmix_position));
        let _ = self
            .headset_tx
            .send(HeadsetMsg::Send(ss_nova_pro::Command::SetChatMix(balance)));
    }

    /// Resets the base station's USB connection (software re-plug) and reconnects.
    pub fn reset_headset_usb(&self) {
        let _ = self.headset_tx.send(HeadsetMsg::ResetUsb);
    }

    /// Sends a setting to the headset.
    pub fn headset(&self, command: Command) {
        let _ = self.headset_tx.send(HeadsetMsg::Send(command));
    }

    /// Everything the UI renders.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            config: self.config(),
            audio: lock(&self.audio_status).clone(),
            headset: lock(&self.headset_state).clone(),
            config_warning: self.config_warning.clone(),
            save_error: lock(&self.save_error).clone(),
            crackle: lock(&self.crackle_status).clone(),
            chatmix_position: *lock(&self.chatmix_position),
        }
    }

    /// Stops the services (restoring the base station) and writes pending config changes.
    pub fn shutdown(&self) {
        *lock(&self.crackle_watch) = None;
        let _ = self.headset_tx.send(HeadsetMsg::Shutdown);
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

/// Locks a mutex, continuing with the data if another thread panicked while holding it.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
