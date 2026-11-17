//! Device pages, built from the device's features (ADR 0008, `docs/architecture/devices.md` §3).
//!
//! A presentation table gives each feature kind its card, label, description and visibility rule;
//! the control is chosen by the feature's shape (level, choice, toggle, curve …). Kinds the table
//! does not know appear in a "More" card, so a new device or feature never needs a new page.

use eframe::egui::{self, RichText};
use ss_core::battery::ChargeState;
use ss_device::{BatterySlot, Descriptor, DeviceClass, Feature, FeatureKind, Value};
use ss_store::presets::PresetData;

use super::components::{self, Tone};
use super::{presets, theme};
use crate::core::{AppCore, Snapshot};
use crate::devices::DeviceSnapshot;

/// Width of the battery gauges.
const BATTERY_WIDTH: f32 = 180.0;
/// Choices with up to this many options are segmented buttons, more become a drop-down.
const MAX_SEGMENTS: usize = 5;
/// Height of the EQ band sliders.
const EQ_SLIDER_HEIGHT: f32 = 150.0;

/// The cards of a device page, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Group {
    NoiseControl,
    Microphone,
    Equalizer,
    Power,
    Device,
    More,
}

impl Group {
    const ALL: [Self; 6] = [
        Self::NoiseControl,
        Self::Microphone,
        Self::Equalizer,
        Self::Power,
        Self::Device,
        Self::More,
    ];

    const fn title(self) -> &'static str {
        match self {
            Self::NoiseControl => "Noise control",
            Self::Microphone => "Microphone",
            Self::Equalizer => "Equalizer",
            Self::Power => "Power and display",
            Self::Device => "Device",
            Self::More => "More",
        }
    }
}

/// How one feature kind appears on the device page.
struct Presentation {
    group: Group,
    label: &'static str,
    description: Option<&'static str>,
    /// Only shown while another feature has this choice (e.g. transparency strength).
    shown_when: Option<(FeatureKind, &'static str)>,
}

/// The presentation of `kind`; `None` = shown elsewhere (status card, AudioMix) or not at all.
const fn presentation(kind: FeatureKind) -> Option<Presentation> {
    const fn p(
        group: Group,
        label: &'static str,
        description: Option<&'static str>,
    ) -> Presentation {
        Presentation {
            group,
            label,
            description,
            shown_when: None,
        }
    }
    Some(match kind {
        FeatureKind::Battery
        | FeatureKind::WirelessLink
        | FeatureKind::AudioEndpoint
        | FeatureKind::BalanceDial => return None,
        FeatureKind::NoiseControl => p(
            Group::NoiseControl,
            "Mode",
            Some("Also switchable with the button on the device."),
        ),
        FeatureKind::Transparency => Presentation {
            shown_when: Some((FeatureKind::NoiseControl, "transparency")),
            ..p(
                Group::NoiseControl,
                "Transparency strength",
                Some("How much of your surroundings you hear."),
            )
        },
        FeatureKind::MicMute => p(
            Group::Microphone,
            "Status",
            Some("Muted with the button on the device."),
        ),
        FeatureKind::MicVolume => p(Group::Microphone, "Microphone volume", None),
        FeatureKind::Sidetone => p(
            Group::Microphone,
            "Sidetone",
            Some("Hear your own voice while you talk."),
        ),
        FeatureKind::MicLedBrightness => p(
            Group::Microphone,
            "Mute light brightness",
            Some("The light on the microphone while it is muted."),
        ),
        FeatureKind::EqPreset => p(
            Group::Equalizer,
            "Preset",
            Some("Stored on the device; works on everything it plays."),
        ),
        FeatureKind::Equalizer => Presentation {
            shown_when: Some((FeatureKind::EqPreset, "custom")),
            ..p(Group::Equalizer, "Custom curve", None)
        },
        FeatureKind::AutoOff => p(
            Group::Power,
            "Turn off when idle",
            Some("Switches off after this long without sound."),
        ),
        FeatureKind::DisplayBrightness => p(Group::Power, "Display brightness", None),
        FeatureKind::Firmware => p(Group::Device, "Firmware", None),
        FeatureKind::UsbReplug => p(
            Group::Device,
            "Reconnect USB",
            Some("Fixes crackling: like unplugging the cable (audio pauses for a second)."),
        ),
        _ => p(Group::More, "Setting", None),
    })
}

/// The page of the connected device at `index` (`None` if it went away).
pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, index: usize) {
    let Some(device) = snapshot.devices.devices.get(index) else {
        components::page_header(ui, "Device", "The device is no longer connected.");
        not_connected(ui);
        return;
    };
    components::page_header(
        ui,
        device.info.name,
        &format!(
            "{} — settings stored on the device.",
            class_word(device.info.class)
        ),
    );
    if device.info.support == ss_device::Support::Experimental {
        components::banner(
            ui,
            Tone::Warning,
            "Experimental support: this device was not tested on hardware by the OpenSeries \
             team. Settings may not work as described.",
        );
        ui.add_space(8.0);
    }
    if !device.ready {
        components::card(ui, None, |ui| {
            components::status(ui, Tone::Neutral, &format!("Reading the {}…", device.hub()));
        });
        return;
    }
    if device.linked() == Some(false) {
        components::banner(
            ui,
            Tone::Neutral,
            &format!(
                "The {} is connected, but the {} is off or out of range. Settings stored in the {} \
                 can still be changed.",
                device.hub(),
                class_word(device.info.class).to_lowercase(),
                device.hub()
            ),
        );
        ui.add_space(8.0);
    }
    status_card(ui, device);
    for group in Group::ALL {
        let features: Vec<(&Feature, Presentation)> = device
            .features
            .iter()
            .filter_map(|f| presentation(f.id.kind).map(|p| (f, p)))
            .filter(|(_, p)| p.group == group && visible(device, p))
            .collect();
        let extra_device_rows = group == Group::Device;
        if features.is_empty() && !extra_device_rows {
            continue;
        }
        components::card(ui, Some(group.title()), |ui| {
            let mut first = true;
            for (feature, presentation) in &features {
                if !first {
                    components::divider(ui);
                }
                first = false;
                feature_row(ui, core, snapshot, device, feature, presentation);
            }
            if extra_device_rows {
                identity_rows(ui, device, first);
            }
        });
    }
}

fn visible(device: &DeviceSnapshot, presentation: &Presentation) -> bool {
    presentation.shown_when.is_none_or(|(kind, key)| {
        device.feature(kind).is_none() || device.choice(kind) == Some(key)
    })
}

/// Battery and charging (only for devices that report a battery).
fn status_card(ui: &mut egui::Ui, device: &DeviceSnapshot) {
    let Some(Descriptor::Battery { slots }) =
        device.feature(FeatureKind::Battery).map(|f| &f.descriptor)
    else {
        return;
    };
    let batteries = device.batteries();
    components::card(ui, Some("Battery"), |ui| {
        for (index, slot) in slots.iter().enumerate() {
            if index > 0 {
                components::divider(ui);
            }
            let level = match slot {
                BatterySlot::Main => device.main_battery(),
                BatterySlot::Spare => {
                    batteries.and_then(|(levels, _)| levels.get(index).copied().flatten())
                }
            };
            let (label, description) = match slot {
                BatterySlot::Main => ("Battery", batteries.map(|(_, charge)| charge_text(charge))),
                BatterySlot::Spare => (
                    "Spare battery",
                    Some("The battery charging in the base station."),
                ),
            };
            components::row(ui, label, description, |ui| {
                components::battery(
                    ui,
                    level.map(ss_core::battery::BatteryLevel::percent),
                    BATTERY_WIDTH,
                );
            });
        }
    });
}

/// One feature as a settings row.
fn feature_row(
    ui: &mut egui::Ui,
    core: &AppCore,
    snapshot: &Snapshot,
    device: &DeviceSnapshot,
    feature: &Feature,
    presentation: &Presentation,
) {
    let label = if presentation.group == Group::More {
        format!("{:?}", feature.id.kind)
    } else {
        presentation.label.to_owned()
    };
    if let Descriptor::Equalizer { .. } = feature.descriptor {
        curve_editor(ui, core, snapshot, device, feature);
        return;
    }
    components::row(ui, &label, presentation.description, |ui| {
        control(ui, core, device, feature);
    });
}

/// The control for one feature, chosen by its shape. Also used by the Overview page.
pub(super) fn control(
    ui: &mut egui::Ui,
    core: &AppCore,
    device: &DeviceSnapshot,
    feature: &Feature,
) {
    let id = feature.id;
    let value = device.values.get(&id);
    let write = |value: Value| core.set_feature(Some(device.key()), id, value);
    match &feature.descriptor {
        Descriptor::Choice(options) => choice_control(ui, device, feature, options, value, write),
        Descriptor::Level(range) => level_control(ui, device, feature, range, value, write),
        Descriptor::Toggle => toggle_control(ui, feature, value, write),
        Descriptor::Versions => {
            let text = match value {
                Some(Value::Versions(parts)) => parts
                    .iter()
                    .map(|(_, version)| version.as_str())
                    .collect::<Vec<_>>()
                    .join(" · "),
                _ => "unknown".to_owned(),
            };
            ui.monospace(text).on_hover_text(
                "Component versions as reported by the device. Which part each number belongs to \
                 may not be known.",
            );
        }
        Descriptor::Action => {
            if feature.id.kind == FeatureKind::UsbReplug {
                let supported = crate::platform::USB_RESET_SUPPORTED;
                if ui
                    .add_enabled(supported, egui::Button::new("Reconnect"))
                    .on_disabled_hover_text("Not available on this operating system yet.")
                    .clicked()
                {
                    core.reconnect_usb(Some(device.key()));
                }
            }
        }
        Descriptor::Battery { .. }
        | Descriptor::Equalizer { .. }
        | Descriptor::Balance { .. }
        | Descriptor::AudioEndpoint { .. } => {}
    }
}

/// Segmented buttons for a few options, a drop-down for more.
fn choice_control(
    ui: &mut egui::Ui,
    device: &DeviceSnapshot,
    feature: &Feature,
    options: &'static [ss_device::ChoiceOption],
    value: Option<&Value>,
    write: impl Fn(Value),
) {
    let current = match value {
        Some(Value::Choice(key)) => Some(*key),
        _ => None,
    };
    let selected = options
        .iter()
        .find(|o| Some(o.key) == current)
        .map_or("Unknown", |o| o.label);
    if !feature.writable {
        ui.label(selected);
    } else if options.len() <= MAX_SEGMENTS {
        let segments: Vec<(&'static str, &str)> =
            options.iter().map(|o| (o.key, o.label)).collect();
        if let Some(key) = components::segmented(ui, current, &segments) {
            write(Value::Choice(key));
        }
    } else {
        egui::ComboBox::from_id_salt(("choice", device.key(), feature.id))
            .selected_text(selected)
            .width(170.0)
            .show_ui(ui, |ui| {
                for option in options {
                    if ui
                        .selectable_label(current == Some(option.key), option.label)
                        .clicked()
                    {
                        write(Value::Choice(option.key));
                    }
                }
            });
    }
}

/// Named steps as segments (e.g. sidetone Off/Low/Medium/High), otherwise a slider.
fn level_control(
    ui: &mut egui::Ui,
    device: &DeviceSnapshot,
    feature: &Feature,
    range: &ss_device::LevelRange,
    value: Option<&Value>,
    write: impl Fn(Value),
) {
    let Some(Value::Level(level)) = value else {
        ui.label("Unknown");
        return;
    };
    let steps = usize::from(range.max - range.min) + 1;
    match range.labels {
        Some(labels) if steps <= MAX_SEGMENTS && feature.writable => {
            let segments: Vec<(u8, &str)> = (range.min..=range.max)
                .zip(labels.iter().copied())
                .collect();
            if let Some(level) = components::segmented(ui, Some(*level), &segments) {
                write(Value::Level(level));
            }
        }
        _ if feature.writable => {
            if let Some(level) = level_slider(
                ui,
                (device.key(), feature.id),
                *level,
                range.min..=range.max,
            ) {
                write(Value::Level(level));
            }
        }
        _ => {
            ui.label(
                range
                    .label(*level)
                    .map_or_else(|| level.to_string(), str::to_owned),
            );
        }
    }
}

/// A switch, or a status word for read-only toggles (mic mute: Muted / Live).
fn toggle_control(
    ui: &mut egui::Ui,
    feature: &Feature,
    value: Option<&Value>,
    write: impl Fn(Value),
) {
    let on = matches!(value, Some(Value::Bool(true)));
    if feature.writable {
        let mut edited = on;
        if components::switch(ui, &mut edited) {
            write(Value::Bool(edited));
        }
    } else if feature.id.kind == FeatureKind::MicMute {
        if on {
            components::status(ui, Tone::Warning, "Muted");
        } else {
            components::status(ui, Tone::Good, "Live");
        }
    } else if on {
        components::status(ui, Tone::Good, "On");
    } else {
        components::status(ui, Tone::Neutral, "Off");
    }
}

/// USB/Bluetooth identity rows of the Device card.
fn identity_rows(ui: &mut egui::Ui, device: &DeviceSnapshot, first: bool) {
    if !first {
        components::divider(ui);
    }
    let instance = &device.instance;
    let bus = match instance.bus {
        ss_device::Bus::Usb => "USB",
        ss_device::Bus::Bluetooth => "Bluetooth",
        ss_device::Bus::Other => "Connection",
    };
    components::row(ui, bus, None, |ui| {
        let release = instance
            .release
            .map_or_else(String::new, |r| format!(" · release {}", bcd_release(r)));
        ui.monospace(format!("PID {:#06x}{release}", instance.product_id));
    });
}

/// Editor for an on-device EQ curve, with presets of the same band count.
fn curve_editor(
    ui: &mut egui::Ui,
    core: &AppCore,
    snapshot: &Snapshot,
    device: &DeviceSnapshot,
    feature: &Feature,
) {
    let p = theme::palette(ui);
    let Descriptor::Equalizer {
        bands,
        centre_hz,
        min_db,
        max_db,
        step_db,
    } = feature.descriptor
    else {
        return;
    };
    let Some(Value::Curve(curve)) = device.values.get(&feature.id).cloned() else {
        ui.label(
            RichText::new("The device did not report a custom curve.").color(p.text_secondary),
        );
        return;
    };
    let write =
        |curve: Vec<f32>| core.set_feature(Some(device.key()), feature.id, Value::Curve(curve));
    ui.horizontal(|ui| {
        ui.label(RichText::new("Custom curve").color(p.text));
        ui.label(
            RichText::new(format!("{min_db:+} … {max_db:+} dB per band, low to high"))
                .small()
                .color(p.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let flat = curve.iter().all(|g| *g == 0.0);
            if ui
                .add_enabled(!flat, egui::Button::new("Reset to flat"))
                .clicked()
            {
                write(vec![0.0; usize::from(bands)]);
            }
        });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        #[allow(clippy::cast_precision_loss)] // a handful of bands
        let band_width = (ui.available_width() / f32::from(bands)).floor();
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.spacing_mut().slider_width = EQ_SLIDER_HEIGHT;
        for (index, gain) in curve.iter().enumerate() {
            ui.allocate_ui_with_layout(
                egui::vec2(band_width, EQ_SLIDER_HEIGHT + 60.0),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    if let Some(db) =
                        band_slider(ui, (device.key(), index), *gain, min_db..=max_db, step_db)
                    {
                        let mut edited = curve.clone();
                        edited[index] = db;
                        write(edited);
                    }
                    let label = band_label(index, usize::from(bands), centre_hz);
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
        presets::Kind::DeviceEq(device.info.id),
        ("device-eq", device.key()),
        || Some(PresetData::HeadsetEq(curve.clone())),
    );
    if let Some(PresetData::HeadsetEq(stored)) = preset
        && stored.len() == usize::from(bands)
    {
        // Snap to the device grid and clamp to its range.
        let snapped = stored
            .iter()
            .map(|db| ((db / step_db).round() * step_db).clamp(min_db, max_db))
            .collect();
        write(snapped);
    }
}

/// Band caption: its centre frequency if known, otherwise Low … High.
fn band_label(index: usize, bands: usize, centre_hz: Option<&[f32]>) -> String {
    if let Some(hz) = centre_hz.and_then(|c| c.get(index)) {
        return if *hz >= 1000.0 {
            let khz = hz / 1000.0;
            if khz.fract() == 0.0 {
                format!("{khz:.0}k")
            } else {
                format!("{khz:.1}k")
            }
        } else {
            format!("{hz:.0}")
        };
    }
    match index {
        0 => "Low".to_owned(),
        i if i + 1 == bands => "High".to_owned(),
        i => format!("{}", i + 1),
    }
}

/// One vertical band slider; returns the new gain once the user lets go.
fn band_slider(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash,
    value: f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
) -> Option<f32> {
    // While dragging, the device still reports the old value; keep the dragged one locally.
    let id = ui.id().with(("eq-band", salt));
    let mut edited = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(value);
    let response = ui.add(
        egui::Slider::new(&mut edited, range)
            .vertical()
            .step_by(f64::from(step))
            .fixed_decimals(1),
    );
    if response.dragged() {
        ui.data_mut(|d| d.insert_temp(id, edited));
        return None;
    }
    ui.data_mut(|d| d.remove::<f32>(id));
    let finished = response.drag_stopped() || response.changed();
    (finished && (edited - value).abs() >= step / 2.0).then_some(edited)
}

/// An integer slider that reports only finished changes (release or click), so dragging does not
/// flood the device with commands.
fn level_slider(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash,
    value: u8,
    range: std::ops::RangeInclusive<u8>,
) -> Option<u8> {
    let id = ui.id().with(("level", salt));
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

/// Empty state when no supported device is connected.
pub(super) fn not_connected(ui: &mut egui::Ui) {
    components::card(ui, None, |ui| {
        components::status(ui, Tone::Error, "No supported device connected");
        let p = theme::palette(ui);
        ui.label(
            RichText::new(
                "Connect your device to this computer. The Arctis Nova Pro Wireless connects with \
                 its base station (on macOS the USB-1 port); if it is plugged in, it may be \
                 switched to the other USB input — switch it back on the base station.",
            )
            .color(p.text_secondary),
        );
    });
}

/// "Headset", "Keyboard", …
pub(super) const fn class_word(class: DeviceClass) -> &'static str {
    match class {
        DeviceClass::Headset => "Headset",
        DeviceClass::Keyboard => "Keyboard",
        DeviceClass::Mouse => "Mouse",
        DeviceClass::Other => "Device",
    }
}

/// Sidebar / overview icon of a device class.
pub(super) const fn class_icon(class: DeviceClass) -> &'static str {
    match class {
        DeviceClass::Headset => "🎧",
        DeviceClass::Keyboard => "⌨",
        DeviceClass::Mouse => "🖱",
        DeviceClass::Other => "🔌",
    }
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

/// A USB `bcdDevice` value as `major.minor` (0x0130 → "1.30").
fn bcd_release(bcd: u16) -> String {
    format!("{:x}.{:02x}", bcd >> 8, bcd & 0xFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_kinds_have_a_card_and_status_kinds_none() {
        assert!(presentation(FeatureKind::Battery).is_none());
        assert!(presentation(FeatureKind::BalanceDial).is_none());
        let transparency = presentation(FeatureKind::Transparency).unwrap();
        assert_eq!(transparency.group, Group::NoiseControl);
        assert_eq!(
            transparency.shown_when,
            Some((FeatureKind::NoiseControl, "transparency"))
        );
    }

    #[test]
    fn band_labels() {
        assert_eq!(band_label(0, 10, None), "Low");
        assert_eq!(band_label(9, 10, None), "High");
        assert_eq!(band_label(4, 10, None), "5");
        assert_eq!(band_label(1, 3, Some(&[100.0, 2500.0, 8000.0])), "2.5k");
        assert_eq!(band_label(2, 3, Some(&[100.0, 2500.0, 8000.0])), "8k");
        assert_eq!(band_label(0, 3, Some(&[100.0, 2500.0, 8000.0])), "100");
    }

    #[test]
    fn usb_release_is_bcd() {
        assert_eq!(bcd_release(0x0130), "1.30");
    }
}
