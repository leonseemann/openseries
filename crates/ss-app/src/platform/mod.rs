//! Everything OS-specific the app needs, behind one small surface (`docs/architecture/app.md` §6).
//!
//! | Function | macOS | Windows |
//! |---|---|---|
//! | [`audio_backend`] | process taps + virtual mic (`ss-audio-macos`) | session mixer, no mic (`ss-audio-windows`, M4a) |
//! | [`set_autostart`] | per-user `LaunchAgent` | `HKCU\…\Run` registry value |
//! | [`reset_usb`] | `IOUSBHost` re-enumeration (`ss-usb-macos`) | not yet |
//! | [`watch_usb_underflows`] | `usbaudiod` log stream | not yet |
//! | [`install_mic_driver`] | copies the bundled HAL driver with an admin prompt | not available (ADR 0004) |
//! | [`install_update`] | swaps the `.app` bundle after quitting (ADR 0006) | runs the Inno Setup installer silently |
//! | [`NativeWindow`] | not needed (no-op) | shows the hidden window via Win32 (`ss-window-windows`) |
//!
//! Each OS module exports the same names; the rest of the app never uses `cfg(target_os)`.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod other;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub use other::*;
