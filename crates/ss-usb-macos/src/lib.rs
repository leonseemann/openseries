//! USB helpers for macOS (`docs/hardware/macos-app.md`, "Reconnect USB").
//!
//! [`reenumerate`] asks the USB stack to reset and re-enumerate a device — the software
//! equivalent of unplugging and replugging the data connection. The device keeps its power
//! (**Observed** on the Nova Pro base station: the display did not restart, audio resumed
//! normally within about a second). Needs no admin rights.

// FFI adapter crate: `unsafe` is allowed here only (AGENTS.md). Every block has a SAFETY note.
#![allow(unsafe_code)]

use thiserror::Error;

/// Re-enumeration failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UsbError {
    /// No device with the given ids is connected.
    #[error("USB device {vendor:04x}:{products:04x?} not found")]
    NotFound {
        /// Vendor id.
        vendor: u16,
        /// Product ids tried.
        products: Vec<u16>,
    },
    /// The device could not be opened or reset.
    #[error("USB {what} failed: {reason}")]
    Failed {
        /// The step that failed.
        what: &'static str,
        /// The system's description.
        reason: String,
    },
    /// Not available on this operating system.
    #[error("USB reset is not supported on this platform")]
    Unsupported,
}

/// Resets and re-enumerates the first connected device with `vendor` and one of `products`.
///
/// # Errors
/// [`UsbError`] if no such device exists or the system refuses the reset.
pub fn reenumerate(vendor: u16, products: &[u16]) -> Result<(), UsbError> {
    #[cfg(target_os = "macos")]
    {
        macos::reenumerate(vendor, products)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (vendor, products);
        Err(UsbError::Unsupported)
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ptr::NonNull;

    use objc2::AnyThread;
    use objc2::rc::Retained;
    use objc2_core_foundation::{CFDictionary, CFRetained};
    use objc2_foundation::NSNumber;
    use objc2_io_kit::{
        IOObjectRelease, IOServiceGetMatchingService, io_service_t, kIOMainPortDefault,
    };
    use objc2_io_usb_host::{IOUSBHostDevice, IOUSBHostObjectInitOptions};

    use super::UsbError;

    pub fn reenumerate(vendor: u16, products: &[u16]) -> Result<(), UsbError> {
        let service = products
            .iter()
            .find_map(|&product| matching_service(vendor, product))
            .ok_or_else(|| UsbError::NotFound {
                vendor,
                products: products.to_vec(),
            })?;
        let result = reset(service);
        // Releases the registry reference returned by IOServiceGetMatchingService (safe binding).
        let _ = IOObjectRelease(service);
        result
    }

    /// The registry entry of the device, or `None`.
    fn matching_service(vendor: u16, product: u16) -> Option<io_service_t> {
        // SAFETY: class method with plain NSNumber arguments; unused criteria are nil.
        let matching = unsafe {
            IOUSBHostDevice::createMatchingDictionaryWithVendorID_productID_bcdDevice_deviceClass_deviceSubclass_deviceProtocol_speed_productIDArray(
                Some(&NSNumber::new_u16(vendor)),
                Some(&NSNumber::new_u16(product)),
                None,
                None,
                None,
                None,
                None,
                None,
            )
        };
        // The dictionary is toll-free bridged; IOServiceGetMatchingService consumes one reference.
        let raw = NonNull::new(Retained::into_raw(matching).cast::<CFDictionary>())?;
        // SAFETY: `raw` is the +1 reference we just released from `Retained`.
        let dictionary = unsafe { CFRetained::from_raw(raw) };
        // SAFETY: default main port and a valid, owned matching dictionary.
        let service = unsafe { IOServiceGetMatchingService(kIOMainPortDefault, Some(dictionary)) };
        (service != 0).then_some(service)
    }

    fn reset(service: io_service_t) -> Result<(), UsbError> {
        let mut error = None;
        // SAFETY: valid service, no queue (synchronous use), no interest handler.
        let device = unsafe {
            IOUSBHostDevice::initWithIOService_options_queue_error_interestHandler(
                IOUSBHostDevice::alloc(),
                service,
                IOUSBHostObjectInitOptions::empty(),
                None,
                Some(&mut error),
                std::ptr::null_mut(),
            )
        };
        let device = device.ok_or_else(|| UsbError::Failed {
            what: "open",
            reason: error.map_or_else(
                || "unknown error".to_owned(),
                |e| e.localizedDescription().to_string(),
            ),
        })?;
        // SAFETY: plain method call on an opened device object.
        let result = unsafe { device.resetWithError() }.map_err(|e| UsbError::Failed {
            what: "reset",
            reason: e.localizedDescription().to_string(),
        });
        // SAFETY: closes the user client opened above; the object is not used afterwards.
        unsafe { device.destroy() };
        result
    }
}
