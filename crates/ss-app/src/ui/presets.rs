//! The preset bar shared by the device, AudioMix and Microphone pages (`docs/architecture/app.md`
//! §3.1): pick a preset to apply it, save the current sound under a name, delete, export, import.

use std::path::PathBuf;

use eframe::egui;
use ss_store::presets::{PresetData, PresetsConfig, export_json, import_json};

use crate::core::{AppCore, Snapshot};
use crate::i18n::tr;

/// Which preset list a bar works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// On-device EQ curves of the device with this id (ADR 0008).
    DeviceEq(&'static str),
    /// AudioMix channel sounds.
    Playback,
    /// Mic chain sounds.
    Mic,
}

impl Kind {
    fn names(self, presets: &PresetsConfig) -> Vec<String> {
        match self {
            Self::DeviceEq(device) => presets
                .eq_of(device)
                .map(|curves| curves.keys().cloned().collect())
                .unwrap_or_default(),
            Self::Playback => presets.playback.keys().cloned().collect(),
            Self::Mic => presets.mic.keys().cloned().collect(),
        }
    }

    fn get(self, presets: &PresetsConfig, name: &str) -> Option<PresetData> {
        match self {
            Self::DeviceEq(device) => presets
                .eq_of(device)
                .and_then(|curves| curves.get(name))
                .cloned()
                .map(PresetData::HeadsetEq),
            Self::Playback => presets
                .playback
                .get(name)
                .cloned()
                .map(PresetData::Playback),
            Self::Mic => presets.mic.get(name).cloned().map(PresetData::Mic),
        }
    }

    fn remove(self, presets: &mut PresetsConfig, name: &str) {
        match self {
            Self::DeviceEq(device) => {
                if let Some(curves) = presets.device_eq.get_mut(device) {
                    curves.remove(name);
                }
            }
            Self::Playback => {
                presets.playback.remove(name);
            }
            Self::Mic => {
                presets.mic.remove(name);
            }
        }
    }

    /// The list `data` belongs to when imported from a bar of kind `bar` (EQ curves go to the
    /// bar's device; `None` if there is none).
    const fn of(data: &PresetData, bar: Self) -> Option<Self> {
        match (data, bar) {
            (PresetData::HeadsetEq(_), Self::DeviceEq(device)) => Some(Self::DeviceEq(device)),
            (PresetData::HeadsetEq(_), _) => None,
            (PresetData::Playback(_), _) => Some(Self::Playback),
            (PresetData::Mic(_), _) => Some(Self::Mic),
        }
    }

    /// The device that EQ curves of this list belong to.
    const fn eq_device(self) -> Option<&'static str> {
        match self {
            Self::DeviceEq(device) => Some(device),
            Self::Playback | Self::Mic => None,
        }
    }

    fn label(self) -> String {
        match self {
            Self::DeviceEq(_) => tr!("presets-kind-device-eq"),
            Self::Playback => tr!("presets-kind-channel"),
            Self::Mic => tr!("presets-kind-mic"),
        }
    }
}

/// Per-bar UI state kept in egui's temp memory.
#[derive(Debug, Clone, Default)]
struct BarState {
    selected: Option<String>,
    new_name: String,
    message: Option<String>,
}

/// Draws the bar. `current` returns the sound to save (`None` if unknown, e.g. headset not
/// connected). Returns the preset the user picked, for the page to apply.
pub fn bar(
    ui: &mut egui::Ui,
    core: &AppCore,
    snapshot: &Snapshot,
    kind: Kind,
    salt: impl std::hash::Hash,
    current: impl FnOnce() -> Option<PresetData>,
) -> Option<PresetData> {
    let id = ui.id().with(("presets", salt));
    let mut state: BarState = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    let presets = &snapshot.config.presets;
    let names = kind.names(presets);
    if state.selected.as_ref().is_some_and(|s| !names.contains(s)) {
        state.selected = None;
    }
    let mut apply = None;

    ui.horizontal_wrapped(|ui| {
        ui.label(tr!("presets-label"));
        egui::ComboBox::from_id_salt(id.with("combo"))
            .selected_text(state.selected.as_deref().unwrap_or("—"))
            .show_ui(ui, |ui| {
                if names.is_empty() {
                    ui.label(egui::RichText::new(tr!("presets-none")).weak());
                }
                for name in &names {
                    if ui
                        .selectable_label(state.selected.as_ref() == Some(name), name)
                        .clicked()
                    {
                        state.selected = Some(name.clone());
                        apply = kind.get(presets, name);
                        state.message = Some(tr!("presets-applied", name = name.as_str()));
                    }
                }
            });
        if let Some(name) = state.selected.clone() {
            if ui.button(tr!("presets-delete")).clicked() {
                core.update_config(|c| kind.remove(&mut c.presets, &name));
                state.selected = None;
                state.message = Some(tr!("presets-deleted", name = name.as_str()));
            }
            if ui.button(tr!("presets-export")).clicked() {
                state.message = kind.get(presets, &name).map(|data| export(&name, &data));
            }
        }
        if ui.button(tr!("presets-import")).clicked() {
            state.message = match import(core, kind) {
                Ok(None) => None,
                Ok(Some((name, imported))) if imported == kind => {
                    state.selected = Some(name.clone());
                    Some(tr!("presets-imported-pick", name = name.as_str()))
                }
                Ok(Some((name, imported))) => Some(tr!(
                    "presets-imported-as",
                    name = name.as_str(),
                    kind = imported.label()
                )),
                Err(error) => Some(tr!("presets-import-failed", error = error)),
            };
        }
    });
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.new_name)
                .hint_text(tr!("presets-name-hint"))
                .desired_width(160.0),
        );
        let name = state.new_name.trim().to_owned();
        if ui
            .add_enabled(!name.is_empty(), egui::Button::new(tr!("presets-save")))
            .clicked()
        {
            if let Some(data) = current() {
                let replaced = names.contains(&name);
                core.update_config(|c| c.presets.insert(name.clone(), data, kind.eq_device()));
                state.message = Some(if replaced {
                    tr!("presets-replaced", name = name.as_str())
                } else {
                    tr!("presets-saved", name = name.as_str())
                });
                state.selected = Some(name);
                state.new_name.clear();
            } else {
                state.message = Some(tr!("presets-nothing-to-save"));
            }
        }
        if let Some(message) = &state.message {
            ui.label(egui::RichText::new(message).weak());
        }
    });

    ui.data_mut(|d| d.insert_temp(id, state));
    apply
}

/// Asks for a file and writes the preset; returns a status message.
fn export(name: &str, data: &PresetData) -> String {
    let Some(path) = rfd::FileDialog::new()
        .add_filter(tr!("presets-file-filter"), &["json"])
        .set_file_name(format!("{}.json", file_stem(name)))
        .save_file()
    else {
        return tr!("presets-export-cancelled");
    };
    match export_json(name, data)
        .map_err(|e| e.to_string())
        .and_then(|text| std::fs::write(&path, text).map_err(|e| e.to_string()))
    {
        Ok(()) => tr!("presets-exported", path = path.display().to_string()),
        Err(error) => tr!("presets-export-failed", error = error),
    }
}

/// Asks for a file and stores the preset it contains (any kind; EQ curves only on a device page,
/// for that device). `Ok(None)` = cancelled.
fn import(core: &AppCore, bar: Kind) -> Result<Option<(String, Kind)>, String> {
    let Some(path): Option<PathBuf> = rfd::FileDialog::new()
        .add_filter(tr!("presets-import-filter"), &["json"])
        .pick_file()
    else {
        return Ok(None);
    };
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let (name, data) = import_json(&text).map_err(|e| e.to_string())?;
    let kind = Kind::of(&data, bar).ok_or_else(|| tr!("presets-device-eq-elsewhere"))?;
    core.update_config(|c| c.presets.insert(name.clone(), data, kind.eq_device()));
    Ok(Some((name, kind)))
}

/// A file name derived from a preset name (no path separators or reserved characters).
fn file_stem(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}
