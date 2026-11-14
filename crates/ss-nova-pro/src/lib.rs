//! Driver for the SteelSeries **Arctis Nova Pro Wireless** base station.
//!
//! * [`Command`] / [`Event`] — pure encoding and decoding of HID reports (no I/O).
//! * [`NovaProWireless`] — a session over any [`ss_core::transport::HidTransport`].
//!
//! # Evidence status
//! All protocol constants are **Inferred** from open-source projects, summarised with
//! source links in `docs/research/prior-art.md` §1.2–§1.3. They must be verified on
//! hardware; GG's own device specs are encrypted and are deliberately not used
//! (`docs/re/engine.md` §4.2).
//!
//! Dangerous operations (factory reset, firmware update) are intentionally **not**
//! representable by [`Command`].

mod codec;
mod command;
mod driver;
mod event;
mod firmware;
pub mod ids;
mod protocol;

pub use command::Command;
pub use driver::{DriverError, NovaProWireless};
pub use event::{DecodeError, Display, Event, Settings, Status};
pub use firmware::{FIRMWARE_VERSIONS, FirmwareVersions, Version};
pub use protocol::REPORT_LEN;
