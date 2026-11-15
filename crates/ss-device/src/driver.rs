//! Driver registration (`devices.md` §4): a driver is a [`DriverEntry`] in the registry crate
//! `ss-devices`.

use crate::device::{Device, DeviceError, DeviceInfo};
use crate::transport::{Candidate, Connection, Match};

/// Opens a device over an established connection.
pub type OpenFn = fn(Box<dyn Connection>, &Candidate) -> Result<Box<dyn Device>, DeviceError>;

/// One driver: what it is, which devices it recognises and how to open them.
#[derive(Debug, Clone, Copy)]
pub struct DriverEntry {
    /// Static description of the device family.
    pub info: &'static DeviceInfo,
    /// Match rules; the first accepting rule is used to open the device.
    pub matches: &'static [Match],
    /// Constructor.
    pub open: OpenFn,
}

impl DriverEntry {
    /// The rule that accepts `candidate`, if any.
    #[must_use]
    pub fn rule_for(&self, candidate: &Candidate) -> Option<&'static Match> {
        self.matches.iter().find(|rule| rule.accepts(candidate))
    }
}
