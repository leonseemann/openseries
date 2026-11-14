//! Settings page: start-up, hotkeys, files, about.

use eframe::egui;

use crate::core::{AppCore, Snapshot};
use crate::platform;

pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    ui.heading("General");
    let mut autostart = snapshot.config.app.autostart;
    if ui
        .checkbox(&mut autostart, "Start StealSeries at login")
        .changed()
    {
        core.update_config(|c| c.app.autostart = autostart);
    }

    crackling(ui, core, snapshot);

    ui.add_space(8.0);
    ui.heading("Hotkeys");
    let hotkeys = &snapshot.config.app.hotkeys;
    egui::Grid::new("hotkeys").num_columns(2).show(ui, |ui| {
        for (label, value) in [
            ("ChatMix towards Game", &hotkeys.chatmix_game),
            ("ChatMix towards Chat", &hotkeys.chatmix_chat),
            ("Mute virtual microphone", &hotkeys.mic_mute),
        ] {
            ui.label(label);
            ui.monospace(value.as_deref().unwrap_or("—"));
            ui.end_row();
        }
    });

    ui.add_space(8.0);
    ui.heading("About");
    ui.label(format!("StealSeries {}", env!("CARGO_PKG_VERSION")));
    ui.label(format!("Settings file: {}", core.config_path().display()));
    ui.label(
        egui::RichText::new(
            "Independent open-source software (MIT OR Apache-2.0); not affiliated with SteelSeries.",
        )
        .weak(),
    );
    ui.add_space(8.0);
    if ui.button("Quit StealSeries").clicked() {
        crate::quit(core);
    }
}

fn crackling(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    ui.add_space(8.0);
    ui.heading("Crackling");
    let mut enabled = snapshot.config.app.auto_fix_crackling;
    let response = ui.add_enabled(
        platform::UNDERFLOW_WATCH_SUPPORTED,
        egui::Checkbox::new(&mut enabled, "Fix crackling automatically"),
    );
    if response.changed() {
        core.update_config(|c| c.app.auto_fix_crackling = enabled);
    }
    ui.label(
        egui::RichText::new(if platform::UNDERFLOW_WATCH_SUPPORTED {
            "When the system reports bursts of USB underflows from the base station (heard as \
             crackling), StealSeries reconnects its USB connection — about one second of silence \
             instead of ongoing crackling."
        } else {
            "Not available on this operating system yet."
        })
        .weak(),
    );
    let status = &snapshot.crackle;
    if let Some(error) = &status.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    } else if status.active {
        let last = status
            .last_fix
            .and_then(|t| t.elapsed().ok())
            .map_or_else(String::new, |ago| format!(", last {} s ago", ago.as_secs()));
        ui.label(format!("Watching · fixed {} time(s){last}", status.fixes));
    }
}
