//! Audio side of the app: what to run ([`plan`]), keeping it running ([`supervisor`]) and the
//! thread that owns the CoreAudio sessions ([`service`]).

pub mod catalogue;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod plan;
pub mod service;
pub mod supervisor;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub mod unsupported;
#[cfg(target_os = "windows")]
pub mod windows;
