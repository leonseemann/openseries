//! The preset bar shared by the Headset, AudioMix and Microphone pages (`docs/architecture/app.md`
//! §3.1): pick a preset to apply it, save the current sound under a name, delete, export, import.

use std::path::PathBuf;

use eframe::egui;
use ss_store::presets::{PresetData, PresetsConfig, export_json, import_json};

use crate::core::{AppCore, Snapshot};

/// Which preset list a bar works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// On-device headset EQ curves.
    HeadsetEq,
    /// AudioMix channel sounds.
    Playback,
    /// Mic chain sounds.
    Mic,
}

impl Kind {
    fn names(self, presets: &PresetsConfig) -> Vec<String> {
        match self {
            Self::HeadsetEq => presets.headset_eq.keys().cloned().collect(),
            Self::Playback => presets.playback.keys().cloned().collect(),
            Self::Mic => presets.mic.keys().cloned().collect(),
        }
    }

    fn get(self, presets: &PresetsConfig, name: &str) -> Option<PresetData> {
        match self {
            Self::HeadsetEq => presets
                .headset_eq
                .get(name)
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
            Self::HeadsetEq => {
                presets.headset_eq.remove(name);
            }
            Self::Playback => {
                presets.playback.remove(name);
            }
            Self::Mic => {
                presets.mic.remove(name);
            }
        }
    }

    const fn of(data: &PresetData) -> Self {
        match data {
            PresetData::HeadsetEq(_) => Self::HeadsetEq,
            PresetData::Playback(_) => Self::Playback,
            PresetData::Mic(_) => Self::Mic,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::HeadsetEq => "headset EQ",
            Self::Playback => "channel",
            Self::Mic => "microphone",
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
        ui.label("Preset");
        egui::ComboBox::from_id_salt(id.with("combo"))
            .selected_text(state.selected.as_deref().unwrap_or("—"))
            .show_ui(ui, |ui| {
                if names.is_empty() {
                    ui.label(egui::RichText::new("no presets yet").weak());
                }
                for name in &names {
                    if ui
                        .selectable_label(state.selected.as_ref() == Some(name), name)
                        .clicked()
                    {
                        state.selected = Some(name.clone());
                        apply = kind.get(presets, name);
                        state.message = Some(format!("applied \"{name}\""));
                    }
                }
            });
        if let Some(name) = state.selected.clone() {
            if ui.button("Delete").clicked() {
                core.update_config(|c| kind.remove(&mut c.presets, &name));
                state.selected = None;
                state.message = Some(format!("deleted \"{name}\""));
            }
            if ui.button("Export…").clicked() {
                state.message = kind.get(presets, &name).map(|data| export(&name, &data));
            }
        }
        if ui.button("Import…").clicked() {
            state.message = match import(core) {
                Ok(None) => None,
                Ok(Some((name, imported))) if imported == kind => {
                    state.selected = Some(name.clone());
                    Some(format!("imported \"{name}\" — pick it to apply"))
                }
                Ok(Some((name, imported))) => Some(format!(
                    "imported \"{name}\" as a {} preset",
                    imported.label()
                )),
                Err(error) => Some(format!("import failed: {error}")),
            };
        }
    });
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.new_name)
                .hint_text("name")
                .desired_width(160.0),
        );
        let name = state.new_name.trim().to_owned();
        if ui
            .add_enabled(!name.is_empty(), egui::Button::new("Save current"))
            .clicked()
        {
            if let Some(data) = current() {
                let replaced = names.contains(&name);
                core.update_config(|c| c.presets.insert(name.clone(), data));
                state.message = Some(if replaced {
                    format!("replaced \"{name}\"")
                } else {
                    format!("saved \"{name}\"")
                });
                state.selected = Some(name);
                state.new_name.clear();
            } else {
                state.message = Some("nothing to save yet".to_owned());
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
        .add_filter("StealSeries preset", &["json"])
        .set_file_name(format!("{}.json", file_stem(name)))
        .save_file()
    else {
        return "export cancelled".to_owned();
    };
    match export_json(name, data)
        .map_err(|e| e.to_string())
        .and_then(|text| std::fs::write(&path, text).map_err(|e| e.to_string()))
    {
        Ok(()) => format!("exported to {}", path.display()),
        Err(error) => format!("export failed: {error}"),
    }
}

/// Asks for a file and stores the preset it contains (any kind). `Ok(None)` = cancelled.
fn import(core: &AppCore) -> Result<Option<(String, Kind)>, String> {
    let Some(path): Option<PathBuf> = rfd::FileDialog::new()
        .add_filter("Preset (StealSeries or Sonar JSON)", &["json"])
        .pick_file()
    else {
        return Ok(None);
    };
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let (name, data) = import_json(&text).map_err(|e| e.to_string())?;
    let kind = Kind::of(&data);
    core.update_config(|c| c.presets.insert(name.clone(), data));
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
