//! Audio side of the app: what to run ([`plan`]), keeping it running ([`supervisor`]) and the
//! thread that owns the CoreAudio sessions ([`service`]).

#[cfg(target_os = "macos")]
pub mod macos;
pub mod plan;
pub mod service;
pub mod supervisor;
