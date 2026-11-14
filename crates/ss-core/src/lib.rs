//! Platform-independent domain model of StealSeries.
//!
//! This crate contains value objects (validated settings), domain events and the
//! *port* traits that adapters (HID, audio backends, storage) implement. It has no
//! OS-specific dependencies; see `docs/architecture/overview.md` §3.

pub mod battery;
pub mod error;
pub mod headset;
pub mod transport;

pub use error::RangeError;
