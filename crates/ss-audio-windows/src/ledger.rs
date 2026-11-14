//! Bookkeeping of the volumes we change (`docs/architecture/audio-engine.md` §5, steps 2–5).
//!
//! Platform-free so the rules are unit-tested: what to write, when the user moved a slider
//! themselves, and the on-disk list of originals that survives a crash.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Two volumes closer than this are considered equal (Windows stores `f32`, the mixer UI rounds).
const VOLUME_EPSILON: f32 = 0.005;
/// Below this channel gain an external change cannot be scaled back reliably and is ignored.
const MIN_ADOPT_GAIN: f32 = 0.01;

/// One session we manage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tracked {
    /// The volume the user had set (restored on stop).
    pub original: f32,
    /// The volume we wrote last.
    pub written: f32,
}

impl Tracked {
    /// Starts tracking a session whose volume is `current`.
    #[must_use]
    pub const fn new(original: f32) -> Self {
        Self {
            original,
            written: original,
        }
    }

    /// The volume to write for channel gain `gain`, or `None` if it is already set.
    ///
    /// `current` is the session's volume now. If it differs from what we wrote, the user moved
    /// the slider in the Windows mixer; their level becomes the new original.
    pub fn update(&mut self, current: f32, gain: f32) -> Option<f32> {
        if (current - self.written).abs() > VOLUME_EPSILON && gain >= MIN_ADOPT_GAIN {
            self.original = (current / gain).clamp(0.0, 1.0);
        }
        let target = (self.original * gain).clamp(0.0, 1.0);
        if (target - current).abs() <= VOLUME_EPSILON / 2.0 {
            self.written = current;
            return None;
        }
        self.written = target;
        Some(target)
    }
}

/// Original volumes on disk, keyed by session identifier (stable per app).
///
/// Format: one `volume<TAB>identifier` line per session; unreadable lines are skipped.
#[derive(Debug, Clone)]
pub struct RestoreFile {
    path: PathBuf,
}

impl RestoreFile {
    /// A restore file at `path` (not created until something is saved).
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Path of the file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The saved originals (empty if the file does not exist).
    #[must_use]
    pub fn load(&self) -> BTreeMap<String, f32> {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return BTreeMap::new();
        };
        text.lines()
            .filter_map(|line| {
                let (volume, identifier) = line.split_once('\t')?;
                let volume: f32 = volume.parse().ok()?;
                (volume.is_finite() && !identifier.is_empty())
                    .then(|| (identifier.to_owned(), volume.clamp(0.0, 1.0)))
            })
            .collect()
    }

    /// Replaces the saved originals; an empty map deletes the file.
    ///
    /// # Errors
    /// If the file cannot be written.
    pub fn save(&self, originals: &BTreeMap<String, f32>) -> io::Result<()> {
        if originals.is_empty() {
            return match fs::remove_file(&self.path) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
                _ => Ok(()),
            };
        }
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut text = String::new();
        for (identifier, volume) in originals {
            if !identifier.contains(['\n', '\r']) {
                let _ = writeln!(text, "{volume}\t{identifier}"); // writing to a String never fails
            }
        }
        let temp = self.path.with_extension("tmp");
        fs::write(&temp, text)?;
        fs::rename(&temp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn writes_original_times_gain_once() {
        let mut tracked = Tracked::new(0.8);
        assert_eq!(tracked.update(0.8, 0.5).map(|v| approx(v, 0.4)), Some(true));
        assert_eq!(tracked.update(0.4, 0.5), None);
    }

    #[test]
    fn user_change_becomes_the_new_original() {
        let mut tracked = Tracked::new(0.8);
        tracked.update(0.8, 0.5); // wrote 0.4
        // The user drags the app's slider to 0.3 in the Windows mixer.
        assert_eq!(tracked.update(0.3, 0.5), None);
        assert!(approx(tracked.original, 0.6));
        // Back to unity gain: their level × 1.
        assert_eq!(tracked.update(0.3, 1.0).map(|v| approx(v, 0.6)), Some(true));
    }

    #[test]
    fn muted_channel_does_not_adopt_changes() {
        let mut tracked = Tracked::new(0.8);
        tracked.update(0.8, 0.0); // wrote 0
        assert_eq!(tracked.update(0.5, 0.0).map(|v| approx(v, 0.0)), Some(true));
        assert!(approx(tracked.original, 0.8));
    }

    #[test]
    fn restore_file_round_trip_and_delete() {
        let dir = std::env::temp_dir().join(format!("ss-ledger-test-{}", std::process::id()));
        let file = RestoreFile::new(dir.join("session-volumes.txt"));
        assert!(file.load().is_empty());
        let originals = BTreeMap::from([
            ("{0.0.0.00000000}.{abc}|C:\\app.exe%b{x}".to_owned(), 0.75),
            ("other".to_owned(), 1.0),
        ]);
        file.save(&originals).unwrap();
        assert_eq!(file.load(), originals);
        file.save(&BTreeMap::new()).unwrap();
        assert!(!file.path().exists());
        let _ = fs::remove_dir_all(dir);
    }
}
