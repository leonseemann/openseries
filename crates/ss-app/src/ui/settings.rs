//! Settings page: start-up, hotkeys, files, about.

use std::sync::OnceLock;

use eframe::egui;

use ss_store::config::HotkeysConfig;

use crate::core::{AppCore, Snapshot};
use crate::hotkeys::{HotkeyAction, binding_from_key, same_binding};
use crate::platform;

pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, capture: &mut HotkeyCapture) {
    ui.heading("General");
    let mut autostart = snapshot.config.app.autostart;
    if ui
        .checkbox(&mut autostart, "Start StealSeries at login")
        .changed()
    {
        core.update_config(|c| c.app.autostart = autostart);
    }

    let mut notifications = snapshot.config.app.notifications.enabled;
    ui.horizontal(|ui| {
        if ui
            .checkbox(&mut notifications, "Desktop notifications")
            .on_hover_text(
                "Headset or base station connected/disconnected, low headset battery, AudioMix or                  virtual microphone errors.",
            )
            .changed()
        {
            core.update_config(|c| c.app.notifications.enabled = notifications);
        }
        let id = ui.id().with("notification-test");
        if ui.small_button("Test").clicked() {
            let result = crate::alerts::show("Test notification")
                .map_or_else(|e| format!("failed: {e}"), |()| "sent".to_owned());
            ui.data_mut(|d| d.insert_temp(id, result));
        }
        if let Some(result) = ui.data(|d| d.get_temp::<String>(id)) {
            ui.label(egui::RichText::new(result).weak());
        }
    });

    if snapshot.config.app.onboarding_done && ui.button("Show getting started").clicked() {
        core.update_config(|c| c.app.onboarding_done = false);
    }

    crackling(ui, core, snapshot);

    ui.add_space(8.0);
    hotkeys(ui, core, snapshot, capture);

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
    third_party_licenses(ui);
    ui.add_space(8.0);
    reset_settings(ui, core);
    ui.add_space(8.0);
    if ui.button("Quit StealSeries").clicked() {
        crate::quit(core);
    }
}

/// State of the hotkey editor: which action is recording, and the last rejection.
#[derive(Debug, Default)]
pub struct HotkeyCapture {
    /// The action waiting for a key combination; global hotkeys are suspended meanwhile.
    pub recording: Option<HotkeyAction>,
    /// Why the last recorded combination was refused.
    pub message: Option<String>,
}

fn hotkeys(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, capture: &mut HotkeyCapture) {
    ui.heading("Hotkeys");
    let config = &snapshot.config.app.hotkeys;
    if let Some(action) = capture.recording {
        record(ui, core, config, action, capture);
    }
    egui::Grid::new("hotkeys").num_columns(4).show(ui, |ui| {
        for action in HotkeyAction::ALL.into_iter().filter(|a| a.available()) {
            ui.label(action.label());
            if capture.recording == Some(action) {
                ui.colored_label(ui.visuals().warn_fg_color, "press keys… (Esc cancels)");
            } else {
                ui.monospace(action.binding(config).unwrap_or("—"));
            }
            if ui.button("Change").clicked() {
                capture.recording = Some(action);
                capture.message = None;
            }
            if ui
                .add_enabled(action.binding(config).is_some(), egui::Button::new("Clear"))
                .clicked()
            {
                core.update_config(|c| *action.binding_mut(&mut c.app.hotkeys) = None);
            }
            ui.end_row();
        }
    });
    if let Some(message) = &capture.message {
        ui.colored_label(ui.visuals().warn_fg_color, message);
    }
    if *config != HotkeysConfig::default() && ui.button("Reset to defaults").clicked() {
        core.update_config(|c| c.app.hotkeys = HotkeysConfig::default());
    }
}

/// Takes the next key press as the binding for `action`.
fn record(
    ui: &egui::Ui,
    core: &AppCore,
    config: &HotkeysConfig,
    action: HotkeyAction,
    capture: &mut HotkeyCapture,
) {
    let pressed = ui.input(|i| {
        i.events.iter().find_map(|event| match event {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => Some((*key, *modifiers)),
            _ => None,
        })
    });
    let Some((key, modifiers)) = pressed else {
        return;
    };
    if key == egui::Key::Escape {
        capture.recording = None;
        return;
    }
    match binding_from_key(key, modifiers) {
        Ok(binding) => {
            let taken = HotkeyAction::ALL.into_iter().find(|other| {
                *other != action
                    && other
                        .binding(config)
                        .is_some_and(|b| same_binding(b, &binding))
            });
            if let Some(other) = taken {
                capture.message = Some(format!(
                    "{binding} is already used by \"{}\"",
                    other.label()
                ));
            } else {
                core.update_config(|c| *action.binding_mut(&mut c.app.hotkeys) = Some(binding));
                capture.message = None;
            }
            capture.recording = None;
        }
        // Keep recording: e.g. a lone modifier or a key without Ctrl/Alt.
        Err(message) => capture.message = Some(message),
    }
}

/// "Reset StealSeries settings…" with a confirmation step (DEV-12, app part). The headset's own
/// factory reset is not offered: its command is unverified (`nova-pro-wireless.md`).
fn reset_settings(ui: &mut egui::Ui, core: &AppCore) {
    let id = ui.id().with("confirm-reset");
    let confirming = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    if !confirming {
        if ui.button("Reset StealSeries settings…").clicked() {
            ui.data_mut(|d| d.insert_temp(id, true));
        }
        return;
    }
    ui.group(|ui| {
        ui.label(
            "Reset AudioMix, microphone, hotkeys, notifications and all other StealSeries \
             settings to their defaults? AudioMix and the virtual microphone switch off, start at \
             login is turned off. Your saved presets are kept. The headset itself is not changed.",
        );
        ui.horizontal(|ui| {
            if ui.button("Reset").clicked() {
                core.update_config(|c| *c = c.reset_keeping_presets());
                ui.data_mut(|d| d.insert_temp(id, false));
            }
            if ui.button("Cancel").clicked() {
                ui.data_mut(|d| d.insert_temp(id, false));
            }
        });
    });
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

/// Licences of all bundled open-source crates (generated by `scripts/third_party_licenses.py`;
/// MIT/BSD/Apache require shipping these texts with the app).
const THIRD_PARTY_LICENSES: &str = include_str!("../../THIRD_PARTY_LICENSES.md");
/// Height of the licence viewer in points.
const LICENSE_VIEW_HEIGHT: f32 = 260.0;

fn third_party_licenses(ui: &mut egui::Ui) {
    static LINES: OnceLock<Vec<&'static str>> = OnceLock::new();
    let lines = LINES.get_or_init(|| THIRD_PARTY_LICENSES.lines().collect());
    egui::CollapsingHeader::new("Third-party licences").show(ui, |ui| {
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        egui::ScrollArea::vertical()
            .max_height(LICENSE_VIEW_HEIGHT)
            .auto_shrink([false, true])
            // Only the visible rows are laid out; the text is ~700 KiB.
            .show_rows(ui, row_height, lines.len(), |ui, rows| {
                for line in &lines[rows] {
                    ui.monospace(*line);
                }
            });
    });
}
