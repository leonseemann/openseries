//! `hidapi` adapter for the [`HidTransport`] port of `ss-core`.
//!
//! Some devices (e.g. the Nova Pro Wireless base station) expose commands and events on
//! two HID top-level collections of the same USB interface. On Windows and macOS each
//! collection is a separate device path, so [`HidapiTransport`] opens both and reads
//! from both. On Linux (hidraw) both collections share one path.

use std::time::{Duration, Instant};

use hidapi::{DeviceInfo, HidApi, HidDevice};
use ss_core::transport::{HidTransport, TransportError};
use thiserror::Error;

/// Slice of the overall timeout spent polling each collection per round.
const POLL_SLICE: Duration = Duration::from_millis(5);

/// Which HID collections belong to a device.
#[derive(Debug, Clone, Copy)]
pub struct DeviceSelector<'a> {
    /// USB vendor ID.
    pub vendor_id: u16,
    /// Accepted product IDs.
    pub product_ids: &'a [u16],
    /// USB interface number.
    pub interface: i32,
    /// Usage page of the collection that accepts output (command) reports.
    pub command_usage_page: u16,
    /// Usage page of the collection that delivers input (event) reports.
    pub event_usage_page: u16,
}

impl DeviceSelector<'_> {
    fn matches_interface(&self, info: &DeviceInfo) -> bool {
        info.vendor_id() == self.vendor_id
            && self.product_ids.contains(&info.product_id())
            && info.interface_number() == self.interface
    }
}

/// A HID collection found during enumeration (for diagnostics / `ssctl list`).
#[derive(Debug, Clone)]
pub struct FoundCollection {
    /// USB product ID.
    pub product_id: u16,
    /// Product string, if the OS provides one.
    pub product: Option<String>,
    /// HID usage page of this collection.
    pub usage_page: u16,
    /// OS device path.
    pub path: String,
}

/// Errors when opening a device.
#[derive(Debug, Error)]
pub enum OpenError {
    /// hidapi could not be initialised or the device could not be opened.
    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),
    /// No matching device is connected.
    #[error("no matching device found")]
    NotFound,
}

/// Lists all collections of connected devices that match `selector`.
#[must_use]
pub fn find_collections(api: &HidApi, selector: &DeviceSelector<'_>) -> Vec<FoundCollection> {
    api.device_list()
        .filter(|info| selector.matches_interface(info))
        .map(|info| FoundCollection {
            product_id: info.product_id(),
            product: info.product_string().map(str::to_owned),
            usage_page: info.usage_page(),
            path: info.path().to_string_lossy().into_owned(),
        })
        .collect()
}

/// Transport over one command collection and an optional separate event collection.
pub struct HidapiTransport {
    command: HidDevice,
    events: Option<HidDevice>,
}

impl HidapiTransport {
    /// Opens the first connected device matching `selector`.
    ///
    /// # Errors
    /// [`OpenError::NotFound`] if no device matches, [`OpenError::Hid`] on OS errors.
    pub fn open(api: &HidApi, selector: &DeviceSelector<'_>) -> Result<Self, OpenError> {
        let candidates: Vec<&DeviceInfo> = api
            .device_list()
            .filter(|info| selector.matches_interface(info))
            .collect();

        let by_page = |page: u16| candidates.iter().find(|i| i.usage_page() == page);
        let command_info = by_page(selector.command_usage_page)
            .or_else(|| candidates.first())
            .ok_or(OpenError::NotFound)?;
        let event_info =
            by_page(selector.event_usage_page).filter(|info| info.path() != command_info.path());

        Ok(Self {
            command: command_info.open_device(api)?,
            events: event_info.map(|info| info.open_device(api)).transpose()?,
        })
    }

    fn read_one(
        device: &HidDevice,
        buf: &mut [u8],
        timeout: Duration,
    ) -> Result<usize, TransportError> {
        let millis = i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX);
        device.read_timeout(buf, millis).map_err(to_transport_error)
    }
}

impl HidTransport for HidapiTransport {
    fn write(&mut self, report: &[u8]) -> Result<(), TransportError> {
        self.command
            .write(report)
            .map(|_| ())
            .map_err(to_transport_error)
    }

    fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(events) = &self.events {
                let n = Self::read_one(events, buf, POLL_SLICE.min(timeout))?;
                if n > 0 {
                    return Ok(n);
                }
            }
            let n = Self::read_one(&self.command, buf, POLL_SLICE.min(timeout))?;
            if n > 0 || Instant::now() >= deadline {
                return Ok(n);
            }
        }
    }
}

#[allow(clippy::needless_pass_by_value)] // signature required by `map_err`
fn to_transport_error(err: hidapi::HidError) -> TransportError {
    TransportError::Io(err.to_string())
}

/// Initialises hidapi.
///
/// # Errors
/// Returns the hidapi error if the library cannot be initialised.
pub fn api() -> Result<HidApi, OpenError> {
    Ok(HidApi::new()?)
}
