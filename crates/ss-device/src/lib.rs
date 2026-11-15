//! The device platform (ADR 0008, `docs/architecture/devices.md`).
//!
//! Three independent extension axes:
//!
//! * [`feature`]: what a device can do (catalogue of feature kinds, descriptors and values);
//! * [`transport`]: how a device is reached (candidates found by a [`transport::Discovery`],
//!   opened as a [`transport::Connection`]);
//! * [`driver`]: which protocol a device speaks ([`device::Device`] implementations, registered
//!   with a [`driver::DriverEntry`]).
//!
//! The app only uses these types; it never names a concrete device.

pub mod device;
pub mod driver;
pub mod feature;
pub mod transport;

pub use device::{Change, Device, DeviceClass, DeviceError, DeviceInfo, Instance, Role, Support};
pub use driver::DriverEntry;
pub use feature::{
    BatterySlot, ChoiceOption, Descriptor, Feature, FeatureId, FeatureKind, LevelRange, Value,
};
pub use transport::{Bus, Candidate, Collection, Connection, Discovery, HidMatch, Match};
