//! macOS implementations of the platform surface.

mod app_info;
mod autostart;
mod driver;
mod underflow;
mod update;

pub use app_info::AppInfoCache;
pub use autostart::set as set_autostart;

/// Removes what the former app name (StealSeries) registered: its login item. Its virtual-mic
/// driver is replaced by "Install driver…" ([`install_mic_driver`]); settings move with
/// `ss-store` (ADR 0007).
pub fn remove_legacy_registrations() {
    autostart::remove_legacy();
}
pub use driver::{install_mic_driver, mic_driver_installed};
pub use underflow::{UnderflowWatch, watch_usb_underflows};
pub use update::{install_update, update_install_target};

/// The CoreAudio backend.
pub type AudioBackend = crate::audio::macos::MacAudioBackend;

/// Creates the audio backend (called on the audio-control thread).
#[must_use]
pub fn audio_backend() -> AudioBackend {
    crate::audio::macos::MacAudioBackend::default()
}

/// The HAL driver provides "OpenSeries Microphone".
pub const VIRTUAL_MIC_SUPPORTED: bool = true;
/// Channel DSP runs in `ss-engine` on the tapped audio.
pub const CHANNEL_DSP_SUPPORTED: bool = true;
/// Whether [`reset_usb`] can work on this OS.
pub const USB_RESET_SUPPORTED: bool = true;
/// Whether [`watch_usb_underflows`] can work on this OS.
pub const UNDERFLOW_WATCH_SUPPORTED: bool = true;

/// Bundle identifier of `OpenSeries.app` (`bundle/Info.plist`).
const BUNDLE_ID: &str = "dev.openseries.app";

/// Sends our notifications as OpenSeries.app (name and icon) instead of another app.
pub fn init_notifications() {
    let _ = notify_rust::set_application(BUNDLE_ID);
}

/// Nothing per notification on macOS ([`init_notifications`] covers it).
pub fn prepare_notification(_notification: &mut notify_rust::Notification) {}

/// Ends a process that did not quit when asked (single instance, `app.md` §9).
pub fn terminate_process(pid: u32) {
    let _ = std::process::Command::new("/bin/kill")
        .args(["-9", &pid.to_string()])
        .status();
}

/// Software re-plug of a USB device (the device platform's `UsbReplug` feature).
///
/// # Errors
/// A user-facing message.
pub fn reset_usb(vendor_id: u16, product_id: u16) -> Result<(), String> {
    ss_usb_macos::reenumerate(vendor_id, &[product_id]).map_err(|e| e.to_string())
}

/// Not needed on this OS: eframe's `ViewportCommand::Visible` works on hidden windows here.
#[derive(Debug, Clone, Copy)]
pub struct NativeWindow;

impl NativeWindow {
    /// Always `None`: the viewport command is enough on this OS.
    pub fn of<T: ?Sized>(_source: &T) -> Option<Self> {
        None
    }

    /// No-op.
    pub fn show(self) {}
}
