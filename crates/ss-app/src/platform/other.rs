//! Fallback for other operating systems (development builds, e.g. Linux): headset, UI, tray and
//! hotkeys only.

/// Placeholder backend until `ss-audio-windows` exists.
pub type AudioBackend = crate::audio::unsupported::UnsupportedBackend;

/// Creates the audio backend.
#[must_use]
pub fn audio_backend() -> AudioBackend {
    crate::audio::unsupported::UnsupportedBackend
}

/// No virtual microphone on this OS.
pub const VIRTUAL_MIC_SUPPORTED: bool = false;
/// No audio backend, so no channel DSP.
pub const CHANNEL_DSP_SUPPORTED: bool = false;
/// Whether [`reset_base_usb`] can work on this OS.
pub const USB_RESET_SUPPORTED: bool = false;
/// Whether [`watch_usb_underflows`] can work on this OS.
pub const UNDERFLOW_WATCH_SUPPORTED: bool = false;

/// Not available on this OS.
///
/// # Errors
/// Always.
pub fn set_autostart(_enabled: bool) -> Result<(), String> {
    Err("autostart is not available on this operating system".to_owned())
}

/// Not available on this OS.
///
/// # Errors
/// Always.
pub fn reset_base_usb() -> Result<(), String> {
    Err("Reconnect USB is not available on this OS".to_owned())
}

/// Not available on this OS.
pub struct UnderflowWatch;

/// Not available on this OS.
///
/// # Errors
/// Always.
pub fn watch_usb_underflows(
    _on_underflow: impl Fn() + Send + 'static,
) -> Result<UnderflowWatch, String> {
    Err("the crackling watch is not available on this OS".to_owned())
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
