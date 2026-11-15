//! Windows implementations of the platform surface. AudioMix is the M4a session mixer (volumes and
//! ChatMix only, ADR 0004); the virtual mic, channel DSP, the USB reset and the underflow watch are
//! not available yet. Headset control, UI, tray and hotkeys work cross-platform.

use std::num::NonZeroIsize;
use std::os::windows::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// The M4a session mixer (`ss-audio-windows`).
pub type AudioBackend = crate::audio::windows::WinAudioBackend;

/// File next to `config.json` that keeps original app volumes until they are restored.
const RESTORE_FILE: &str = "session-volumes.txt";

/// Creates the audio backend (called on the audio-control thread).
#[must_use]
pub fn audio_backend() -> AudioBackend {
    let path = ss_store::Store::per_user().map_or_else(
        |_| std::env::temp_dir().join("StealSeries").join(RESTORE_FILE),
        |store| store.path().with_file_name(RESTORE_FILE),
    );
    crate::audio::windows::WinAudioBackend::new(path)
}

/// No virtual microphone on Windows (ADR 0004 update: no paid signing, no VB-Cable).
pub const VIRTUAL_MIC_SUPPORTED: bool = false;
/// No per-channel DSP on Windows: it would need a signed virtual audio driver (ADR 0004).
pub const CHANNEL_DSP_SUPPORTED: bool = false;

/// Whether [`reset_base_usb`] can work on this OS.
pub const USB_RESET_SUPPORTED: bool = false;
/// Whether [`watch_usb_underflows`] can work on this OS.
pub const UNDERFLOW_WATCH_SUPPORTED: bool = false;

/// `CREATE_NO_WINDOW` process creation flag (Win32 `CreateProcess`).
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Command-line switches of the Inno Setup installer for an in-app update: no wizard, no
/// message boxes, no reboot; the installer starts the new version when done (`installer/windows`).
const SILENT_INSTALL_ARGS: [&str; 4] = ["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"];

/// Registry key for per-user start-up programs.
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
/// Value name under [`RUN_KEY`].
const RUN_VALUE: &str = "StealSeries";

/// Starts the app at login via the per-user `Run` registry value (verified on Windows 11, 2026-11-14).
///
/// # Errors
/// A user-facing message if `reg.exe` fails.
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let mut command = Command::new("reg");
    // A GUI app starting a console program would flash a console window.
    command.creation_flags(CREATE_NO_WINDOW);
    if enabled {
        let exe = std::env::current_exe().map_err(|e| format!("locating the app: {e}"))?;
        command.args([
            "add",
            RUN_KEY,
            "/v",
            RUN_VALUE,
            "/t",
            "REG_SZ",
            "/d",
            &format!("\"{}\"", exe.display()),
            "/f",
        ]);
    } else {
        command.args(["delete", RUN_KEY, "/v", RUN_VALUE, "/f"]);
    }
    let output = command
        .output()
        .map_err(|e| format!("running reg.exe: {e}"))?;
    if output.status.success() || !enabled {
        // Deleting a value that does not exist fails harmlessly.
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

/// Not available on Windows yet.
///
/// # Errors
/// Always.
pub fn reset_base_usb() -> Result<(), String> {
    Err("Reconnect USB is not available on Windows yet".to_owned())
}

/// Not available on Windows yet.
pub struct UnderflowWatch;

/// Not available on Windows yet.
///
/// # Errors
/// Always.
pub fn watch_usb_underflows(
    _on_underflow: impl Fn() + Send + 'static,
) -> Result<UnderflowWatch, String> {
    Err("the crackling watch is not available on Windows yet".to_owned())
}

/// The main window's native handle, used to show it again from the tray.
///
/// Needed because eframe stops running hidden windows on Windows (see `ss-window-windows`).
#[derive(Debug, Clone, Copy)]
pub struct NativeWindow(NonZeroIsize);

impl NativeWindow {
    /// The Win32 handle of `source`, if it is a Win32 window.
    pub fn of(source: &impl HasWindowHandle) -> Option<Self> {
        match source.window_handle().ok()?.as_raw() {
            RawWindowHandle::Win32(handle) => Some(Self(handle.hwnd)),
            _ => None,
        }
    }

    /// Shows, restores and focuses the window.
    pub fn show(self) {
        ss_window_windows::show_window(self.0);
    }
}

/// The virtual-mic driver does not exist on this OS yet (`None` = not applicable).
#[must_use]
pub const fn mic_driver_installed() -> Option<bool> {
    None
}

/// Not available on this OS.
///
/// # Errors
/// Always.
pub fn install_mic_driver() -> Result<(), String> {
    Err("no virtual-mic driver for this operating system yet".to_owned())
}

/// Ends a process that did not quit when asked (single instance, `app.md` §9).
pub fn terminate_process(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

/// Updates install through the per-user installer from wherever the app runs; returns the
/// running executable.
///
/// # Errors
/// If the executable cannot be located.
pub fn update_install_target() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| format!("locating the app: {e}"))
}

/// Starts the verified installer silently (ADR 0006 §4); the caller quits right after so the
/// installer can replace the files. The installer starts the new version.
///
/// # Errors
/// If the installer cannot be started.
pub fn install_update(package: &Path) -> Result<(), String> {
    Command::new(package)
        .args(SILENT_INSTALL_ARGS)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("starting the installer: {e}"))
}
