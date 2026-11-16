//! Loading and saving `config.json` (`docs/architecture/app.md` §3).
//!
//! * Saving is atomic: write `config.json.tmp`, then rename over the old file.
//! * A missing file yields defaults. An unreadable, invalid or newer-version file is moved aside
//!   to `config.invalid-<unix seconds>.json` and defaults are used — the app never fails to start
//!   because of its config, and the user's file is never silently overwritten.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;

use crate::config::{CONFIG_VERSION, Config};

/// Folder name below the OS config directory.
const APP_DIR: &str = "OpenSeries";
/// File name of the configuration.
const FILE_NAME: &str = "config.json";

/// Saving failed.
#[derive(Debug, Error)]
pub enum StoreError {
    /// File system error.
    #[error("{context} {path}: {source}")]
    Io {
        /// What was attempted.
        context: &'static str,
        /// The file involved.
        path: PathBuf,
        /// The OS error.
        source: io::Error,
    },
    /// Serialisation failed (cannot happen for our DTOs, kept for completeness).
    #[error("serialising the config: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// Result of [`Store::load`].
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    /// The configuration to use.
    pub config: Config,
    /// Set when the file could not be used and defaults were loaded instead.
    pub warning: Option<String>,
}

/// The configuration file of one user.
#[derive(Debug, Clone)]
pub struct Store {
    path: PathBuf,
}

impl Store {
    /// A store at an explicit path (tests, portable setups).
    #[must_use]
    pub const fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// The store in the per-user config directory
    /// (`~/Library/Application Support/OpenSeries/config.json` on macOS,
    /// `%APPDATA%\OpenSeries\config.json` on Windows).
    ///
    /// # Errors
    /// [`StoreError::Io`] if the directory cannot be determined.
    pub fn per_user() -> Result<Self, StoreError> {
        let base = config_base_dir().ok_or_else(|| StoreError::Io {
            context: "locating the config directory",
            path: PathBuf::new(),
            source: io::Error::new(io::ErrorKind::NotFound, "no home directory"),
        })?;
        Ok(Self::at(base.join(APP_DIR).join(FILE_NAME)))
    }

    /// Path of the config file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads the configuration (see module docs for the fallback rules).
    #[must_use]
    pub fn load(&self) -> Loaded {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Loaded {
                    config: Config::default(),
                    warning: None,
                };
            }
            Err(error) => return self.fallback(&format!("cannot read: {error}")),
        };
        // Windows editors and PowerShell often write a UTF-8 byte order mark; JSON parsers reject it.
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        match serde_json::from_str::<Config>(text) {
            Ok(config) if config.version > CONFIG_VERSION => self.fallback(&format!(
                "written by a newer version (schema {} > {CONFIG_VERSION})",
                config.version
            )),
            Ok(mut config) => {
                config.version = CONFIG_VERSION;
                config.presets.adopt_legacy();
                Loaded {
                    config,
                    warning: None,
                }
            }
            Err(error) => self.fallback(&format!("invalid JSON: {error}")),
        }
    }

    /// Saves `config` atomically, creating the directory if needed.
    ///
    /// # Errors
    /// [`StoreError`] on file system errors.
    pub fn save(&self, config: &Config) -> Result<(), StoreError> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|source| StoreError::Io {
                context: "creating",
                path: dir.to_owned(),
                source,
            })?;
        }
        let text = serde_json::to_string_pretty(config)?;
        let temp = self.path.with_extension("json.tmp");
        fs::write(&temp, text).map_err(|source| StoreError::Io {
            context: "writing",
            path: temp.clone(),
            source,
        })?;
        fs::rename(&temp, &self.path).map_err(|source| StoreError::Io {
            context: "replacing",
            path: self.path.clone(),
            source,
        })
    }

    /// Moves the unusable file aside and returns defaults with a warning.
    fn fallback(&self, reason: &str) -> Loaded {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let aside = self
            .path
            .with_file_name(format!("config.invalid-{seconds}.json"));
        let moved = fs::rename(&self.path, &aside);
        let warning = match moved {
            Ok(()) => format!(
                "{} {reason}; moved to {} and started with defaults",
                self.path.display(),
                aside.display()
            ),
            Err(error) => format!(
                "{} {reason}; could not move it aside ({error}); using defaults",
                self.path.display()
            ),
        };
        Loaded {
            config: Config::default(),
            warning: Some(warning),
        }
    }
}

/// The OS-specific per-user configuration directory.
fn config_base_dir() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Channel;

    /// A fresh directory below the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "ss-store-{name}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn missing_file_gives_defaults_without_warning() {
        let dir = TempDir::new("missing");
        let loaded = Store::at(dir.0.join("config.json")).load();
        assert_eq!(loaded.config, Config::default());
        assert!(loaded.warning.is_none());
        assert!(
            !loaded.config.audiomix.enabled,
            "AudioMix is off by default"
        );
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = TempDir::new("roundtrip");
        let store = Store::at(dir.0.join("sub").join("config.json"));
        let mut config = Config::default();
        config.audiomix.enabled = true;
        config
            .audiomix
            .assignments
            .insert("com.spotify.client".to_owned(), Channel::Media);
        config.mic.denoise = Some(0.9);
        store.save(&config).unwrap();
        assert_eq!(store.load().config, config);
        assert!(!dir.0.join("sub").join("config.json.tmp").exists());
    }

    #[test]
    fn a_byte_order_mark_is_accepted() {
        let dir = TempDir::new("bom");
        let path = dir.0.join("config.json");
        fs::create_dir_all(&dir.0).unwrap();
        fs::write(
            &path,
            "\u{feff}{\"version\":1,\"audiomix\":{\"enabled\":true}}",
        )
        .unwrap();
        let loaded = Store::at(path).load();
        assert!(loaded.warning.is_none(), "{:?}", loaded.warning);
        assert!(loaded.config.audiomix.enabled);
    }

    #[test]
    fn invalid_file_is_moved_aside() {
        let dir = TempDir::new("invalid");
        let path = dir.0.join("config.json");
        fs::write(&path, "{ not json").unwrap();
        let loaded = Store::at(path.clone()).load();
        assert_eq!(loaded.config, Config::default());
        assert!(loaded.warning.is_some());
        assert!(!path.exists(), "original moved away");
        let aside = fs::read_dir(&dir.0)
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("config.invalid-")
            });
        assert!(aside);
    }

    #[test]
    fn newer_schema_is_not_overwritten() {
        let dir = TempDir::new("newer");
        let path = dir.0.join("config.json");
        fs::write(&path, r#"{ "version": 99 }"#).unwrap();
        let loaded = Store::at(path).load();
        assert!(loaded.warning.unwrap().contains("newer version"));
    }

    #[test]
    fn partial_file_fills_in_defaults() {
        let dir = TempDir::new("partial");
        let path = dir.0.join("config.json");
        fs::write(
            &path,
            r#"{ "version": 1, "audiomix": { "enabled": true, "assignments": { "com.hnc.Discord": "chat" } },
                 "mic": { "gate": { "mode": "automatic", "offset_db": 12 } } }"#,
        )
        .unwrap();
        let loaded = Store::at(path).load();
        assert!(loaded.warning.is_none());
        let config = loaded.config;
        assert!(config.audiomix.enabled);
        assert_eq!(
            config.audiomix.assignments.get("com.hnc.Discord"),
            Some(&Channel::Chat)
        );
        assert!((config.audiomix.master.volume - 1.0).abs() < f32::EPSILON);
        assert!(config.app.hotkeys.mic_mute.is_some());
    }
}
