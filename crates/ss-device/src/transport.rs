//! Transports (`devices.md` §5): how devices are found and reached.
//!
//! A [`Discovery`] lists [`Candidate`]s of its transport; drivers say with [`Match`] rules which
//! candidates they speak to. Only matched candidates are ever opened, so foreign devices are never
//! touched (`app.md` §8).

pub use ss_core::transport::{HidTransport as Connection, TransportError};

/// The bus a HID device is attached through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bus {
    /// USB (including 2.4 GHz dongles).
    Usb,
    /// Bluetooth Classic or BLE "HID over GATT", exposed as HID by the OS.
    Bluetooth,
    /// Anything else the OS reports.
    Other,
}

/// One HID top-level collection of a candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    /// HID usage page.
    pub usage_page: u16,
    /// HID usage.
    pub usage: u16,
    /// OS path used to open it.
    pub path: String,
}

/// A device interface found by a [`Discovery`], before any driver talks to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Bus it is attached through.
    pub bus: Bus,
    /// Vendor id (USB or Bluetooth).
    pub vendor_id: u16,
    /// Product id.
    pub product_id: u16,
    /// Device release number (`bcdDevice`), if reported.
    pub release: Option<u16>,
    /// Serial number, if reported.
    pub serial: Option<String>,
    /// Interface number (`-1` if the bus has none).
    pub interface: i32,
    /// Product string, if reported.
    pub product: Option<String>,
    /// Its HID collections.
    pub collections: Vec<Collection>,
}

/// A HID match rule of a driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HidMatch {
    /// Required bus; `None` accepts any.
    pub bus: Option<Bus>,
    /// Vendor id.
    pub vendor_id: u16,
    /// Accepted product ids.
    pub product_ids: &'static [u16],
    /// Required interface number; `None` accepts any.
    pub interface: Option<i32>,
    /// Usage page of the collection that takes commands (output reports).
    pub command_usage_page: u16,
    /// Usage page of a separate collection that delivers events, if the device splits them.
    pub event_usage_page: Option<u16>,
}

/// How a driver recognises its devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    /// A HID device (USB or Bluetooth).
    Hid(HidMatch),
}

impl Match {
    /// Whether `candidate` is one of this rule's devices.
    #[must_use]
    pub fn accepts(&self, candidate: &Candidate) -> bool {
        match self {
            Self::Hid(rule) => {
                rule.bus.is_none_or(|bus| bus == candidate.bus)
                    && rule.vendor_id == candidate.vendor_id
                    && rule.product_ids.contains(&candidate.product_id)
                    && rule.interface.is_none_or(|i| i == candidate.interface)
                    && candidate
                        .collections
                        .iter()
                        .any(|c| c.usage_page == rule.command_usage_page)
            }
        }
    }
}

/// Finds and opens devices of one transport.
pub trait Discovery: Send {
    /// Short name for logs ("HID", "BLE").
    fn name(&self) -> &'static str;

    /// Lists the currently attached candidates (cheap; called every few seconds).
    fn scan(&mut self) -> Vec<Candidate>;

    /// Opens `candidate` the way `rule` describes.
    ///
    /// # Errors
    /// [`TransportError`] if the OS refuses or the device is gone.
    fn open(
        &mut self,
        candidate: &Candidate,
        rule: &Match,
    ) -> Result<Box<dyn Connection>, TransportError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULE: Match = Match::Hid(HidMatch {
        bus: Some(Bus::Usb),
        vendor_id: 0x1038,
        product_ids: &[0x12e0, 0x12e5],
        interface: Some(4),
        command_usage_page: 0xffc0,
        event_usage_page: Some(0xff00),
    });

    fn candidate() -> Candidate {
        Candidate {
            bus: Bus::Usb,
            vendor_id: 0x1038,
            product_id: 0x12e0,
            release: Some(0x0130),
            serial: None,
            interface: 4,
            product: None,
            collections: vec![
                Collection {
                    usage_page: 0xffc0,
                    usage: 1,
                    path: "a".to_owned(),
                },
                Collection {
                    usage_page: 0xff00,
                    usage: 1,
                    path: "b".to_owned(),
                },
            ],
        }
    }

    #[test]
    fn matching_candidate_is_accepted() {
        assert!(RULE.accepts(&candidate()));
    }

    #[test]
    fn other_bus_product_interface_or_page_is_rejected() {
        let mut c = candidate();
        c.bus = Bus::Bluetooth;
        assert!(!RULE.accepts(&c));
        let mut c = candidate();
        c.product_id = 0x1234;
        assert!(!RULE.accepts(&c));
        let mut c = candidate();
        c.interface = 3;
        assert!(!RULE.accepts(&c));
        let mut c = candidate();
        c.collections.remove(0);
        assert!(!RULE.accepts(&c));
    }
}
