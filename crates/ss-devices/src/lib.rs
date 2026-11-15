//! Registry of all drivers and transports (ADR 0008, `docs/architecture/devices.md` §2).
//!
//! The only crate that names concrete drivers. Adding a device = one entry in [`DRIVERS`];
//! adding a transport = one entry in [`discoveries`].

use ss_device::{Candidate, Discovery, DriverEntry, Match, Support};

/// Every driver, in matching priority order.
pub static DRIVERS: &[&DriverEntry] = &[&ss_nova_pro::device::DRIVER];

/// One discovery per transport.
#[must_use]
pub fn discoveries() -> Vec<Box<dyn Discovery>> {
    vec![Box::new(ss_hid::HidDiscovery::new())]
}

/// The drivers the app may use: verified ones, plus experimental ones when the user enabled them.
pub fn enabled(experimental: bool) -> impl Iterator<Item = &'static DriverEntry> {
    DRIVERS
        .iter()
        .copied()
        .filter(move |d| experimental || d.info.support == Support::Verified)
}

/// The first enabled driver that recognises `candidate`, with the matching rule.
#[must_use]
pub fn driver_for(
    candidate: &Candidate,
    experimental: bool,
) -> Option<(&'static DriverEntry, &'static Match)> {
    enabled(experimental).find_map(|driver| driver.rule_for(candidate).map(|rule| (driver, rule)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_ids_are_unique() {
        let mut ids: Vec<&str> = DRIVERS.iter().map(|d| d.info.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), DRIVERS.len());
    }

    #[test]
    fn the_nova_pro_is_enabled_by_default() {
        assert!(enabled(false).any(|d| d.info.id == "arctis-nova-pro-wireless"));
    }
}
