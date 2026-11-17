//! In-app updates (ADR 0006 §4): looks for a newer GitHub release shortly after start and every
//! few hours, and on request; "Install" downloads the package, verifies its signature
//! (`ss-update`) and hands it to [`platform::install_update`], then quits so it can be replaced.
//!
//! Nothing is installed without the user's click.

use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::{Duration, Instant, SystemTime};

use ss_update::semver::Version;
use ss_update::{Asset, Platform, Release, Update};

use crate::core::AppCore;
use crate::i18n::tr;
use crate::notify::Notifier;
use crate::platform;

/// First automatic check after start (lets the app settle; no network at login yet).
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(20);
/// Interval of automatic checks.
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
/// How often the thread re-reads the "check automatically" setting.
const CONFIG_POLL: Duration = Duration::from_secs(60);
/// Timeout of the release lookup.
const API_TIMEOUT: Duration = Duration::from_secs(30);
/// Timeout of a package download.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(15 * 60);
/// Largest accepted API answer.
const API_LIMIT: u64 = 1 << 20;
/// Largest accepted package (current packages are ~10–30 MB).
const PACKAGE_LIMIT: u64 = 256 << 20;
/// Largest accepted signature file.
const SIGNATURE_LIMIT: u64 = 4096;
/// Progress is published after this many downloaded bytes.
const PROGRESS_STEP: u64 = 256 << 10;
/// Download buffer size.
const CHUNK: usize = 64 << 10;
/// Folder under the temp directory that holds downloads.
const DOWNLOAD_DIR: &str = "OpenSeries-update";

/// What the updater is doing; rendered in Settings → Updates and the tray.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateState {
    /// Not checked yet.
    #[default]
    Idle,
    /// Asking GitHub for the latest release.
    Checking,
    /// The running version is the newest.
    UpToDate {
        /// When the check finished.
        checked: SystemTime,
    },
    /// A newer version can be installed.
    Available(Update),
    /// Downloading the package.
    Downloading {
        /// The update being downloaded.
        update: Update,
        /// Bytes received so far.
        received: u64,
    },
    /// The installer runs; the app quits now.
    Installing(Version),
    /// The last check or install failed (user-facing message); `update` is still offered.
    Failed {
        /// What went wrong.
        message: String,
        /// The update that was found before, if any.
        update: Option<Update>,
    },
    /// No packages are published for this OS / CPU.
    Unsupported,
}

impl UpdateState {
    /// The update on offer, if any.
    #[must_use]
    pub fn update(&self) -> Option<&Update> {
        match self {
            Self::Available(update) | Self::Downloading { update, .. } => Some(update),
            Self::Failed { update, .. } => update.as_ref(),
            _ => None,
        }
    }

    /// A check or download is running.
    #[must_use]
    pub const fn busy(&self) -> bool {
        matches!(
            self,
            Self::Checking | Self::Downloading { .. } | Self::Installing(_)
        )
    }
}

enum Request {
    Check,
    Install,
}

/// Handle to the updater thread.
#[derive(Clone)]
pub struct Updater {
    state: Arc<Mutex<UpdateState>>,
    requests: Sender<Request>,
}

impl Updater {
    /// The current state.
    #[must_use]
    pub fn state(&self) -> UpdateState {
        lock(&self.state).clone()
    }

    /// Looks for a new release now.
    pub fn check_now(&self) {
        let _ = self.requests.send(Request::Check);
    }

    /// Downloads, verifies and installs the update on offer, then quits the app.
    pub fn install(&self) {
        let _ = self.requests.send(Request::Install);
    }
}

/// Starts the updater thread; it ends when the core is dropped.
///
/// # Errors
/// If the thread cannot be spawned.
pub fn spawn(core: Weak<AppCore>, notify: Notifier) -> std::io::Result<Updater> {
    let state = Arc::new(Mutex::new(UpdateState::Idle));
    let (requests, receiver) = mpsc::channel();
    let worker = Worker {
        core,
        notify,
        state: Arc::clone(&state),
        current: current_version(),
        notified: None,
    };
    std::thread::Builder::new()
        .name("updater".to_owned())
        .spawn(move || worker.run(&receiver))?;
    Ok(Updater { state, requests })
}

/// The running version (from `Cargo.toml`).
fn current_version() -> Version {
    // The workspace version is always valid semver (cargo enforces it).
    Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_else(|_| Version::new(0, 0, 0))
}

struct Worker {
    core: Weak<AppCore>,
    notify: Notifier,
    state: Arc<Mutex<UpdateState>>,
    current: Version,
    /// The version a notification was shown for (once per version).
    notified: Option<Version>,
}

impl Worker {
    fn run(mut self, requests: &Receiver<Request>) {
        let Some(platform) = Platform::current() else {
            self.set(UpdateState::Unsupported);
            return;
        };
        let mut next_check = Instant::now() + FIRST_CHECK_DELAY;
        loop {
            let Some(automatic) = self.setting(|c| c.app.updates.check_automatically) else {
                return;
            };
            let wait = if automatic {
                next_check
                    .saturating_duration_since(Instant::now())
                    .min(CONFIG_POLL)
            } else {
                CONFIG_POLL
            };
            match requests.recv_timeout(wait) {
                Ok(Request::Check) => {
                    self.check(platform);
                    next_check = Instant::now() + CHECK_INTERVAL;
                }
                Ok(Request::Install) => self.install(platform),
                Err(RecvTimeoutError::Timeout) if automatic && Instant::now() >= next_check => {
                    self.check(platform);
                    next_check = Instant::now() + CHECK_INTERVAL;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    /// Reads one config value; `None` once the app shuts down.
    fn setting<T>(&self, read: impl FnOnce(&ss_store::Config) -> T) -> Option<T> {
        self.core.upgrade().map(|core| read(&core.config()))
    }

    fn set(&self, state: UpdateState) {
        *lock(&self.state) = state;
        (self.notify)();
    }

    fn check(&mut self, platform: Platform) -> Option<Update> {
        if lock(&self.state).busy() {
            return None;
        }
        let previous = lock(&self.state).update().cloned();
        self.set(UpdateState::Checking);
        match find_update(platform, &self.current) {
            Ok(Some(update)) => {
                self.announce(&update);
                self.set(UpdateState::Available(update.clone()));
                Some(update)
            }
            Ok(None) => {
                self.set(UpdateState::UpToDate {
                    checked: SystemTime::now(),
                });
                None
            }
            Err(message) => {
                self.set(UpdateState::Failed {
                    message,
                    update: previous.clone(),
                });
                previous
            }
        }
    }

    /// One notification per new version.
    fn announce(&mut self, update: &Update) {
        if self.notified.as_ref() == Some(&update.version) {
            return;
        }
        self.notified = Some(update.version.clone());
        if self.setting(|c| c.app.notifications.enabled) == Some(true) {
            let _ = crate::alerts::show(&tr!(
                "update-notification",
                version = update.version.to_string()
            ));
        }
    }

    fn install(&mut self, platform: Platform) {
        if lock(&self.state).busy() {
            return;
        }
        let offered = lock(&self.state).update().cloned();
        let Some(update) = offered.or_else(|| self.check(platform)) else {
            return;
        };
        let result = platform::update_install_target()
            .and_then(|_| self.download(&update))
            .and_then(|package| {
                platform::install_update(&package)?;
                Ok(())
            });
        match result {
            Ok(()) => {
                self.set(UpdateState::Installing(update.version.clone()));
                if let Some(core) = self.core.upgrade() {
                    // Restores app volumes and the default input, saves the config, exits.
                    crate::quit(&core);
                }
            }
            Err(message) => self.set(UpdateState::Failed {
                message,
                update: Some(update),
            }),
        }
    }

    /// Downloads the package and its signature, verifies them and returns the package path.
    fn download(&self, update: &Update) -> Result<PathBuf, String> {
        self.set(UpdateState::Downloading {
            update: update.clone(),
            received: 0,
        });
        let dir = std::env::temp_dir().join(DOWNLOAD_DIR);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|e| {
            tr!(
                "update-error-folder",
                path = dir.display().to_string(),
                error = e.to_string()
            )
        })?;

        let signature = fetch(&update.signature.browser_download_url, SIGNATURE_LIMIT)?;
        let signature = String::from_utf8(signature).map_err(|_| tr!("update-error-signature"))?;
        let bytes = self.fetch_package(update)?;
        ss_update::verify(&update.package.name, &bytes, &signature).map_err(|e| e.to_string())?;

        let path = dir.join(&update.package.name);
        std::fs::write(&path, &bytes)
            .map_err(|e| tr!("update-error-save", error = e.to_string()))?;
        Ok(path)
    }

    fn fetch_package(&self, update: &Update) -> Result<Vec<u8>, String> {
        let Asset {
            browser_download_url: url,
            size,
            ..
        } = &update.package;
        let mut response = agent(DOWNLOAD_TIMEOUT)
            .get(url)
            .call()
            .map_err(|e| network_problem(&e))?;
        let mut reader = response
            .body_mut()
            .with_config()
            .limit(PACKAGE_LIMIT)
            .reader();
        let mut bytes = Vec::with_capacity(usize::try_from(*size).unwrap_or_default());
        let mut chunk = vec![0_u8; CHUNK];
        let mut published = 0;
        loop {
            let read = reader
                .read(&mut chunk)
                .map_err(|e| tr!("update-error-interrupted", error = e.to_string()))?;
            if read == 0 {
                break;
            }
            bytes
                .write_all(&chunk[..read])
                .map_err(|e| tr!("update-error-download", error = e.to_string()))?;
            let received = bytes.len() as u64;
            if received - published >= PROGRESS_STEP {
                published = received;
                self.set(UpdateState::Downloading {
                    update: update.clone(),
                    received,
                });
            }
        }
        Ok(bytes)
    }
}

/// Asks GitHub for the latest release and returns it if it is newer and has a package for us.
fn find_update(platform: Platform, current: &Version) -> Result<Option<Update>, String> {
    let body = fetch(&ss_update::latest_release_url(), API_LIMIT)?;
    let release = Release::parse(&body).map_err(|e| e.to_string())?;
    release
        .update_for(platform, current)
        .map_err(|e| e.to_string())
}

/// GETs `url` (following redirects) and returns at most `limit` bytes.
fn fetch(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let mut response = agent(API_TIMEOUT)
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| network_problem(&e))?;
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|e| tr!("update-error-interrupted", error = e.to_string()))
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .user_agent(concat!("OpenSeries/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// A plain-language network error.
fn network_problem(error: &ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(404) => tr!("update-error-no-release"),
        ureq::Error::StatusCode(403 | 429) => tr!("update-error-rate-limited"),
        ureq::Error::StatusCode(code) => tr!("update-error-http", code = code.to_string()),
        other => tr!("update-error-unreachable", error = other.to_string()),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Talks to the real GitHub API: `cargo test -p ss-app -- --ignored release_lookup`.
    #[test]
    #[ignore = "needs the network"]
    fn release_lookup_reaches_github() {
        let result = find_update(Platform::WindowsX64, &Version::new(0, 0, 0));
        println!("{result:?}");
        // Either a release was found, or a plain-language reason (no release yet).
        if let Err(message) = result {
            assert!(!message.contains("could not reach"), "{message}");
        }
    }

    #[test]
    fn offered_update_survives_a_failed_check() {
        let state = UpdateState::Failed {
            message: "x".to_owned(),
            update: None,
        };
        assert!(state.update().is_none());
        assert!(!state.busy());
        assert!(UpdateState::Checking.busy());
    }
}
