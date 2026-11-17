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
/// Whether [`reset_usb`] can work on this OS.
pub const USB_RESET_SUPPORTED: bool = false;
/// Whether [`watch_usb_underflows`] can work on this OS.
pub const UNDERFLOW_WATCH_SUPPORTED: bool = false;

/// Not available on this OS.
///
/// # Errors
/// Always.
pub fn set_autostart(_enabled: bool) -> Result<(), String> {
    Err(crate::i18n::tr!("platform-autostart-unavailable"))
}

/// Not available on this OS.
///
/// # Errors
/// Always.
pub fn reset_usb(_vendor_id: u16, _product_id: u16) -> Result<(), String> {
    Err(crate::i18n::tr!("platform-usb-reset-unavailable"))
}

/// Nothing to register on this OS.
pub fn init_notifications() {}

/// Nothing per notification on this OS.
pub fn prepare_notification(_notification: &mut notify_rust::Notification) {}

/// Ends a process that did not quit when asked (single instance, `app.md` §9).
pub fn terminate_process(pid: u32) {
    let _ = std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status();
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
    Err(crate::i18n::tr!("platform-crackling-watch-unavailable"))
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
    Err(crate::i18n::tr!("platform-mic-driver-unavailable"))
}

/// No release packages for this OS.
///
/// # Errors
/// Always.
pub fn update_install_target() -> Result<std::path::PathBuf, String> {
    Err(crate::i18n::tr!("platform-update-unavailable"))
}

/// Not available on this OS.
///
/// # Errors
/// Always.
pub fn install_update(_package: &std::path::Path) -> Result<(), String> {
    Err(crate::i18n::tr!("platform-update-unavailable"))
}
