//! Overview page: every connected device at a glance and the controls people use daily
//! (`docs/architecture/ui-design.md` §4, ADR 0008).

use eframe::egui::{self, RichText};
use ss_device::FeatureKind;

use super::components::{self, Tone};
use super::{Page, audiomix, device, theme};
use crate::core::{AppCore, Snapshot};
use crate::devices::DeviceSnapshot;

/// Width of the battery gauges.
const BATTERY_WIDTH: f32 = 180.0;

pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, page: &mut Page) {
    components::page_header(ui, "Overview", "Your devices at a glance.");
    if snapshot.devices.devices.is_empty() {
        device::not_connected(ui);
    }
    for (index, d) in snapshot.devices.devices.iter().enumerate() {
        device_card(ui, d, index, page);
    }
    quick_controls(ui, core, snapshot, page);
    balance_card(ui, core, snapshot);
}

fn device_card(ui: &mut egui::Ui, d: &DeviceSnapshot, index: usize, page: &mut Page) {
    components::card(ui, None, |ui| {
        let p = theme::palette(ui);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(device::class_icon(d.info.class))
                    .size(34.0)
                    .color(p.accent),
            );
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(d.info.name)
                        .text_style(egui::TextStyle::Name(theme::CARD_TITLE.into()))
                        .color(p.text),
                );
                let charge = d.batteries().map(|(_, charge)| device::charge_text(charge));
                match d.linked() {
                    _ if !d.ready => {
                        components::status(ui, Tone::Neutral, &format!("Reading the {}…", d.hub()));
                    }
                    Some(false) => components::status(
                        ui,
                        Tone::Neutral,
                        &format!(
                            "{} connected · {} off or out of range",
                            capitalised(d.hub()),
                            device::class_word(d.info.class).to_lowercase()
                        ),
                    ),
                    _ => components::status(
                        ui,
                        Tone::Good,
                        &charge
                            .map_or_else(|| "Connected".to_owned(), |c| format!("Connected · {c}")),
                    ),
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Settings").clicked() {
                    *page = Page::Device(index);
                }
            });
        });
        if d.has(FeatureKind::Battery) && d.ready {
            components::divider(ui);
            components::row(ui, "Battery", None, |ui| {
                components::battery(
                    ui,
                    d.main_battery()
                        .map(ss_core::battery::BatteryLevel::percent),
                    BATTERY_WIDTH,
                );
            });
        }
    });
}

fn quick_controls(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, page: &mut Page) {
    components::card(ui, Some("Quick controls"), |ui| {
        // The everyday device settings, from the first device that has each of them.
        for (kind, label, description) in [
            (FeatureKind::NoiseControl, "Noise control", None),
            (FeatureKind::EqPreset, "Equalizer", Some("Device preset.")),
        ] {
            let Some(d) = snapshot.devices.with(kind).filter(|d| d.ready) else {
                continue;
            };
            let Some(feature) = d.feature(kind) else {
                continue;
            };
            components::row(ui, label, description, |ui| {
                device::control(ui, core, d, feature);
            });
            components::divider(ui);
        }
        let mut enabled = snapshot.config.audiomix.enabled;
        components::row(
            ui,
            "AudioMix",
            Some("Sort apps into Game, Chat, Media and Aux and balance Game against Chat."),
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

fn balance_card(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    components::card(ui, Some("Game/Chat balance"), |ui| {
        if snapshot.config.audiomix.enabled {
            audiomix::chatmix_control(ui, core, snapshot);
        } else {
            let p = theme::palette(ui);
            let dial = if snapshot.devices.with(FeatureKind::BalanceDial).is_some() {
                ", the hotkeys or the dial on your device"
            } else {
                " or the hotkeys"
            };
            ui.label(
                RichText::new(format!(
                    "Balances game and chat audio — with this slider{dial}. Turn on AudioMix to use it."
                ))
                .color(p.text_secondary),
            );
        }
    });
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}
