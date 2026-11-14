//! Headset page: the settings the Nova Pro supports (`ss-nova-pro` commands), grouped in cards
//! (`docs/architecture/ui-design.md` §4).

use eframe::egui::{self, RichText};
use ss_core::battery::ChargeState;
use ss_core::headset::{
    AutoOff, DisplayBrightness, EqGain, EqPreset, HardwareEq, MicLedBrightness, MicVolume,
    NoiseControl, SidetoneLevel, TransparencyLevel,
};
use ss_nova_pro::{Command, Settings, Status};
use ss_store::presets::PresetData;

use super::components::{self, Tone};
use super::{presets, theme};
use crate::core::{AppCore, Snapshot};
use crate::headset::HeadsetState;

/// Resolution of the hardware EQ (Observed: 0.5 dB steps read back exactly).
const EQ_STEP_DB: f32 = 0.5;

pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let headset = &snapshot.headset;
    components::page_header(
        ui,
        "Headset",
        "Arctis Nova Pro Wireless — sound, microphone, power and the base station.",
    );
    if let Some(error) = &headset.error {
        components::banner(ui, Tone::Warning, error);
        ui.add_space(8.0);
    }
    let Some(status) = headset.status else {
        not_connected(ui, headset.base_station);
        return;
    };
    if !status.connected {
        components::banner(
            ui,
            Tone::Neutral,
            "The base station is connected, but the headset is off or out of range. Settings \
             stored in the base station can still be changed.",
        );
        ui.add_space(8.0);
    }
    noise_control_card(ui, core, status);
    microphone_card(ui, core, status, headset.settings);
    equalizer_card(ui, core, snapshot, headset.settings);
    power_card(ui, core, status, headset);
    device_card(ui, core, headset);
}

/// Empty state when no base station is available.
fn not_connected(ui: &mut egui::Ui, base_station: bool) {
    components::card(ui, None, |ui| {
        if base_station {
            components::status(ui, Tone::Neutral, "Reading the base station…");
        } else {
            components::status(ui, Tone::Error, "Base station not found");
            let p = theme::palette(ui);
            ui.label(
                RichText::new(
                    "Connect the base station to this computer with the USB cable (on macOS \
                     the USB-1 port). If it is plugged in, it may be switched to the other USB \
                     input — switch it back on the base station.",
                )
                .color(p.text_secondary),
            );
        }
    });
}

/// Noise-control choice, shared with the overview.
pub(super) fn noise_control_choice(ui: &mut egui::Ui, core: &AppCore, status: Status) {
    if let Some(mode) = components::segmented(
        ui,
        status.noise_control,
        &[
            (NoiseControl::NoiseCancelling, "Noise cancelling"),
            (NoiseControl::Transparency, "Transparency"),
            (NoiseControl::Off, "Off"),
        ],
    ) {
        core.headset(Command::SetNoiseControl(mode));
    }
}

/// Hardware EQ preset choice, shared with the overview.
pub(super) fn eq_preset_choice(ui: &mut egui::Ui, core: &AppCore, settings: Option<Settings>) {
    if let Some(preset) = components::segmented(
        ui,
        settings.and_then(|s| s.eq_preset),
        &[
            (EqPreset::Flat, "Flat"),
            (EqPreset::BassBoost, "Bass"),
            (EqPreset::Focus, "Focus"),
            (EqPreset::Smiley, "Smiley"),
            (EqPreset::Custom, "Custom"),
        ],
    ) {
        core.headset(Command::SetEqPreset(preset));
    }
}

fn noise_control_card(ui: &mut egui::Ui, core: &AppCore, status: Status) {
    components::card(ui, Some("Noise control"), |ui| {
        components::row(
            ui,
            "Mode",
            Some("Also switchable with the button on the headset."),
            |ui| noise_control_choice(ui, core, status),
        );
        if status.noise_control == Some(NoiseControl::Transparency) {
            if let Some(level) = status.transparency {
                components::divider(ui);
                components::row(
                    ui,
                    "Transparency strength",
                    Some("How much of your surroundings you hear."),
                    |ui| {
                        if let Some(value) = level_slider(
                            ui,
                            "transparency",
                            level.get(),
                            TransparencyLevel::MIN..=TransparencyLevel::MAX,
                        ) {
                            core.headset(Command::SetTransparency(TransparencyLevel::saturating(
                                value,
                            )));
                        }
                    },
                );
            }
        }
    });
}

fn microphone_card(ui: &mut egui::Ui, core: &AppCore, status: Status, settings: Option<Settings>) {
    components::card(ui, Some("Microphone"), |ui| {
        components::row(
            ui,
            "Status",
            Some("Muted with the button on the headset."),
            |ui| {
                if status.mic_muted {
                    components::status(ui, Tone::Warning, "Muted");
                } else {
                    components::status(ui, Tone::Good, "Live");
                }
            },
        );
        if let Some(volume) = settings.and_then(|s| s.mic_volume) {
            components::divider(ui);
            components::row(ui, "Microphone volume", None, |ui| {
                if let Some(value) = level_slider(
                    ui,
                    "mic-volume",
                    volume.get(),
                    MicVolume::MIN..=MicVolume::MAX,
                ) {
                    core.headset(Command::SetMicVolume(MicVolume::saturating(value)));
                }
            });
        }
        components::divider(ui);
        components::row(
            ui,
            "Sidetone",
            Some("Hear your own voice in the headset while you talk."),
            |ui| {
                if let Some(level) = components::segmented(
                    ui,
                    settings.and_then(|s| s.sidetone),
                    &[
                        (SidetoneLevel::Off, "Off"),
                        (SidetoneLevel::Low, "Low"),
                        (SidetoneLevel::Medium, "Medium"),
                        (SidetoneLevel::High, "High"),
                    ],
                ) {
                    core.headset(Command::SetSidetone(level));
                }
            },
        );
        if let Some(led) = status.mic_led {
            components::divider(ui);
            components::row(
                ui,
                "Mute light brightness",
                Some("The red light on the microphone while it is muted."),
                |ui| {
                    if let Some(value) = level_slider(
                        ui,
                        "mic-led",
                        led.get(),
                        MicLedBrightness::MIN..=MicLedBrightness::MAX,
                    ) {
                        core.headset(Command::SetMicLedBrightness(MicLedBrightness::saturating(
                            value,
                        )));
                    }
                },
            );
        }
    });
}

fn equalizer_card(
    ui: &mut egui::Ui,
    core: &AppCore,
    snapshot: &Snapshot,
    settings: Option<Settings>,
) {
    components::card(ui, Some("Equalizer"), |ui| {
        components::row(
            ui,
            "Preset",
            Some("Stored in the base station; works on every connected device."),
            |ui| eq_preset_choice(ui, core, settings),
        );
        if settings.and_then(|s| s.eq_preset) == Some(EqPreset::Custom) {
            components::divider(ui);
            custom_eq(ui, core, snapshot, settings.and_then(|s| s.custom_eq));
        }
    });
}

/// Editor for the on-device 10-band curve (`06 33`, Observed in `nova-pro-wireless.md`).
///
/// The band centre frequencies are Unknown, so the bands are labelled from low to high.
fn custom_eq(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, curve: Option<HardwareEq>) {
    let p = theme::palette(ui);
    let Some(curve) = curve else {
        ui.label(
            RichText::new("The base station did not report a custom curve.")
                .color(p.text_secondary),
        );
        return;
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new("Custom curve").color(p.text));
        ui.label(
            RichText::new("−10 … +10 dB per band, low to high")
                .small()
                .color(p.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(!curve.is_flat(), egui::Button::new("Reset to flat"))
                .clicked()
            {
                write_custom_eq(core, HardwareEq::FLAT);
            }
        });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let band_width = (ui.available_width() / 10.0).floor();
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.spacing_mut().slider_width = 150.0;
        for (index, gain) in curve.bands.iter().enumerate() {
            ui.allocate_ui_with_layout(
                egui::vec2(band_width, 210.0),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    if let Some(db) = band_slider(ui, index, gain.db()) {
                        if let Ok(gain) = EqGain::from_db(db) {
                            let mut edited = curve;
                            edited.bands[index] = gain;
                            write_custom_eq(core, edited);
                        }
                    }
                    let label = match index {
                        0 => "Low".to_owned(),
                        9 => "High".to_owned(),
                        _ => format!("{}", index + 1),
                    };
                    ui.label(RichText::new(label).small().color(p.text_secondary));
                },
            );
        }
    });
    components::divider(ui);
    let preset = presets::bar(
        ui,
        core,
        snapshot,
        presets::Kind::HeadsetEq,
        "headset-eq",
        || {
            Some(PresetData::HeadsetEq(
                curve.bands.iter().map(|g| g.db()).collect(),
            ))
        },
    );
    if let Some(PresetData::HeadsetEq(bands)) = preset {
        if let Some(curve) = hardware_eq(&bands) {
            write_custom_eq(core, curve);
        }
    }
}

/// Writes the curve and re-selects "Custom", like `ssctl set eq`. Whether the curve applies live
/// without the re-select is Unknown; with it, the change is audible (Observed, see
/// `nova-pro-wireless.md`).
fn write_custom_eq(core: &AppCore, curve: HardwareEq) {
    core.headset(Command::SetCustomEq(curve));
    core.headset(Command::SetEqPreset(EqPreset::Custom));
}

/// A stored curve as a device curve (snapped to the 0.5 dB grid, clamped to the device range).
fn hardware_eq(bands: &[f32]) -> Option<HardwareEq> {
    let mut curve = HardwareEq::FLAT;
    if bands.len() != curve.bands.len() {
        return None;
    }
    for (slot, db) in curve.bands.iter_mut().zip(bands) {
        let snapped =
            ((db / EQ_STEP_DB).round() * EQ_STEP_DB).clamp(EqGain::MIN_DB, EqGain::MAX_DB);
        *slot = EqGain::from_db(snapped).ok()?;
    }
    Some(curve)
}

/// One vertical band slider; returns the new gain once the user lets go.
fn band_slider(ui: &mut egui::Ui, index: usize, value: f32) -> Option<f32> {
    // While dragging, the device still reports the old value; keep the dragged one locally.
    let id = ui.id().with(("eq-band", index));
    let mut edited = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(value);
    let response = ui.add(
        egui::Slider::new(&mut edited, EqGain::MIN_DB..=EqGain::MAX_DB)
            .vertical()
            .step_by(f64::from(EQ_STEP_DB))
            .fixed_decimals(1),
    );
    if response.dragged() {
        ui.data_mut(|d| d.insert_temp(id, edited));
        return None;
    }
    ui.data_mut(|d| d.remove::<f32>(id));
    let finished = response.drag_stopped() || response.changed();
    (finished && (edited - value).abs() >= EQ_STEP_DB / 2.0).then_some(edited)
}

fn power_card(ui: &mut egui::Ui, core: &AppCore, status: Status, headset: &HeadsetState) {
    components::card(ui, Some("Power and display"), |ui| {
        components::row(
            ui,
            "Turn off when idle",
            Some("The headset switches off after this long without sound."),
            |ui| {
                let current = status.auto_off;
                egui::ComboBox::from_id_salt("auto-off")
                    .selected_text(current.map_or("Unknown", auto_off_label))
                    .width(160.0)
                    .show_ui(ui, |ui| {
                        for option in [
                            AutoOff::Never,
                            AutoOff::After1Min,
                            AutoOff::After5Min,
                            AutoOff::After10Min,
                            AutoOff::After15Min,
                            AutoOff::After30Min,
                            AutoOff::After60Min,
                        ] {
                            if ui
                                .selectable_label(current == Some(option), auto_off_label(option))
                                .clicked()
                            {
                                core.headset(Command::SetAutoOff(option));
                            }
                        }
                    });
            },
        );
        if let Some(brightness) = headset.display.and_then(|d| d.brightness) {
            components::divider(ui);
            components::row(
                ui,
                "Base station display",
                Some("Brightness of the screen."),
                |ui| {
                    if let Some(value) = level_slider(
                        ui,
                        "oled",
                        brightness.get(),
                        DisplayBrightness::MIN..=DisplayBrightness::MAX,
                    ) {
                        core.headset(Command::SetDisplayBrightness(
                            DisplayBrightness::saturating(value),
                        ));
                    }
                },
            );
        }
    });
}

fn device_card(ui: &mut egui::Ui, core: &AppCore, headset: &HeadsetState) {
    components::card(ui, Some("Device"), |ui| {
        components::row(ui, "Firmware", None, |ui| {
            ui.monospace(
                headset
                    .firmware
                    .map_or_else(|| "unknown".to_owned(), |f| f.to_string()),
            )
            .on_hover_text(
                "Four component versions as reported by the base station, in the order and \
                 notation SteelSeries GG uses. Which part each number belongs to is not known.",
            );
        });
        if let Some(usb) = headset.usb {
            components::divider(ui);
            components::row(ui, "USB", None, |ui| {
                ui.monospace(format!(
                    "PID {:#06x} · release {}",
                    usb.product_id,
                    bcd_release(usb.release)
                ));
            });
        }
        if crate::platform::USB_RESET_SUPPORTED {
            components::divider(ui);
            components::row(
                ui,
                "Reconnect USB",
                Some("Fixes crackling: like unplugging the cable (audio pauses for a second)."),
                |ui| {
                    if ui.button("Reconnect").clicked() {
                        core.reset_headset_usb();
                    }
                },
            );
        }
    });
}

/// A USB `bcdDevice` value as `major.minor` (0x0130 → "1.30").
fn bcd_release(bcd: u16) -> String {
    format!("{:x}.{:02x}", bcd >> 8, bcd & 0xFF)
}

/// An integer slider that reports only finished changes (release or click), so dragging does
/// not flood the headset with commands.
fn level_slider(
    ui: &mut egui::Ui,
    id: &str,
    value: u8,
    range: std::ops::RangeInclusive<u8>,
) -> Option<u8> {
    // While dragging, the device still reports the old value; keep the dragged one locally.
    let id = ui.id().with(("level", id));
    let mut edited = ui.data(|d| d.get_temp::<u8>(id)).unwrap_or(value);
    let response = ui.add(egui::Slider::new(&mut edited, range));
    if response.dragged() {
        ui.data_mut(|d| d.insert_temp(id, edited));
        return None;
    }
    ui.data_mut(|d| d.remove::<u8>(id));
    let finished = response.drag_stopped() || response.changed();
    (finished && edited != value).then_some(edited)
}

/// Plain-language charging state.
pub(super) const fn charge_text(state: ChargeState) -> &'static str {
    match state {
        ChargeState::Discharging => "On battery",
        ChargeState::Charging => "Charging",
        ChargeState::PluggedNotCharging => "Plugged in",
        ChargeState::Unknown => "Battery state unknown",
    }
}

const fn auto_off_label(option: AutoOff) -> &'static str {
    match option {
        AutoOff::Never => "Never",
        AutoOff::After1Min => "After 1 minute",
        AutoOff::After5Min => "After 5 minutes",
        AutoOff::After10Min => "After 10 minutes",
        AutoOff::After15Min => "After 15 minutes",
        AutoOff::After30Min => "After 30 minutes",
        AutoOff::After60Min => "After 1 hour",
    }
}
