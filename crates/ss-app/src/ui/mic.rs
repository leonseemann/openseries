//! Microphone page: virtual mic on/off, source and the mic chain.

use eframe::egui;
use ss_dsp::{DEFAULT_AUTO_GATE_OFFSET_DB, DEFAULT_DENOISE_MIX, DEFAULT_GATE_THRESHOLD_DB};
use ss_store::config::{GateConfig, MicConfig};

use ss_store::presets::{PresetData, apply_mic};

use super::{audiomix, presets, widgets};
use crate::audio::supervisor::ServiceStatus;
use crate::core::{AppCore, Snapshot};

/// Gain range of the mic output (dB).
const GAIN_RANGE_DB: std::ops::RangeInclusive<f32> = -12.0..=12.0;

pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let config = &snapshot.config.mic;
    ui.heading("Virtual microphone");
    let mut enabled = config.enabled;
    if ui
        .checkbox(&mut enabled, "Feed \"StealSeries Microphone\"")
        .changed()
    {
        core.update_config(|c| c.mic.enabled = enabled);
    }
    ui.label(
        egui::RichText::new(
            "Select \"StealSeries Microphone\" as input in Discord, Zoom, OBS … to get the \
             processed voice.",
        )
        .weak(),
    );
    widgets::status_detail(ui, &snapshot.audio.mic);
    if let ServiceStatus::Failed(error) = &snapshot.audio.mic {
        if error.contains("driver") {
            ui.label("Install the driver once: scripts/macos/hal-driver.sh install");
        }
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Source");
        if let Some(uid) = widgets::device_combo(
            ui,
            "mic-input",
            config.input_device_uid.as_deref(),
            &snapshot.audio.inputs,
        ) {
            core.update_config(|c| c.mic.input_device_uid = uid.into_uid());
        }
    });

    ui.add_space(8.0);
    ui.heading("Processing");
    let preset = presets::bar(ui, core, snapshot, presets::Kind::Mic, "mic", || {
        Some(PresetData::mic_of(config))
    });
    if let Some(PresetData::Mic(preset)) = preset {
        core.update_config(|c| apply_mic(&mut c.mic, &preset));
    }
    processing(ui, core, config);
}

fn processing(ui: &mut egui::Ui, core: &AppCore, config: &MicConfig) {
    ui.horizontal(|ui| {
        let mut denoise = config.denoise.is_some();
        if ui.checkbox(&mut denoise, "Noise suppression").changed() {
            core.update_config(|c| c.mic.denoise = denoise.then_some(DEFAULT_DENOISE_MIX));
        }
        if let Some(mix) = config.denoise {
            if let Some(new) = widgets::slider(ui, mix * 100.0, 0.0..=100.0, "Strength", " %") {
                core.update_config(|c| c.mic.denoise = Some(new / 100.0));
            }
        }
    });

    ui.horizontal(|ui| {
        let mode = gate_mode(config.gate);
        let mut selected = mode;
        egui::ComboBox::from_id_salt("gate-mode")
            .selected_text(gate_label(mode))
            .show_ui(ui, |ui| {
                for option in [GateChoice::Off, GateChoice::Automatic, GateChoice::Manual] {
                    ui.selectable_value(&mut selected, option, gate_label(option));
                }
            });
        if selected != mode {
            core.update_config(|c| {
                c.mic.gate = match selected {
                    GateChoice::Off => None,
                    GateChoice::Automatic => Some(GateConfig::Automatic {
                        offset_db: DEFAULT_AUTO_GATE_OFFSET_DB,
                    }),
                    GateChoice::Manual => Some(GateConfig::Manual {
                        threshold_db: DEFAULT_GATE_THRESHOLD_DB,
                    }),
                };
            });
        }
        match config.gate {
            Some(GateConfig::Manual { threshold_db }) => {
                if let Some(db) =
                    widgets::slider(ui, threshold_db, -80.0..=-20.0, "Threshold", " dBFS")
                {
                    core.update_config(|c| {
                        c.mic.gate = Some(GateConfig::Manual { threshold_db: db });
                    });
                }
            }
            Some(GateConfig::Automatic { offset_db }) => {
                if let Some(db) = widgets::slider(ui, offset_db, 3.0..=20.0, "Above noise", " dB") {
                    core.update_config(|c| {
                        c.mic.gate = Some(GateConfig::Automatic { offset_db: db });
                    });
                }
            }
            None => {}
        }
    });

    let mut stabilizer = config.stabilizer;
    if ui
        .checkbox(
            &mut stabilizer,
            "Volume stabilizer (evens out quiet and loud speech)",
        )
        .changed()
    {
        core.update_config(|c| c.mic.stabilizer = stabilizer);
    }
    if let Some(db) = widgets::slider(ui, config.gain_db, GAIN_RANGE_DB, "Output gain", " dB") {
        core.update_config(|c| c.mic.gain_db = db);
    }
    if let Some(eq) = audiomix::eq_editor(ui, "mic-eq", &config.eq) {
        core.update_config(|c| c.mic.eq = eq);
    }
    ui.label(
        egui::RichText::new("Changing processing restarts the microphone feed for a moment.")
            .weak(),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateChoice {
    Off,
    Automatic,
    Manual,
}

const fn gate_mode(gate: Option<GateConfig>) -> GateChoice {
    match gate {
        None => GateChoice::Off,
        Some(GateConfig::Automatic { .. }) => GateChoice::Automatic,
        Some(GateConfig::Manual { .. }) => GateChoice::Manual,
    }
}

const fn gate_label(choice: GateChoice) -> &'static str {
    match choice {
        GateChoice::Off => "Noise gate off",
        GateChoice::Automatic => "Noise gate automatic",
        GateChoice::Manual => "Noise gate manual",
    }
}
