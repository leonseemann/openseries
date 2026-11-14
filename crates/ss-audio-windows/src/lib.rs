//! Windows backend for AudioMix, milestone M4a: the **session mixer**
//! (`docs/architecture/audio-engine.md` §5, [ADR 0004](../../docs/adr/0004-windows-audio-strategy.md)).
//!
//! Process loopback cannot capture an app while its original playback is silenced (ADR 0004,
//! S3/S4, **Observed**), so without a driver no audio passes through us. Instead each app's
//! session volume follows its channel's fader, the master and ChatMix.
//!
//! [`gains`] and [`ledger`] are platform-free and unit-tested; the COM parts are Windows-only.

// FFI adapter crate: `unsafe` is allowed here only (AGENTS.md). Every block has a SAFETY note.
#![allow(unsafe_code)]

pub mod gains;
pub mod ledger;

#[cfg(windows)]
mod mixer;
#[cfg(windows)]
mod session;

#[cfg(windows)]
pub use mixer::{SessionMixer, restore_pending};
#[cfg(windows)]
pub use session::{Endpoint, SessionInfo, endpoints, init_com, list_sessions};

use thiserror::Error;

/// A Windows audio call failed.
#[derive(Debug, Error)]
pub enum AudioError {
    /// A COM / WASAPI call failed.
    #[error("{context}: {message}")]
    Os {
        /// What we were doing.
        context: &'static str,
        /// The OS error text.
        message: String,
    },
}

impl AudioError {
    /// Wraps an OS error.
    pub fn os(context: &'static str, error: impl std::fmt::Display) -> Self {
        Self::Os {
            context,
            message: error.to_string(),
        }
    }
}
