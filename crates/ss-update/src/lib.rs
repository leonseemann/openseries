//! In-app updates without the network or OS parts (ADR 0006): which release is newer, which
//! package of it fits this computer, and whether a downloaded package carries our signature.
//!
//! Downloading and installing live in `ss-app` (`src/update/`).

pub mod release;
pub mod signature;

pub use release::{Asset, Platform, Release, Update};
pub use semver;
pub use signature::{SignatureError, verify};

/// GitHub repository that publishes the releases.
pub const REPOSITORY: &str = "leonseemann/openseries";

/// GitHub API endpoint of the newest published (non-draft, non-prerelease) release.
#[must_use]
pub fn latest_release_url() -> String {
    format!("https://api.github.com/repos/{REPOSITORY}/releases/latest")
}

/// Suffix of the detached signature file next to each package.
pub const SIGNATURE_SUFFIX: &str = ".sig";
