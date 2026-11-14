//! Small shared widgets.

use eframe::egui;

use crate::audio::supervisor::ServiceStatus;

/// A rounded status label; green-ish when `ok`.
pub fn pill(ui: &mut egui::Ui, text: &str, ok: bool) {
    let visuals = ui.visuals();
    let (fill, stroke) = if ok {
        (visuals.selection.bg_fill, visuals.selection.stroke.color)
    } else {
        (visuals.faint_bg_color, visuals.weak_text_color())
    };
    egui::Frame::new()
        .fill(fill)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.colored_label(stroke, text);
        });
}

/// Explains a service status below its switch (nothing for `Off`/`Running`).
pub fn status_detail(ui: &mut egui::Ui, status: &ServiceStatus) {
    match status {
        ServiceStatus::Off | ServiceStatus::Running => {}
        ServiceStatus::Blocked(reason) => {
            ui.colored_label(ui.visuals().warn_fg_color, format!("Paused: {reason}"));
        }
        ServiceStatus::Failed(error) => {
            ui.colored_label(ui.visuals().error_fg_color, format!("Error: {error}"));
        }
    }
}

/// A slider that reports a new value only when the user changed it.
pub fn slider(
    ui: &mut egui::Ui,
    value: f32,
    range: std::ops::RangeInclusive<f32>,
    label: &str,
    suffix: &str,
) -> Option<f32> {
    let mut edited = value;
    let response = ui.add(
        egui::Slider::new(&mut edited, range)
            .text(label)
            .suffix(suffix),
    );
    (response.changed() && (edited - value).abs() > f32::EPSILON).then_some(edited)
}

/// A device picked in [`device_combo`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceChoice {
    /// Follow the system default device.
    SystemDefault,
    /// A specific device (UID).
    Device(String),
}

impl DeviceChoice {
    /// The UID to store (`None` = system default).
    pub fn into_uid(self) -> Option<String> {
        match self {
            Self::SystemDefault => None,
            Self::Device(uid) => Some(uid),
        }
    }
}

/// A device picker with a "System default" entry. Returns the new selection when changed.
pub fn device_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: Option<&str>,
    devices: &[crate::audio::plan::DeviceInfo],
) -> Option<DeviceChoice> {
    let label = selected.map_or_else(
        || "System default".to_owned(),
        |uid| {
            devices
                .iter()
                .find(|d| d.uid == uid)
                .map_or_else(|| format!("{uid} (not connected)"), |d| d.name.clone())
        },
    );
    let mut choice: Option<DeviceChoice> = None;
    egui::ComboBox::from_id_salt(id)
        .selected_text(label)
        .width(280.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(selected.is_none(), "System default")
                .clicked()
            {
                choice = Some(DeviceChoice::SystemDefault);
            }
            for device in devices {
                let is_selected = selected == Some(device.uid.as_str());
                if ui.selectable_label(is_selected, &device.name).clicked() {
                    choice = Some(DeviceChoice::Device(device.uid.clone()));
                }
            }
        });
    choice.filter(|new| match new {
        DeviceChoice::SystemDefault => selected.is_some(),
        DeviceChoice::Device(uid) => selected != Some(uid.as_str()),
    })
}

/// Editable priority list of backup devices (SON-27). Returns the new list when it changed.
pub fn fallback_list(
    ui: &mut egui::Ui,
    id_salt: &str,
    list: &[String],
    devices: &[crate::audio::plan::DeviceInfo],
) -> Option<Vec<String>> {
    let name_of = |uid: &str| {
        devices
            .iter()
            .find(|d| d.uid == uid)
            .map_or_else(|| format!("{uid} (not connected)"), |d| d.name.clone())
    };
    let mut changed: Option<Vec<String>> = None;
    for (index, uid) in list.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("{}. {}", index + 1, name_of(uid)));
            if ui
                .add_enabled(index > 0, egui::Button::new("↑").small())
                .clicked()
            {
                let mut new = list.to_vec();
                new.swap(index, index - 1);
                changed = Some(new);
            }
            if ui
                .add_enabled(index + 1 < list.len(), egui::Button::new("↓").small())
                .clicked()
            {
                let mut new = list.to_vec();
                new.swap(index, index + 1);
                changed = Some(new);
            }
            if ui.small_button("✕").clicked() {
                let mut new = list.to_vec();
                new.remove(index);
                changed = Some(new);
            }
        });
    }
    let candidates: Vec<_> = devices.iter().filter(|d| !list.contains(&d.uid)).collect();
    if !candidates.is_empty() {
        egui::ComboBox::from_id_salt((id_salt, "add-backup"))
            .selected_text("Add backup…")
            .show_ui(ui, |ui| {
                for device in candidates {
                    if ui.selectable_label(false, &device.name).clicked() {
                        let mut new = list.to_vec();
                        new.push(device.uid.clone());
                        changed = Some(new);
                    }
                }
            });
    }
    changed
}
