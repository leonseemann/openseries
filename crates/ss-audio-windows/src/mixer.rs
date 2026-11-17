//! The running session mixer (`docs/architecture/audio-engine.md` §5).

use std::collections::{BTreeMap, HashMap};

use ss_core::mixer::ChannelId;
use ss_engine::Command;

use crate::AudioError;
use crate::gains::ChannelGains;
use crate::ledger::{RestoreFile, Tracked};
use crate::session::{SessionInfo, list_sessions};

/// Process id of the system-sounds session, which is never touched.
const SYSTEM_SOUNDS_PID: u32 = 0;

struct Managed {
    session: SessionInfo,
    tracked: Tracked,
}

/// Drives app session volumes from channel gains while it lives; dropping it restores them.
///
/// Must stay on the thread that called [`crate::init_com`].
pub struct SessionMixer {
    routes: HashMap<u32, ChannelId>,
    gains: ChannelGains,
    /// Keyed by session instance identifier.
    managed: HashMap<String, Managed>,
    file: RestoreFile,
    /// Originals by session identifier; mirrors `file`.
    saved: BTreeMap<String, f32>,
}

impl SessionMixer {
    /// Starts managing all app sessions. `routes` maps process ids to channels; every other app is
    /// on Media.
    ///
    /// # Errors
    /// If the sessions cannot be listed.
    pub fn start(routes: HashMap<u32, ChannelId>, file: RestoreFile) -> Result<Self, AudioError> {
        let saved = file.load();
        let mut mixer = Self {
            routes,
            gains: ChannelGains::default(),
            managed: HashMap::new(),
            file,
            saved,
        };
        mixer.adopt_sessions(list_sessions()?);
        mixer.apply_all();
        Ok(mixer)
    }

    /// Applies a live change (fader, master, ChatMix). DSP strips are ignored.
    pub fn command(&mut self, command: &Command) {
        if self.gains.apply(command) {
            self.apply_all();
        }
    }

    /// Picks up new sessions, forgets ended ones and re-applies (user changes are adopted).
    pub fn refresh(&mut self) {
        if let Ok(sessions) = list_sessions() {
            self.managed
                .retain(|instance, _| sessions.iter().any(|s| s.instance == *instance));
            self.adopt_sessions(sessions);
        }
        self.apply_all();
    }

    /// Tracks sessions we do not manage yet. Their originals are saved to disk before any write.
    fn adopt_sessions(&mut self, sessions: Vec<SessionInfo>) {
        let mut changed = false;
        for session in sessions {
            if session.pid == SYSTEM_SOUNDS_PID || self.managed.contains_key(&session.instance) {
                continue;
            }
            let original = match self.saved.get(&session.identifier) {
                // Left over from a crash or an app closed while we ran: the stored level is the
                // user's, the current one is ours.
                Some(&saved) => saved,
                None => match session.volume() {
                    Ok(volume) => volume,
                    Err(_) => continue,
                },
            };
            if self.saved.insert(session.identifier.clone(), original) != Some(original) {
                changed = true;
            }
            self.managed.insert(
                session.instance.clone(),
                Managed {
                    session,
                    tracked: Tracked {
                        original,
                        written: f32::NAN, // force the first write
                    },
                },
            );
        }
        if changed {
            self.persist();
        }
    }

    fn apply_all(&mut self) {
        let mut changed = false;
        for managed in self.managed.values_mut() {
            let channel = self
                .routes
                .get(&managed.session.pid)
                .copied()
                .unwrap_or(ChannelId::Media);
            let Ok(current) = managed.session.volume() else {
                continue;
            };
            let gain = self.gains.gain(channel);
            let before = managed.tracked.original;
            if let Some(level) = managed.tracked.update(current, gain) {
                let _ = managed.session.set_volume(level);
            }
            #[allow(clippy::float_cmp)] // exact: did `update` replace the value at all?
            if managed.tracked.original != before {
                self.saved
                    .insert(managed.session.identifier.clone(), managed.tracked.original);
                changed = true;
            }
        }
        if changed {
            self.persist();
        }
    }

    fn persist(&self) {
        // Best effort: without the file only crash recovery is lost, not the mix.
        let _ = self.file.save(&self.saved);
    }
}

impl Drop for SessionMixer {
    fn drop(&mut self) {
        for managed in self.managed.values() {
            if managed.session.set_volume(managed.tracked.original).is_ok() {
                self.saved.remove(&managed.session.identifier);
            }
        }
        self.persist();
    }
}

/// Restores sessions listed in `file` (crash leftovers, apps closed while the mix ran). Call
/// periodically while no [`SessionMixer`] runs.
pub fn restore_pending(file: &RestoreFile) {
    let mut saved = file.load();
    if saved.is_empty() {
        return;
    }
    let Ok(sessions) = list_sessions() else {
        return;
    };
    let before = saved.len();
    for session in sessions {
        if let Some(&level) = saved.get(&session.identifier)
            && session.set_volume(level).is_ok()
        {
            saved.remove(&session.identifier);
        }
    }
    if saved.len() != before {
        let _ = file.save(&saved);
    }
}
