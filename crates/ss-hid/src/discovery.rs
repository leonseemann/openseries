//! HID transport of the device platform (`devices.md` §5): finds USB **and** Bluetooth HID
//! devices (hidapi reports the bus) and opens the collections a driver's [`HidMatch`] names.

use std::collections::BTreeMap;
use std::ffi::CString;

use hidapi::{BusType, DeviceInfo, HidApi};
use ss_device::transport::{
    Bus, Candidate, Collection, Connection, Discovery, Match, TransportError,
};

use crate::HidapiTransport;

/// [`Discovery`] over hidapi. Keeps one hidapi context and refreshes its device list per scan.
#[derive(Default)]
pub struct HidDiscovery {
    api: Option<HidApi>,
}

impl HidDiscovery {
    /// A discovery that initialises hidapi on the first scan.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Discovery for HidDiscovery {
    fn name(&self) -> &'static str {
        "HID"
    }

    fn scan(&mut self) -> Vec<Candidate> {
        let api = match &mut self.api {
            Some(api) => {
                if api.refresh_devices().is_err() {
                    return Vec::new();
                }
                api
            }
            None => match HidApi::new() {
                Ok(api) => self.api.insert(api),
                Err(_) => return Vec::new(),
            },
        };
        group(api.device_list())
    }

    fn open(
        &mut self,
        candidate: &Candidate,
        rule: &Match,
    ) -> Result<Box<dyn Connection>, TransportError> {
        let Match::Hid(rule) = rule;
        let api = self.api.as_ref().ok_or(TransportError::Disconnected)?;
        let path_of = |page: u16| {
            candidate
                .collections
                .iter()
                .find(|c| c.usage_page == page)
                .map(|c| c.path.clone())
        };
        let command = path_of(rule.command_usage_page).ok_or(TransportError::Disconnected)?;
        let events = rule
            .event_usage_page
            .and_then(path_of)
            .filter(|path| *path != command);
        let transport = HidapiTransport::open_paths(api, &command, events.as_deref())
            .map_err(|e| TransportError::Io(e.to_string()))?;
        Ok(Box::new(transport))
    }
}

/// Groups HID collections into one candidate per device interface.
fn group<'a>(infos: impl Iterator<Item = &'a DeviceInfo>) -> Vec<Candidate> {
    type Key = (u8, u16, u16, Option<String>, i32);
    let mut candidates: BTreeMap<Key, Candidate> = BTreeMap::new();
    for info in infos {
        let bus = bus_of(info.bus_type());
        let serial = info
            .serial_number()
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        let key = (
            bus_order(bus),
            info.vendor_id(),
            info.product_id(),
            serial.clone(),
            info.interface_number(),
        );
        let candidate = candidates.entry(key).or_insert_with(|| Candidate {
            bus,
            vendor_id: info.vendor_id(),
            product_id: info.product_id(),
            release: Some(info.release_number()),
            serial,
            interface: info.interface_number(),
            product: info.product_string().map(str::to_owned),
            collections: Vec::new(),
        });
        candidate.collections.push(Collection {
            usage_page: info.usage_page(),
            usage: info.usage(),
            path: info.path().to_string_lossy().into_owned(),
        });
    }
    candidates.into_values().collect()
}

const fn bus_of(bus: BusType) -> Bus {
    match bus {
        BusType::Usb => Bus::Usb,
        BusType::Bluetooth => Bus::Bluetooth,
        _ => Bus::Other,
    }
}

/// Stable ordering of buses for deterministic candidate order.
const fn bus_order(bus: Bus) -> u8 {
    match bus {
        Bus::Usb => 0,
        Bus::Bluetooth => 1,
        Bus::Other => 2,
    }
}

/// A HID path as hidapi needs it.
pub(crate) fn c_path(path: &str) -> Result<CString, hidapi::HidError> {
    CString::new(path).map_err(|_| hidapi::HidError::HidApiError {
        message: format!("invalid device path {path:?}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lists the attached HID devices (enumeration only, nothing is opened or sent):
    /// `cargo test -p ss-hid -- --ignored --nocapture scan`.
    #[test]
    #[ignore = "needs the local HID devices"]
    fn scan_lists_attached_devices() {
        for c in HidDiscovery::new().scan() {
            let pages: Vec<String> = c
                .collections
                .iter()
                .map(|x| format!("{:04x}", x.usage_page))
                .collect();
            println!(
                "{:?} {:04x}:{:04x} if{} {:?} pages {}",
                c.bus,
                c.vendor_id,
                c.product_id,
                c.interface,
                c.product,
                pages.join(",")
            );
        }
    }
}
