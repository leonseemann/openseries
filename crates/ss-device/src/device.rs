//! The [`Device`] trait and what describes a device (`devices.md` §4).

use std::time::Duration;

use crate::feature::{Feature, FeatureId, Value, ValueError};
use crate::transport::{Bus, Candidate, TransportError};

/// Display category of a device. Only a label and an icon: code never branches on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceClass {
    /// Headset or headphones.
    Headset,
    /// Keyboard.
    Keyboard,
    /// Mouse.
    Mouse,
    /// Anything else.
    Other,
}

/// How well a driver is tested (`devices.md` §4 of ADR 0008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Support {
    /// Tested on hardware; enabled by default.
    Verified,
    /// Built from public facts or partly tested; off unless the user enables experimental devices.
    Experimental,
}

/// Static description of a device family (one per driver).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Stable id, used as config key, e.g. `"arctis-nova-pro-wireless"`.
    pub id: &'static str,
    /// User-facing name.
    pub name: &'static str,
    /// Display category.
    pub class: DeviceClass,
    /// Test status.
    pub support: Support,
    /// Name of the part the computer is connected to, for messages ("base station", "dongle").
    pub hub_name: Option<&'static str>,
}

/// One connected device of a family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// Unique key: driver id plus serial (or vendor/product id when there is none).
    pub key: String,
    /// Bus it is attached through.
    pub bus: Bus,
    /// Vendor id.
    pub vendor_id: u16,
    /// Product id.
    pub product_id: u16,
    /// Device release number, if reported.
    pub release: Option<u16>,
    /// Serial number, if reported.
    pub serial: Option<String>,
}

impl Instance {
    /// The instance of driver `driver_id` behind `candidate`.
    #[must_use]
    pub fn of(driver_id: &str, candidate: &Candidate) -> Self {
        let suffix = candidate
            .serial
            .clone()
            .unwrap_or_else(|| format!("{:04x}-{:04x}", candidate.vendor_id, candidate.product_id));
        Self {
            key: format!("{driver_id}/{suffix}"),
            bus: candidate.bus,
            vendor_id: candidate.vendor_id,
            product_id: candidate.product_id,
            release: candidate.release,
            serial: candidate.serial.clone(),
        }
    }
}

/// A role the app can take over on a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// The app mixes Game/Chat itself and drives the device's balance dial (AudioMix on).
    HostMixer,
}

/// Something that changed on the device.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// One feature has a new value (button, dial, battery, link …).
    Value(FeatureId, Value),
    /// Several things may have changed; read everything again.
    Refresh,
}

/// Why a device operation failed.
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    /// The connection failed or the device is gone.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The device has no such writable feature.
    #[error("not supported by this device: {0:?}")]
    Unsupported(FeatureId),
    /// The value does not fit the feature.
    #[error(transparent)]
    InvalidValue(#[from] ValueError),
    /// The device answered something unexpected.
    #[error("device protocol error: {0}")]
    Protocol(String),
}

/// A connected device, driven by its driver. One actor thread owns it (`devices.md` §6).
pub trait Device: Send {
    /// Static description.
    fn info(&self) -> &'static DeviceInfo;

    /// This connected instance.
    fn instance(&self) -> &Instance;

    /// What this device can do.
    fn features(&self) -> &[Feature];

    /// Reads all readable feature values.
    ///
    /// # Errors
    /// [`DeviceError`] if the device does not answer.
    fn refresh(&mut self) -> Result<Vec<(FeatureId, Value)>, DeviceError>;

    /// Writes one value. Callers check it first with [`Feature::check`]; drivers check again.
    ///
    /// # Errors
    /// [`DeviceError::Unsupported`] for unknown or read-only features, transport errors otherwise.
    fn set(&mut self, id: FeatureId, value: &Value) -> Result<(), DeviceError>;

    /// Waits up to `timeout` for the next change initiated by the device.
    ///
    /// # Errors
    /// [`DeviceError::Transport`] when the device is gone.
    fn next_change(&mut self, timeout: Duration) -> Result<Option<Change>, DeviceError>;

    /// The app takes over (`true`) or releases (`false`) a role.
    ///
    /// # Errors
    /// [`DeviceError`] if the device does not answer.
    fn set_role(&mut self, role: Role, active: bool) -> Result<(), DeviceError> {
        let _ = (role, active);
        Ok(())
    }

    /// Returns the device to its normal state before the app lets go of it.
    ///
    /// # Errors
    /// [`DeviceError`] if the device does not answer.
    fn shutdown(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }

    /// The feature `id`, if the device has it.
    fn feature(&self, id: FeatureId) -> Option<&Feature> {
        self.features().iter().find(|f| f.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::Collection;

    fn candidate(serial: Option<&str>) -> Candidate {
        Candidate {
            bus: Bus::Usb,
            vendor_id: 0x1038,
            product_id: 0x12e0,
            release: None,
            serial: serial.map(str::to_owned),
            interface: 4,
            product: None,
            collections: vec![Collection {
                usage_page: 0xffc0,
                usage: 1,
                path: "p".to_owned(),
            }],
        }
    }

    #[test]
    fn instance_key_prefers_the_serial() {
        assert_eq!(Instance::of("x", &candidate(Some("AB12"))).key, "x/AB12");
        assert_eq!(Instance::of("x", &candidate(None)).key, "x/1038-12e0");
    }
}
