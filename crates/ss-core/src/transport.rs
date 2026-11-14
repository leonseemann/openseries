//! Port: raw HID report transport.
//!
//! Device drivers (e.g. `ss-nova-pro`) talk to hardware only through this trait, so they
//! can be unit-tested with an in-memory fake and run on any OS that has an adapter.

use std::time::Duration;

use thiserror::Error;

/// Errors produced by a [`HidTransport`] implementation.
#[derive(Debug, Error)]
pub enum TransportError {
    /// The device was unplugged or the handle became invalid.
    #[error("device disconnected")]
    Disconnected,
    /// The OS / HID library reported an error.
    #[error("HID I/O failed: {0}")]
    Io(String),
}

/// Bidirectional transport for HID reports of one logical device.
///
/// Reports include the report-ID byte at index 0.
pub trait HidTransport: Send {
    /// Sends one output report.
    ///
    /// # Errors
    /// Returns [`TransportError`] if the report could not be delivered.
    fn write(&mut self, report: &[u8]) -> Result<(), TransportError>;

    /// Waits up to `timeout` for one input report and copies it into `buf`.
    ///
    /// Returns the number of bytes written to `buf`, or `0` on timeout.
    ///
    /// # Errors
    /// Returns [`TransportError`] if reading failed.
    fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError>;
}
