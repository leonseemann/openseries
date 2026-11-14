//! macOS implementations of the platform surface.

mod autostart;
mod underflow;

pub use autostart::set as set_autostart;
pub use underflow::{UnderflowWatch, watch_usb_underflows};

use ss_nova_pro::ids;

/// The CoreAudio backend.
pub type AudioBackend = crate::audio::macos::MacAudioBackend;

/// Creates the audio backend (called on the audio-control thread).
#[must_use]
pub fn audio_backend() -> AudioBackend {
    crate::audio::macos::MacAudioBackend
}

/// Channel DSP runs in `ss-engine` on the tapped audio.
pub const CHANNEL_DSP_SUPPORTED: bool = true;
/// Whether [`reset_base_usb`] can work on this OS.
pub const USB_RESET_SUPPORTED: bool = true;
/// Whether [`watch_usb_underflows`] can work on this OS.
pub const UNDERFLOW_WATCH_SUPPORTED: bool = true;

/// Software re-plug of the Nova Pro base station.
///
/// # Errors
/// A user-facing message.
pub fn reset_base_usb() -> Result<(), String> {
    ss_usb_macos::reenumerate(ids::VENDOR_ID, &ids::PRODUCT_IDS).map_err(|e| e.to_string())
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
