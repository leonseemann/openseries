//! Audio endpoints and per-app audio sessions via WASAPI / COM.

use std::collections::HashMap;

use wasapi::{DeviceEnumerator, Direction};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Media::Audio::{
    AudioSessionStateActive, IAudioSessionControl2, IAudioSessionManager2, ISimpleAudioVolume,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoTaskMemFree};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::core::{Interface, PWSTR};

use crate::AudioError;

/// Initialises COM (multithreaded apartment) on the calling thread. Call once per thread before
/// any other function of this crate; calling it again is harmless.
///
/// # Errors
/// If the thread already uses an incompatible apartment.
pub fn init_com() -> Result<(), AudioError> {
    wasapi::initialize_mta()
        .ok()
        .map_err(|e| AudioError::os("initialising COM", e))
}

/// An active audio endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// Endpoint id (`{0.0.0.00000000}.{guid}`), stable across replugs.
    pub id: String,
    /// Friendly name, e.g. `Kopfhörer (2- Arctis Nova Pro Wireless)`.
    pub name: String,
    /// The default endpoint for the console role.
    pub is_default: bool,
}

/// Active render (`output = true`) or capture endpoints.
///
/// # Errors
/// If the endpoints cannot be enumerated.
pub fn endpoints(output: bool) -> Result<Vec<Endpoint>, AudioError> {
    let direction = if output {
        Direction::Render
    } else {
        Direction::Capture
    };
    let os = |e| AudioError::os("listing audio endpoints", e);
    let enumerator = DeviceEnumerator::new().map_err(os)?;
    let default_id = enumerator
        .get_default_device(direction)
        .and_then(|d| d.get_id())
        .ok();
    let collection = enumerator.get_device_collection(direction).map_err(os)?;
    let mut list = Vec::new();
    for index in 0..collection.get_nbr_devices().map_err(os)? {
        let device = collection.get_device_at_index(index).map_err(os)?;
        let id = device.get_id().map_err(os)?;
        list.push(Endpoint {
            is_default: default_id.as_deref() == Some(id.as_str()),
            name: device.get_friendlyname().unwrap_or_else(|_| id.clone()),
            id,
        });
    }
    Ok(list)
}

/// One app's audio session on one render endpoint.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Process id (0 = system sounds).
    pub pid: u32,
    /// Lower-case executable name, e.g. `chrome.exe` (empty if unknown).
    pub exe: String,
    /// Session identifier: stable per app and endpoint, also across restarts.
    pub identifier: String,
    /// Session instance identifier: unique per running session.
    pub instance: String,
    /// Currently playing.
    pub active: bool,
    volume: ISimpleAudioVolume,
}

impl SessionInfo {
    /// The session's volume (`0.0..=1.0`, the app's slider in the Windows mixer).
    ///
    /// # Errors
    /// If the session is gone.
    pub fn volume(&self) -> Result<f32, AudioError> {
        // SAFETY: COM getter on a valid interface.
        unsafe { self.volume.GetMasterVolume() }
            .map_err(|e| AudioError::os("reading a session volume", e))
    }

    /// Sets the session's volume (`0.0..=1.0`). Windows remembers it for the app.
    ///
    /// # Errors
    /// If the session is gone.
    pub fn set_volume(&self, level: f32) -> Result<(), AudioError> {
        // SAFETY: COM setter on a valid interface; a null event context is allowed.
        unsafe {
            self.volume
                .SetMasterVolume(level.clamp(0.0, 1.0), std::ptr::null())
        }
        .map_err(|e| AudioError::os("setting a session volume", e))
    }
}

/// All audio sessions on all active render endpoints.
///
/// # Errors
/// If the endpoints cannot be enumerated. Sessions that vanish while listing are skipped.
pub fn list_sessions() -> Result<Vec<SessionInfo>, AudioError> {
    let os = |e| AudioError::os("listing audio sessions", e);
    let names = exe_names();
    let enumerator = DeviceEnumerator::new().map_err(os)?;
    let collection = enumerator
        .get_device_collection(Direction::Render)
        .map_err(os)?;
    let mut sessions = Vec::new();
    for index in 0..collection.get_nbr_devices().map_err(os)? {
        let Ok(device) = collection.get_device_at_index(index) else {
            continue;
        };
        // SAFETY: COM calls on valid interfaces; COM is initialised (`init_com`).
        let manager: Result<IAudioSessionManager2, _> =
            unsafe { device.as_immdevice().Activate(CLSCTX_ALL, None) };
        let Ok(manager) = manager else {
            continue;
        };
        // SAFETY: as above.
        let Ok(list) = (unsafe { manager.GetSessionEnumerator() }) else {
            continue;
        };
        // SAFETY: as above.
        let count = unsafe { list.GetCount() }.unwrap_or(0);
        for i in 0..count {
            // SAFETY: index within GetCount.
            if let Ok(control) = unsafe { list.GetSession(i) } {
                if let Some(session) = session_info(&control, &names) {
                    sessions.push(session);
                }
            }
        }
    }
    Ok(sessions)
}

fn session_info(
    control: &windows::Win32::Media::Audio::IAudioSessionControl,
    names: &HashMap<u32, String>,
) -> Option<SessionInfo> {
    let control2: IAudioSessionControl2 = control.cast().ok()?;
    // SAFETY: COM getters on valid interfaces; returned strings are freed in `take_string`.
    unsafe {
        let pid = control2.GetProcessId().ok()?;
        Some(SessionInfo {
            pid,
            exe: names.get(&pid).cloned().unwrap_or_default(),
            identifier: take_string(control2.GetSessionIdentifier().ok()?),
            instance: take_string(control2.GetSessionInstanceIdentifier().ok()?),
            active: control.GetState().ok()? == AudioSessionStateActive,
            volume: control.cast().ok()?,
        })
    }
}

/// Copies and frees a COM-allocated string.
///
/// # Safety
/// `text` must be a valid, CoTaskMem-allocated, NUL-terminated string (or null).
unsafe fn take_string(text: PWSTR) -> String {
    if text.is_null() {
        return String::new();
    }
    // SAFETY: guaranteed by the caller.
    unsafe {
        let value = text.to_string().unwrap_or_default();
        CoTaskMemFree(Some(text.0.cast_const().cast()));
        value
    }
}

/// pid → lower-case executable name, from a Toolhelp snapshot (empty on failure).
fn exe_names() -> HashMap<u32, String> {
    let mut names = HashMap::new();
    // SAFETY: Toolhelp snapshot API; `dwSize` is set as required and the handle is closed.
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return names;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: u32::try_from(size_of::<PROCESSENTRY32W>()).unwrap_or(0),
            ..Default::default()
        };
        let mut more = Process32FirstW(snapshot, &raw mut entry).is_ok();
        while more {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            names.insert(
                entry.th32ProcessID,
                String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase(),
            );
            more = Process32NextW(snapshot, &raw mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
    }
    names
}
