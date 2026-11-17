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
use crate::i18n::{self, tr};

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

    fn title(self) -> String {
        match self {
            Self::NoiseControl => tr!("device-group-noise-control"),
            Self::Microphone => tr!("device-group-microphone"),
            Self::Equalizer => tr!("device-group-equalizer"),
            Self::Power => tr!("device-group-power"),
            Self::Device => tr!("device-group-device"),
            Self::More => tr!("device-group-more"),
        }
    }
}

/// How one feature kind appears on the device page.
struct Presentation {
    group: Group,
    label: String,
    description: Option<String>,
    /// Only shown while another feature has this choice (e.g. transparency strength).
    shown_when: Option<(FeatureKind, &'static str)>,
}

/// The presentation of `kind`; `None` = shown elsewhere (status card, AudioMix) or not at all.
fn presentation(kind: FeatureKind) -> Option<Presentation> {
    fn p(group: Group, label: String, description: Option<String>) -> Presentation {
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
            tr!("device-noise-control-mode"),
            Some(tr!("device-noise-control-mode-hint")),
        ),
        FeatureKind::Transparency => Presentation {
            shown_when: Some((FeatureKind::NoiseControl, "transparency")),
            ..p(
                Group::NoiseControl,
                tr!("device-transparency-strength"),
                Some(tr!("device-transparency-strength-hint")),
            )
        },
        FeatureKind::MicMute => p(
            Group::Microphone,
            tr!("device-mic-mute"),
            Some(tr!("device-mic-mute-hint")),
        ),
        FeatureKind::MicVolume => p(Group::Microphone, tr!("device-mic-volume"), None),
        FeatureKind::Sidetone => p(
            Group::Microphone,
            tr!("device-sidetone"),
            Some(tr!("device-sidetone-hint")),
        ),
        FeatureKind::MicLedBrightness => p(
            Group::Microphone,
            tr!("device-mic-led-brightness"),
            Some(tr!("device-mic-led-brightness-hint")),
        ),
        FeatureKind::EqPreset => p(
            Group::Equalizer,
            tr!("device-eq-preset"),
            Some(tr!("device-eq-preset-hint")),
        ),
        FeatureKind::Equalizer => Presentation {
            shown_when: Some((FeatureKind::EqPreset, "custom")),
            ..p(Group::Equalizer, tr!("device-custom-curve"), None)
        },
        FeatureKind::AutoOff => p(
            Group::Power,
            tr!("device-auto-off"),
            Some(tr!("device-auto-off-hint")),
        ),
        FeatureKind::DisplayBrightness => p(Group::Power, tr!("device-display-brightness"), None),
        FeatureKind::Firmware => p(Group::Device, tr!("device-firmware"), None),
        FeatureKind::UsbReplug => p(
            Group::Device,
            tr!("device-usb-replug"),
            Some(tr!("device-usb-replug-hint")),
        ),
        _ => p(Group::More, tr!("device-setting"), None),
    })
}

/// The page of the connected device at `index` (`None` if it went away).
pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, index: usize) {
    let Some(device) = snapshot.devices.devices.get(index) else {
        components::page_header(ui, &tr!("device-title"), &tr!("device-gone"));
        not_connected(ui);
        return;
    };
    components::page_header(
        ui,
        device.info.name,
        &tr!("device-subtitle", class = class_word(device.info.class)),
    );
    if device.info.support == ss_device::Support::Experimental {
        components::banner(ui, Tone::Warning, &tr!("device-experimental"));
        ui.add_space(8.0);
    }
    if !device.ready {
        components::card(ui, None, |ui| {
            components::status(ui, Tone::Neutral, &tr!("device-reading", hub = hub(device)));
        });
        return;
    }
    if device.linked() == Some(false) {
        components::banner(
            ui,
            Tone::Neutral,
            &tr!(
                "device-unlinked",
                hub = hub(device),
                class = class_word_inline(device.info.class)
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
        components::card(ui, Some(&group.title()), |ui| {
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
    components::card(ui, Some(&tr!("device-battery")), |ui| {
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
                BatterySlot::Main => (
                    tr!("device-battery"),
                    batteries.map(|(_, charge)| charge_text(charge)),
                ),
                BatterySlot::Spare => (
                    tr!("device-spare-battery"),
                    Some(tr!("device-spare-battery-hint")),
                ),
            };
            components::row(ui, &label, description.as_deref(), |ui| {
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
        let kind = format!("{:?}", feature.id.kind);
        i18n::lookup(&format!("device-kind-{}", i18n::id_part(&kind))).unwrap_or(kind)
    } else {
        presentation.label.clone()
    };
    if let Descriptor::Equalizer { .. } = feature.descriptor {
        curve_editor(ui, core, snapshot, device, feature);
        return;
    }
    components::row(ui, &label, presentation.description.as_deref(), |ui| {
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
                _ => tr!("device-versions-unknown"),
            };
            ui.monospace(text)
                .on_hover_text(tr!("device-versions-hint"));
        }
        Descriptor::Action => {
            if feature.id.kind == FeatureKind::UsbReplug {
                let supported = crate::platform::USB_RESET_SUPPORTED;
                if ui
                    .add_enabled(supported, egui::Button::new(tr!("device-reconnect")))
                    .on_disabled_hover_text(tr!("device-reconnect-unsupported"))
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
    let kind = feature.id.kind;
    let labels: Vec<String> = options
        .iter()
        .map(|o| driver_text(kind, o.key, o.label))
        .collect();
    let selected = options
        .iter()
        .zip(&labels)
        .find(|(o, _)| Some(o.key) == current)
        .map_or_else(|| tr!("device-unknown"), |(_, label)| label.clone());
    if !feature.writable {
        ui.label(selected);
    } else if options.len() <= MAX_SEGMENTS {
        let segments: Vec<(&'static str, &str)> = options
            .iter()
            .zip(&labels)
            .map(|(o, label)| (o.key, label.as_str()))
            .collect();
        if let Some(key) = components::segmented(ui, current, &segments) {
            write(Value::Choice(key));
        }
    } else {
        egui::ComboBox::from_id_salt(("choice", device.key(), feature.id))
            .selected_text(selected)
            .width(170.0)
            .show_ui(ui, |ui| {
                for (option, label) in options.iter().zip(&labels) {
                    if ui
                        .selectable_label(current == Some(option.key), label)
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
        ui.label(tr!("device-unknown"));
        return;
    };
    let kind = feature.id.kind;
    let steps = usize::from(range.max - range.min) + 1;
    match range.labels {
        Some(labels) if steps <= MAX_SEGMENTS && feature.writable => {
            let labels: Vec<String> = labels
                .iter()
                .map(|label| driver_text(kind, label, label))
                .collect();
            let segments: Vec<(u8, &str)> = (range.min..=range.max)
                .zip(labels.iter().map(String::as_str))
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
            ui.label(range.label(*level).map_or_else(
                || level.to_string(),
                |label| driver_text(kind, label, label),
            ));
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
            components::status(ui, Tone::Warning, &tr!("device-muted"));
        } else {
            components::status(ui, Tone::Good, &tr!("device-live"));
        }
    } else if on {
        components::status(ui, Tone::Good, &tr!("device-on"));
    } else {
        components::status(ui, Tone::Neutral, &tr!("device-off"));
    }
}

/// USB/Bluetooth identity rows of the Device card.
fn identity_rows(ui: &mut egui::Ui, device: &DeviceSnapshot, first: bool) {
    if !first {
        components::divider(ui);
    }
    let instance = &device.instance;
    let bus = match instance.bus {
        ss_device::Bus::Usb => "USB".to_owned(),
        ss_device::Bus::Bluetooth => "Bluetooth".to_owned(),
        ss_device::Bus::Other => tr!("device-connection"),
    };
    components::row(ui, &bus, None, |ui| {
        let release = instance.release.map_or_else(String::new, |r| {
            format!(" · {}", tr!("device-release", version = bcd_release(r)))
        });
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
        ui.label(RichText::new(tr!("device-no-curve")).color(p.text_secondary));
        return;
    };
    let write =
        |curve: Vec<f32>| core.set_feature(Some(device.key()), feature.id, Value::Curve(curve));
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr!("device-custom-curve")).color(p.text));
        ui.label(
            RichText::new(tr!(
                "device-curve-range",
                min = format!("{min_db:+}"),
                max = format!("{max_db:+}")
            ))
            .small()
            .color(p.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let flat = curve.iter().all(|g| *g == 0.0);
            if ui
                .add_enabled(!flat, egui::Button::new(tr!("device-reset-flat")))
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
        0 => tr!("device-band-low"),
        i if i + 1 == bands => tr!("device-band-high"),
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
        components::status(ui, Tone::Error, &tr!("device-none"));
        let p = theme::palette(ui);
        ui.label(RichText::new(tr!("device-none-hint")).color(p.text_secondary));
    });
}

/// "Headset", "Keyboard", … as a label or at the start of a sentence.
pub(super) fn class_word(class: DeviceClass) -> String {
    match class {
        DeviceClass::Headset => tr!("device-class-headset"),
        DeviceClass::Keyboard => tr!("device-class-keyboard"),
        DeviceClass::Mouse => tr!("device-class-mouse"),
        DeviceClass::Other => tr!("device-class-other"),
    }
}

/// The class word inside a sentence ("headset" in English, "Headset" in German).
pub(super) fn class_word_inline(class: DeviceClass) -> String {
    match class {
        DeviceClass::Headset => tr!("device-class-headset-inline"),
        DeviceClass::Keyboard => tr!("device-class-keyboard-inline"),
        DeviceClass::Mouse => tr!("device-class-mouse-inline"),
        DeviceClass::Other => tr!("device-class-other-inline"),
    }
}

/// The device's hub (e.g. "base station") inside a sentence, translated by the message
/// `device-hub-<name>` if one exists.
pub(super) fn hub(device: &DeviceSnapshot) -> String {
    i18n::hub_name(device.hub())
}

/// A text of the device driver (choice option or named level), translated by the message
/// `device-<kind>-<key>` (e.g. `device-noise-control-anc`), otherwise the driver's English
/// `fallback`.
fn driver_text(kind: FeatureKind, key: &str, fallback: &str) -> String {
    i18n::lookup(&driver_text_id(kind, key)).unwrap_or_else(|| fallback.to_owned())
}

/// `device-<kind>-<key>`, both parts in Fluent-safe kebab case.
fn driver_text_id(kind: FeatureKind, key: &str) -> String {
    format!(
        "device-{}-{}",
        i18n::id_part(&format!("{kind:?}")),
        i18n::id_part(key)
    )
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
pub(super) fn charge_text(state: ChargeState) -> String {
    match state {
        ChargeState::Discharging => tr!("device-charge-discharging"),
        ChargeState::Charging => tr!("device-charge-charging"),
        ChargeState::PluggedNotCharging => tr!("device-charge-plugged"),
        ChargeState::Unknown => tr!("device-charge-unknown"),
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
    fn driver_text_ids_are_kebab_case() {
        assert_eq!(
            driver_text_id(FeatureKind::NoiseControl, "anc"),
            "device-noise-control-anc"
        );
        assert_eq!(
            driver_text_id(FeatureKind::EqPreset, "bass_boost"),
            "device-eq-preset-bass-boost"
        );
        assert_eq!(
            driver_text_id(FeatureKind::AutoOff, "10min"),
            "device-auto-off-10min"
        );
        assert_eq!(
            driver_text_id(FeatureKind::Sidetone, "Medium"),
            "device-sidetone-medium"
        );
        assert_eq!(i18n::id_part("base station"), "base-station");
    }

    #[test]
    fn driver_texts_have_messages() {
        // English is loaded in tests; a missing message would return the "?" fallback.
        assert_eq!(
            driver_text(FeatureKind::NoiseControl, "anc", "?"),
            "Noise cancelling"
        );
        assert_eq!(driver_text(FeatureKind::Sidetone, "Medium", "?"), "Medium");
        assert_eq!(driver_text(FeatureKind::AutoOff, "never", "?"), "Never");
    }

    #[test]
    fn usb_release_is_bcd() {
        assert_eq!(bcd_release(0x0130), "1.30");
    }
}
