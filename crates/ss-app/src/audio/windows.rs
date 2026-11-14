//! [`AudioBackend`] on top of `ss-audio-windows`: the M4a session mixer
//! (`docs/architecture/audio-engine.md` §5, ADR 0004).

use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use ss_audio_windows::ledger::RestoreFile;
use ss_audio_windows::{SessionMixer, endpoints, init_com, list_sessions, restore_pending};
use ss_engine::Command as EngineCommand;

use super::plan::{DeviceInfo, DeviceSet, MicPlan, MixPlan, ProcessInfo};
use super::supervisor::{AudioBackend, MicHandle, MixHandle};

/// Shown when the virtual microphone is enabled on Windows.
const NO_MIC: &str =
    "The virtual microphone needs the StealSeries audio driver on Windows (milestone M4b)";

/// The Windows backend. Lives on the audio-control thread (COM objects are not `Send`).
pub struct WinAudioBackend {
    restore: RestoreFile,
    /// Set while a [`SessionMixer`] runs; otherwise leftovers are restored on every scan.
    mixing: Rc<Cell<bool>>,
    com_error: Option<String>,
}

impl WinAudioBackend {
    /// Creates the backend; `restore_path` holds original volumes for crash recovery.
    #[must_use]
    pub fn new(restore_path: PathBuf) -> Self {
        let com_error = init_com().err().map(|e| e.to_string());
        let backend = Self {
            restore: RestoreFile::new(restore_path),
            mixing: Rc::new(Cell::new(false)),
            com_error,
        };
        restore_pending(&backend.restore); // volumes left over from a crash
        backend
    }
}

struct MixSession {
    mixer: SessionMixer,
    mixing: Rc<Cell<bool>>,
}

impl MixHandle for MixSession {
    fn send(&mut self, command: EngineCommand) -> bool {
        self.mixer.command(&command);
        true
    }

    fn refresh(&mut self) {
        self.mixer.refresh();
    }
}

impl Drop for MixSession {
    fn drop(&mut self) {
        self.mixing.set(false);
    }
}

impl AudioBackend for WinAudioBackend {
    fn processes(&mut self) -> Result<Vec<ProcessInfo>, String> {
        if let Some(error) = &self.com_error {
            return Err(error.clone());
        }
        if !self.mixing.get() {
            restore_pending(&self.restore);
        }
        let sessions = list_sessions().map_err(|e| e.to_string())?;
        Ok(sessions
            .into_iter()
            .filter(|s| s.pid != 0 && !s.exe.is_empty())
            .filter_map(|s| {
                Some(ProcessInfo {
                    pid: i32::try_from(s.pid).ok()?,
                    bundle_id: Some(s.exe),
                    playing: s.active,
                })
            })
            .collect())
    }

    fn devices(&mut self) -> DeviceSet {
        let convert = |output: bool| endpoints(output).unwrap_or_default();
        let (outputs, inputs) = (convert(true), convert(false));
        let default = |list: &[ss_audio_windows::Endpoint]| {
            list.iter().find(|e| e.is_default).map(|e| e.id.clone())
        };
        let info = |list: &[ss_audio_windows::Endpoint]| {
            list.iter()
                .map(|e| DeviceInfo {
                    uid: e.id.clone(),
                    name: e.name.clone(),
                    id: 0, // endpoint ids survive replugs; no separate object id on Windows
                })
                .collect()
        };
        DeviceSet {
            default_output: default(&outputs),
            default_input: default(&inputs),
            outputs: info(&outputs),
            inputs: info(&inputs),
        }
    }

    fn start_mix(&mut self, plan: &MixPlan) -> Result<Box<dyn MixHandle>, String> {
        let routes: HashMap<u32, _> = plan
            .routes
            .iter()
            .filter_map(|(&pid, &channel)| Some((u32::try_from(pid).ok()?, channel)))
            .collect();
        let mixer = SessionMixer::start(routes, self.restore.clone()).map_err(|e| e.to_string())?;
        self.mixing.set(true);
        Ok(Box::new(MixSession {
            mixer,
            mixing: Rc::clone(&self.mixing),
        }))
    }

    fn start_mic(&mut self, _plan: &MicPlan) -> Result<Box<dyn MicHandle>, String> {
        Err(NO_MIC.to_owned())
    }
}
