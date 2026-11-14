//! `stealseries` — the StealSeries menu-bar app (`docs/architecture/app.md`).
//!
//! Composition root: loads the config, starts the services and the UI.
//!
//! * `--headless` prints the state to the terminal instead of opening the window.
//! * `STEALSERIES_NO_HEADSET=1` skips the headset thread (development runs that must not talk to
//!   the base station).

mod audio;
#[cfg(target_os = "macos")]
mod autostart;
mod core;
mod headset;
mod hotkeys;
mod notify;
mod tray;
mod ui;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use eframe::egui;
use ss_store::Store;

use crate::core::AppCore;
use crate::notify::Notifier;

/// How often the headless mode prints a status line.
const STATUS_INTERVAL: Duration = Duration::from_secs(2);
/// Environment switch that disables all headset I/O.
const NO_HEADSET_ENV: &str = "STEALSERIES_NO_HEADSET";

fn main() -> Result<()> {
    let headless = std::env::args().any(|a| a == "--headless");
    let with_headset = std::env::var_os(NO_HEADSET_ENV).is_none();
    let store = Store::per_user().context("locating the config directory")?;

    let egui_ctx: Arc<OnceLock<egui::Context>> = Arc::new(OnceLock::new());
    let repaint_ctx = Arc::clone(&egui_ctx);
    let notify: Notifier = Arc::new(move || {
        if let Some(ctx) = repaint_ctx.get() {
            ctx.request_repaint();
        }
    });
    let core =
        AppCore::start(store, &notify, with_headset, backend).context("starting services")?;
    if let Some(warning) = core.snapshot().config_warning {
        eprintln!("warning: {warning}");
    }

    if headless {
        run_headless(&core)?;
        core.shutdown();
        return Ok(());
    }

    let ui_core = Arc::clone(&core);
    eframe::run_native(
        "StealSeries",
        ui::App::options(),
        Box::new(move |cc| {
            let _ = egui_ctx.set(cc.egui_ctx.clone());
            Ok(Box::new(ui::App::new(cc, ui_core)))
        }),
    )
    .map_err(|e| anyhow!("window: {e}"))?;
    core.shutdown();
    Ok(())
}

/// Stops the services (restoring the base station, saving the config) and exits the process.
pub fn quit(core: &AppCore) -> ! {
    core.shutdown();
    std::process::exit(0);
}

#[cfg(target_os = "macos")]
fn backend() -> audio::macos::MacAudioBackend {
    audio::macos::MacAudioBackend
}

/// Prints the state until Ctrl-C.
fn run_headless(core: &Arc<AppCore>) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let flag = Arc::clone(&running);
    ctrlc::set_handler(move || flag.store(false, Ordering::SeqCst))?;
    println!("config: {}", core.config_path().display());
    let mut last = String::new();
    while running.load(Ordering::SeqCst) {
        let snapshot = core.snapshot();
        let headset = &snapshot.headset;
        let line = format!(
            "audiomix {:?} ({} apps routed) | mic {:?} | headset base={} link={} battery={:?} chatmix={:?}{}",
            snapshot.audio.mix,
            snapshot.audio.routed_apps,
            snapshot.audio.mic,
            headset.base_station,
            headset.status.is_some_and(|s| s.connected),
            headset.status.and_then(|s| s.headset_battery),
            headset.chatmix.map(|c| (c.game.get(), c.chat.get())),
            headset
                .error
                .as_ref()
                .map_or(String::new(), |e| format!(" | {e}")),
        );
        if line != last {
            println!("{line}");
            last = line;
        }
        std::thread::sleep(STATUS_INTERVAL);
    }
    Ok(())
}
