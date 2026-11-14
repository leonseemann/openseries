//! Watches macOS's USB audio daemon for underflow reports (the crackling signature,
//! `docs/hardware/macos-app.md` Findings).
//!
//! Runs `/usr/bin/log stream` filtered to `usbaudiod` underflow messages (no admin rights needed)
//! and calls back once per reported underflow. Depends on Apple's message text
//! (**Observed** on macOS 27.2: `AUAInputTransferManager completeBlock USB underflow -64`); if it
//! changes, the watch simply reports nothing.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::thread::JoinHandle;

/// The unified-log tool (in zsh `log` is a shell builtin, so always use the full path).
const LOG_TOOL: &str = "/usr/bin/log";
/// Text that marks one underflow report.
const UNDERFLOW_MARKER: &str = "USB underflow";
/// Server-side filter so only relevant lines reach us.
const PREDICATE: &str = r#"process == "usbaudiod" AND eventMessage CONTAINS "USB underflow""#;

/// A running watch; dropping it stops the log stream.
pub struct UnderflowWatch {
    child: Child,
    reader: Option<JoinHandle<()>>,
}

/// Starts watching; `on_underflow` runs on a background thread for every report.
///
/// # Errors
/// A user-facing message if the log tool cannot be started.
pub fn watch_usb_underflows(
    on_underflow: impl Fn() + Send + 'static,
) -> Result<UnderflowWatch, String> {
    let mut child = Command::new(LOG_TOOL)
        .args(["stream", "--style", "compact", "--predicate", PREDICATE])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("starting {LOG_TOOL}: {e}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "log stream has no output".to_owned())?;
    let reader = std::thread::Builder::new()
        .name("usb-underflow-watch".to_owned())
        .spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if line.contains(UNDERFLOW_MARKER) {
                    on_underflow();
                }
            }
        })
        .map_err(|e| format!("starting the watch thread: {e}"))?;
    Ok(UnderflowWatch {
        child,
        reader: Some(reader),
    })
}

impl Drop for UnderflowWatch {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join(); // ends once the child's pipe closes
        }
    }
}
