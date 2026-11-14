//! Settings page: start-up, hotkeys, files, about.

use eframe::egui;

use crate::core::{AppCore, Snapshot};

pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    ui.heading("General");
    let mut autostart = snapshot.config.app.autostart;
    if ui
        .checkbox(&mut autostart, "Start StealSeries at login")
        .changed()
    {
        core.update_config(|c| c.app.autostart = autostart);
    }

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
