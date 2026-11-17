//! Installing the virtual-mic HAL driver from the app (onboarding; `app.md` §2).
//!
//! **Untested on hardware**: written on Windows and not compile-checked for macOS (the app crate
//! needs hidapi's C build). Mirrors `scripts/macos/hal-driver.sh install`, which is verified
//! (`docs/hardware/macos-virtual-mic.md`).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Bundle name of the driver (`scripts/macos/hal-driver.sh`).
const DRIVER_BUNDLE: &str = "OpenSeriesMic.driver";
/// Where CoreAudio loads HAL plug-ins from.
const HAL_DIR: &str = "/Library/Audio/Plug-Ins/HAL";

/// Whether the driver bundle is installed.
#[must_use]
#[allow(clippy::unnecessary_wraps, reason = "same signature on every platform")]
pub fn mic_driver_installed() -> Option<bool> {
    Some(Path::new(HAL_DIR).join(DRIVER_BUNDLE).exists())
}

/// The driver bundled in `OpenSeries.app/Contents/Resources` (`scripts/macos/app.sh`).
fn bundled_driver() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let path = exe
        .parent()? // Contents/MacOS
        .parent()? // Contents
        .join("Resources")
        .join(DRIVER_BUNDLE);
    path.exists().then_some(path)
}

/// Copies the bundled driver into the HAL folder and restarts `coreaudiod` (all audio pauses for
/// about a second). macOS asks for an administrator password.
///
/// # Errors
/// A user-facing message (not bundled, cancelled, copy failed).
pub fn install_mic_driver() -> Result<(), String> {
    let source = bundled_driver().ok_or_else(|| {
        "the driver is not inside this app build; run scripts/macos/hal-driver.sh install"
            .to_owned()
    })?;
    let source = applescript_string(&source.to_string_lossy());
    let target = applescript_string(&format!("{HAL_DIR}/{DRIVER_BUNDLE}"));
    let hal_dir = applescript_string(HAL_DIR);
    let script = format!(
        "do shell script \"rm -rf \" & quoted form of {target} & \" && cp -R \" \
         & quoted form of {source} & \" \" & quoted form of {hal_dir} & \" && \
         killall coreaudiod\" with administrator privileges"
    );
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| format!("running osascript: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if message.contains("-128") {
            crate::i18n::tr!("platform-driver-install-cancelled")
        } else {
            message
        })
    }
}

/// An `AppleScript` string literal (quotes and backslashes escaped).
fn applescript_string(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::applescript_string;

    #[test]
    fn applescript_strings_are_escaped() {
        assert_eq!(applescript_string("/a b/c"), "\"/a b/c\"");
        assert_eq!(applescript_string("x\"y\\z"), "\"x\\\"y\\\\z\"");
    }
}
