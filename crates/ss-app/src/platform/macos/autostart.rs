//! Start at login via a per-user `LaunchAgent` (`docs/architecture/app.md` §2, ADR 0005).
//!
//! Writing or removing `~/Library/LaunchAgents/dev.stealseries.app.plist` takes effect at the
//! next login; nothing is loaded into launchd immediately.

use std::path::PathBuf;

/// launchd label (also the plist file name).
const LABEL: &str = "dev.stealseries.app";

/// Enables or disables starting the current executable at login.
///
/// # Errors
/// A user-facing message on file system errors.
pub fn set(enabled: bool) -> Result<(), String> {
    let path = plist_path()?;
    if !enabled {
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("removing {}: {error}", path.display())),
        };
    }
    let executable = std::env::current_exe().map_err(|e| format!("locating the app: {e}"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    }
    std::fs::write(&path, plist(&executable.to_string_lossy()))
        .map_err(|e| format!("writing {}: {e}", path.display()))
}

fn plist_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("no home directory")?;
    Ok(PathBuf::from(home)
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist")))
}

fn plist(executable: &str) -> String {
    let executable = xml_escape(executable);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{LABEL}</string>
	<key>ProgramArguments</key>
	<array>
		<string>{executable}</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>ProcessType</key>
	<string>Interactive</string>
</dict>
</plist>
"#
    )
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_runs_the_executable_at_load() {
        let text = plist("/Applications/Steal & Series.app/Contents/MacOS/stealseries");
        assert!(text.contains("<string>dev.stealseries.app</string>"));
        assert!(text.contains("Steal &amp; Series.app"));
        assert!(text.contains("<key>RunAtLoad</key>\n\t<true/>"));
    }
}
