//! Firmware version block (`06 10` reply), `docs/hardware/nova-pro-wireless.md`.
//!
//! **Observed** 2026-11-14: the reply carries four ASCII versions of 12 characters each,
//! `NNNN.NNN.NNN`, e.g. `0000.003.082` `0001.031.000` `0002.002.001` `0002.001.000`. GG shows the
//! same device as `0.3.82/1.31.0/2.2.1/2.1.0` (`physical_devices.firmware_version` in its engine
//! database), so the parts are plain decimal numbers. Which component (base-station MCU, headset
//! MCU, DSP, Bluetooth chip …) each version belongs to is **Unknown**.

use std::fmt;

/// Number of versions in the block (Observed).
pub const FIRMWARE_VERSIONS: usize = 4;
/// Characters per version field, `NNNN.NNN.NNN`.
const FIELD_LEN: usize = 12;

/// One `major.minor.patch` version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    /// Major.
    pub major: u16,
    /// Minor.
    pub minor: u16,
    /// Patch.
    pub patch: u16,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Version {
    fn parse(field: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(field).ok()?;
        let mut parts = text.split('.').map(|p| p.parse::<u16>().ok());
        let version = Self {
            major: parts.next()??,
            minor: parts.next()??,
            patch: parts.next()??,
        };
        parts.next().is_none().then_some(version)
    }
}

/// The four versions, in the order the base station reports them (as GG shows them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareVersions(pub [Version; FIRMWARE_VERSIONS]);

impl FirmwareVersions {
    /// Parses the argument bytes of the reply (after report ID and opcode). `None` if the layout
    /// does not match.
    #[must_use]
    pub fn parse(args: &[u8]) -> Option<Self> {
        let mut versions = [Version {
            major: 0,
            minor: 0,
            patch: 0,
        }; FIRMWARE_VERSIONS];
        for (index, slot) in versions.iter_mut().enumerate() {
            let start = index * FIELD_LEN;
            *slot = Version::parse(args.get(start..start + FIELD_LEN)?)?;
        }
        Some(Self(versions))
    }
}

impl fmt::Display for FirmwareVersions {
    /// Same notation as GG: `0.3.82/1.31.0/2.2.1/2.1.0`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, version) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str("/")?;
            }
            write!(f, "{version}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Argument bytes of the reply captured on the owner's base station.
    const OBSERVED: &[u8] = b"0000.003.0820001.031.0000002.002.0010002.001.000\0\0\0";

    #[test]
    fn parses_the_observed_reply_like_gg_shows_it() {
        let versions = FirmwareVersions::parse(OBSERVED).unwrap();
        assert_eq!(versions.to_string(), "0.3.82/1.31.0/2.2.1/2.1.0");
    }

    #[test]
    fn rejects_other_layouts() {
        assert!(FirmwareVersions::parse(b"0000.003.082").is_none());
        assert!(FirmwareVersions::parse(&[0; 60]).is_none());
        assert!(
            FirmwareVersions::parse(b"0000.003.0820001.031.0000002.002.0010002.001.00x").is_none()
        );
    }
}
