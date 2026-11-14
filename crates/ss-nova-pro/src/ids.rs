//! USB identifiers of the base station (prior-art.md §1.2).

/// SteelSeries USB vendor ID.
pub const VENDOR_ID: u16 = 0x1038;

/// Product IDs of the base station in normal operation.
pub const PRODUCT_IDS: [u16; 3] = [
    0x12E0, // Nova Pro Wireless (PC / PlayStation)
    0x12E5, // Nova Pro Wireless X (Xbox)
    0x225D, // Nova Pro Wireless X (white)
];

/// Product IDs while the base station is in firmware-update mode. Never talk to these.
pub const BOOTLOADER_PRODUCT_IDS: [u16; 3] = [0x12E1, 0x12E7, 0x225F];

/// USB interface number carrying the vendor HID collections.
pub const CONTROL_INTERFACE: i32 = 4;

/// HID usage page of the collection that accepts command (output) reports.
pub const COMMAND_USAGE_PAGE: u16 = 0xFFC0;

/// HID usage page of the collection that delivers event (input) reports.
pub const EVENT_USAGE_PAGE: u16 = 0xFF00;

/// Returns `true` if `(vendor_id, product_id)` is a base station in normal mode.
#[must_use]
pub fn is_supported(vendor_id: u16, product_id: u16) -> bool {
    vendor_id == VENDOR_ID && PRODUCT_IDS.contains(&product_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootloader_ids_are_not_supported() {
        for pid in BOOTLOADER_PRODUCT_IDS {
            assert!(!is_supported(VENDOR_ID, pid));
        }
        assert!(is_supported(VENDOR_ID, 0x12E0));
        assert!(!is_supported(0x0000, 0x12E0));
    }
}
