//! Windows implementations of the platform surface. AudioMix, the virtual mic, the USB reset and
//! the underflow watch arrive with milestone M4; headset control, UI, tray and hotkeys already
//! work cross-platform.

use std::process::Command;

/// Placeholder backend until `ss-audio-windows` exists.
pub type AudioBackend = crate::audio::unsupported::UnsupportedBackend;

/// Creates the audio backend.
#[must_use]
pub fn audio_backend() -> AudioBackend {
    crate::audio::unsupported::UnsupportedBackend
}

/// Whether [`reset_base_usb`] can work on this OS.
pub const USB_RESET_SUPPORTED: bool = false;
/// Whether [`watch_usb_underflows`] can work on this OS.
pub const UNDERFLOW_WATCH_SUPPORTED: bool = false;

/// Registry key for per-user start-up programs.
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
/// Value name under [`RUN_KEY`].
const RUN_VALUE: &str = "StealSeries";

/// Starts the app at login via the per-user `Run` registry value (**untested**, verify on Windows).
///
/// # Errors
/// A user-facing message if `reg.exe` fails.
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let mut command = Command::new("reg");
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
