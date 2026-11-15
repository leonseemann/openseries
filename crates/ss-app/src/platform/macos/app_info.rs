//! Executable path and app category of a process on macOS, for automatic channel assignment
//! (`docs/architecture/app.md` §3.2). **Untested on hardware** (written on Windows); the tools
//! used are part of every macOS install.
//!
//! * path: `ps -o comm= -p <pid>` (the full executable path);
//! * category: `LSApplicationCategoryType` from the enclosing `.app` bundle's `Info.plist`, read
//!   with `/usr/libexec/PlistBuddy` (handles binary and XML plists).
//!
//! Results are cached per pid, so each process costs two short tool runs once.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Info.plist key of the App Store / Finder category.
const CATEGORY_KEY: &str = "LSApplicationCategoryType";

/// Cache of path and category per process id.
#[derive(Debug, Default)]
pub struct AppInfoCache {
    entries: HashMap<i32, (Option<String>, Option<String>)>,
}

impl AppInfoCache {
    /// Path and category of `pid` (looked up once, then cached).
    pub fn lookup(&mut self, pid: i32) -> (Option<String>, Option<String>) {
        if let Some(found) = self.entries.get(&pid) {
            return found.clone();
        }
        let path = executable_path(pid);
        let category = path.as_deref().and_then(category_of);
        let found = (path, category);
        self.entries.insert(pid, found.clone());
        found
    }

    /// Forgets processes that are gone (pids are reused).
    pub fn retain(&mut self, alive: &[i32]) {
        self.entries.retain(|pid, _| alive.contains(pid));
    }
}

fn executable_path(pid: i32) -> Option<String> {
    let output = Command::new("/bin/ps")
        .args(["-o", "comm=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !path.is_empty()).then_some(path)
}

/// The category of the `.app` bundle that contains `executable`.
fn category_of(executable: &str) -> Option<String> {
    let bundle = Path::new(executable)
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))?;
    let plist = bundle.join("Contents/Info.plist");
    let output = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", &format!("Print :{CATEGORY_KEY}")])
        .arg(&plist)
        .output()
        .ok()?;
    let category = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !category.is_empty()).then_some(category)
}
