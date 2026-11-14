//! `AppCore`: the handle the UI, tray and hotkeys use (`docs/architecture/app.md` §1).
//!
//! It owns the configuration, routes changes to the service threads, saves the config debounced
//! and hands out state snapshots. It is `Send + Sync`, so tray and hotkey handlers can use it
//! while the window is hidden.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use ss_nova_pro::Command;
use ss_store::{Config, Store};

use crate::audio::service::{self, AudioMsg};
use crate::audio::supervisor::{AudioBackend, AudioStatus};
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

        // Dial events go from the headset thread to the audio thread.
        let forward_tx = audio_tx.clone();
        threads.push(
            std::thread::Builder::new()
                .name("chatmix-forward".to_owned())
                .spawn(move || {
                    for balance in chatmix_rx {
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

        Ok(Arc::new(Self {
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
        }))
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
        let (new, audiomix_toggled) = {
            let mut config = lock(&self.config);
            let before = config.audiomix.enabled;
            change(&mut config);
            (config.clone(), config.audiomix.enabled != before)
        };
        if audiomix_toggled {
            let _ = self
                .headset_tx
                .send(HeadsetMsg::AudioMixEnabled(new.audiomix.enabled));
        }
        let _ = self.audio_tx.send(AudioMsg::Config(Box::new(new.clone())));
        let _ = self.save_tx.send(new);
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
        }
    }

    /// Stops the services (restoring the base station) and writes pending config changes.
    pub fn shutdown(&self) {
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
