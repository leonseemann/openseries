//! Installing a downloaded, verified update on macOS (ADR 0006 §4): unpack the zipped
//! `StealSeries.app`, then a detached shell helper waits for this process to quit, swaps the
//! bundle in place and opens the new one.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Name of the bundle inside the release zip.
const BUNDLE_NAME: &str = "StealSeries.app";

/// Helper run by `/bin/sh` after the app quits. `$1` pid, `$2` installed bundle, `$3` new bundle.
/// Keeps the old bundle until the new one is in place and puts it back if the move fails.
const SWAP_SCRIPT: &str = r#"
while kill -0 "$1" 2>/dev/null; do sleep 0.2; done
rm -rf "$2.old"
if mv "$2" "$2.old"; then
    if mv "$3" "$2"; then
        rm -rf "$2.old"
    else
        mv "$2.old" "$2"
    fi
fi
open "$2"
"#;

/// The `.app` bundle this process runs from.
///
/// # Errors
/// A user-facing reason if the app does not run from a bundle (e.g. `cargo run`).
pub fn update_install_target() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("locating the app: {e}"))?;
    // …/StealSeries.app/Contents/MacOS/stealseries
    exe.ancestors()
        .nth(3)
        .filter(|bundle| bundle.extension().is_some_and(|e| e == "app"))
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            "StealSeries does not run from an app bundle; install the update from the release \
             page instead"
                .to_owned()
        })
}

/// Unpacks `package` (the verified zip) and starts the helper that swaps the bundle once this
/// process has quit. The caller quits right after.
///
/// # Errors
/// A user-facing message if unpacking fails or the zip has no app.
pub fn install_update(package: &Path) -> Result<(), String> {
    let installed = update_install_target()?;
    let extract = package.with_extension("extracted");
    let _ = std::fs::remove_dir_all(&extract);
    let status = Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(package)
        .arg(&extract)
        .status()
        .map_err(|e| format!("running ditto: {e}"))?;
    if !status.success() {
        return Err("the update could not be unpacked".to_owned());
    }
    let new_bundle = extract.join(BUNDLE_NAME);
    if !new_bundle.is_dir() {
        return Err(format!("the update contains no {BUNDLE_NAME}"));
    }
    // Downloads made by the app itself carry no quarantine flag; clear it anyway in case the
    // zip carried one inside.
    let _ = Command::new("/usr/bin/xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(&new_bundle)
        .status();
    Command::new("/bin/sh")
        .args(["-c", SWAP_SCRIPT, "stealseries-update"])
        .arg(std::process::id().to_string())
        .arg(&installed)
        .arg(&new_bundle)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("starting the update helper: {e}"))
}
