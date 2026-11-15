//! The GitHub release JSON and the choice of the package for this computer (ADR 0006 §2).

use semver::Version;
use serde::Deserialize;

use crate::SIGNATURE_SUFFIX;

/// A published release as returned by `GET /repos/{owner}/{repo}/releases/latest`
/// (only the fields we use).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Release {
    /// Tag such as `v0.2.0`.
    pub tag_name: String,
    /// Release page in the browser.
    pub html_url: String,
    /// Release notes (Markdown).
    #[serde(default)]
    pub body: Option<String>,
    /// Uploaded files.
    #[serde(default)]
    pub assets: Vec<Asset>,
}

/// One uploaded file of a release.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Asset {
    /// File name, e.g. `StealSeries-0.2.0-windows-x64-setup.exe`.
    pub name: String,
    /// Direct download URL.
    pub browser_download_url: String,
    /// Size in bytes.
    pub size: u64,
}

/// The package flavours CI builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Inno Setup installer for 64-bit Windows.
    WindowsX64,
    /// Zipped `StealSeries.app` for Apple Silicon Macs.
    MacosArm64,
}

impl Platform {
    /// The platform this binary was built for, if CI publishes packages for it.
    #[must_use]
    pub const fn current() -> Option<Self> {
        if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            Some(Self::WindowsX64)
        } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            Some(Self::MacosArm64)
        } else {
            None
        }
    }

    /// File name of the package for `version` (the release workflow uses the same names).
    #[must_use]
    pub fn package_name(self, version: &Version) -> String {
        match self {
            Self::WindowsX64 => format!("StealSeries-{version}-windows-x64-setup.exe"),
            Self::MacosArm64 => format!("StealSeries-{version}-macos-arm64.zip"),
        }
    }
}

/// A release that is newer than the running app and has a signed package for this platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    /// The new version.
    pub version: Version,
    /// Release page in the browser.
    pub page_url: String,
    /// Release notes (Markdown, may be empty).
    pub notes: String,
    /// The package to install.
    pub package: Asset,
    /// Its detached signature.
    pub signature: Asset,
}

/// Why a release cannot be offered.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReleaseError {
    /// The API answer is not a release.
    #[error("unexpected answer from GitHub: {0}")]
    Json(String),
    /// The tag is not `vX.Y.Z`.
    #[error("release tag {0:?} is not a version")]
    Tag(String),
    /// The release has no package (or no signature) for this computer.
    #[error("release {version} has no signed package {name}")]
    MissingPackage {
        /// The release version.
        version: Version,
        /// The expected package file name.
        name: String,
    },
}

impl Release {
    /// Parses the API answer.
    ///
    /// # Errors
    /// [`ReleaseError::Json`] if `json` is not a release object.
    pub fn parse(json: &[u8]) -> Result<Self, ReleaseError> {
        serde_json::from_slice(json).map_err(|e| ReleaseError::Json(e.to_string()))
    }

    /// The release version from its tag (`v0.2.0` or `0.2.0`).
    ///
    /// # Errors
    /// [`ReleaseError::Tag`] if the tag is not a semantic version.
    pub fn version(&self) -> Result<Version, ReleaseError> {
        let text = self.tag_name.strip_prefix('v').unwrap_or(&self.tag_name);
        Version::parse(text).map_err(|_| ReleaseError::Tag(self.tag_name.clone()))
    }

    /// The update this release offers to an app at `current` on `platform`: `Ok(None)` if it
    /// is not newer.
    ///
    /// # Errors
    /// If the tag is not a version, or a newer release lacks the signed package.
    pub fn update_for(
        &self,
        platform: Platform,
        current: &Version,
    ) -> Result<Option<Update>, ReleaseError> {
        let version = self.version()?;
        if version <= *current {
            return Ok(None);
        }
        let name = platform.package_name(&version);
        let signature_name = format!("{name}{SIGNATURE_SUFFIX}");
        let find = |wanted: &str| self.assets.iter().find(|a| a.name == wanted).cloned();
        match (find(&name), find(&signature_name)) {
            (Some(package), Some(signature)) => Ok(Some(Update {
                version,
                page_url: self.html_url.clone(),
                notes: self.body.clone().unwrap_or_default(),
                package,
                signature,
            })),
            _ => Err(ReleaseError::MissingPackage { version, name }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LATEST: &str = r#"{
        "tag_name": "v0.2.0",
        "html_url": "https://github.com/leonseemann/stealseries/releases/tag/v0.2.0",
        "body": "Fixes",
        "draft": false,
        "assets": [
            {"name": "StealSeries-0.2.0-windows-x64-setup.exe", "size": 10,
             "browser_download_url": "https://example.invalid/setup.exe"},
            {"name": "StealSeries-0.2.0-windows-x64-setup.exe.sig", "size": 1,
             "browser_download_url": "https://example.invalid/setup.exe.sig"},
            {"name": "StealSeries-0.2.0-macos-arm64.zip", "size": 10,
             "browser_download_url": "https://example.invalid/app.zip"}
        ]
    }"#;

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn newer_release_offers_the_signed_package() {
        let release = Release::parse(LATEST.as_bytes()).unwrap();
        let update = release
            .update_for(Platform::WindowsX64, &version("0.1.0"))
            .unwrap()
            .unwrap();
        assert_eq!(update.version, version("0.2.0"));
        assert_eq!(
            update.package.name,
            "StealSeries-0.2.0-windows-x64-setup.exe"
        );
        assert_eq!(
            update.signature.name,
            "StealSeries-0.2.0-windows-x64-setup.exe.sig"
        );
        assert_eq!(update.notes, "Fixes");
    }

    #[test]
    fn same_or_older_release_is_no_update() {
        let release = Release::parse(LATEST.as_bytes()).unwrap();
        for current in ["0.2.0", "0.3.0", "1.0.0"] {
            assert_eq!(
                release.update_for(Platform::WindowsX64, &version(current)),
                Ok(None)
            );
        }
    }

    #[test]
    fn prerelease_of_the_same_version_is_older() {
        let release = Release::parse(LATEST.as_bytes()).unwrap();
        let update = release.update_for(Platform::WindowsX64, &version("0.2.0-beta.1"));
        assert!(update.unwrap().is_some());
    }

    #[test]
    fn missing_signature_is_an_error() {
        let release = Release::parse(LATEST.as_bytes()).unwrap();
        let error = release
            .update_for(Platform::MacosArm64, &version("0.1.0"))
            .unwrap_err();
        assert!(matches!(error, ReleaseError::MissingPackage { .. }));
    }

    #[test]
    fn tag_without_v_and_bad_tags() {
        let mut release = Release::parse(LATEST.as_bytes()).unwrap();
        release.tag_name = "0.2.0".to_owned();
        assert_eq!(release.version(), Ok(version("0.2.0")));
        release.tag_name = "latest".to_owned();
        assert!(release.version().is_err());
    }

    #[test]
    fn package_names_match_the_release_workflow() {
        let v = version("1.2.3");
        assert_eq!(
            Platform::WindowsX64.package_name(&v),
            "StealSeries-1.2.3-windows-x64-setup.exe"
        );
        assert_eq!(
            Platform::MacosArm64.package_name(&v),
            "StealSeries-1.2.3-macos-arm64.zip"
        );
    }
}
