//! Overview page: headset state and the controls people use daily
//! (`docs/architecture/ui-design.md` §4).

use eframe::egui::{self, RichText};

use super::components::{self, Tone};
use super::{Page, audiomix, headset, theme};
use crate::core::{AppCore, Snapshot};

/// Width of the battery gauges.
const BATTERY_WIDTH: f32 = 180.0;

pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, page: &mut Page) {
    components::page_header(ui, "Overview", "Your headset at a glance.");
    headset_card(ui, snapshot, page);
    quick_controls(ui, core, snapshot, page);
    chatmix_card(ui, core, snapshot);
}

fn headset_card(ui: &mut egui::Ui, snapshot: &Snapshot, page: &mut Page) {
    let headset = &snapshot.headset;
    components::card(ui, None, |ui| {
        let p = theme::palette(ui);
        ui.horizontal(|ui| {
            ui.label(RichText::new("🎧").size(34.0).color(p.accent));
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Arctis Nova Pro Wireless")
                        .text_style(egui::TextStyle::Name(theme::CARD_TITLE.into()))
                        .color(p.text),
                );
                match headset.status {
                    Some(status) if status.connected => components::status(
                        ui,
                        Tone::Good,
                        &format!("Connected · {}", headset::charge_text(status.charge_state)),
                    ),
                    Some(_) => components::status(
                        ui,
                        Tone::Neutral,
                        "Base station connected · headset off or out of range",
                    ),
                    None if headset.base_station => {
                        components::status(ui, Tone::Neutral, "Reading the base station…");
                    }
                    None => components::status(ui, Tone::Error, "Base station not found"),
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Headset settings").clicked() {
                    *page = Page::Headset;
                }
            });
        });
        let Some(status) = headset.status else {
            return;
        };
        components::divider(ui);
        components::row(ui, "Headset battery", None, |ui| {
            components::battery(
                ui,
                status
                    .connected
                    .then_some(status.headset_battery)
                    .flatten()
                    .map(ss_core::battery::BatteryLevel::percent),
                BATTERY_WIDTH,
            );
        });
        components::row(
            ui,
            "Spare battery",
            Some("The battery charging in the base station."),
            |ui| {
                components::battery(
                    ui,
                    status
                        .spare_battery
                        .map(ss_core::battery::BatteryLevel::percent),
                    BATTERY_WIDTH,
                );
            },
        );
    });
}

fn quick_controls(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, page: &mut Page) {
    let headset = &snapshot.headset;
    components::card(ui, Some("Quick controls"), |ui| {
        if let Some(status) = headset.status {
            components::row(ui, "Noise control", None, |ui| {
                headset::noise_control_choice(ui, core, status);
            });
            components::divider(ui);
            components::row(ui, "Equalizer", Some("Headset preset."), |ui| {
                headset::eq_preset_choice(ui, core, headset.settings);
            });
            components::divider(ui);
        }
        let mut enabled = snapshot.config.audiomix.enabled;
        components::row(
            ui,
            "AudioMix",
            Some("Sort apps into Game, Chat, Media and Aux and balance them with ChatMix."),
            |ui| {
                if components::switch(ui, &mut enabled) {
                    core.update_config(|c| c.audiomix.enabled = enabled);
                }
                let (tone, word) = components::service_tone(&snapshot.audio.mix);
                components::status(ui, tone, word);
                if ui.small_button("Manage").clicked() {
                    *page = Page::AudioMix;
                }
            },
        );
        components::service_problem(ui, &snapshot.audio.mix);
    });
}

fn chatmix_card(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    components::card(ui, Some("ChatMix"), |ui| {
        if snapshot.config.audiomix.enabled {
            audiomix::chatmix_control(ui, core, snapshot);
        } else {
            let p = theme::palette(ui);
            ui.label(
                RichText::new(
                    "Balances game and chat audio — with this slider, the hotkeys or the dial on \
                     the base station. Turn on AudioMix to use it.",
                )
                .color(p.text_secondary),
            );
        }
    });
}
