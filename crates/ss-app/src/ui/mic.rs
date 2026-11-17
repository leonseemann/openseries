//! Microphone page: virtual microphone, its source and the processing chain
//! (`docs/architecture/ui-design.md` §4). On systems without a virtual microphone the page
//! explains that instead of showing controls without effect.

use eframe::egui::{self, RichText};
use ss_dsp::{DEFAULT_AUTO_GATE_OFFSET_DB, DEFAULT_DENOISE_MIX, DEFAULT_GATE_THRESHOLD_DB};
use ss_store::config::{GateConfig, MicConfig};
use ss_store::presets::{PresetData, apply_mic};

use super::components::{self, Tone};
use super::{audiomix, presets, theme, widgets};
use crate::audio::supervisor::ServiceStatus;
use crate::core::{AppCore, Snapshot};
use crate::i18n::tr;

/// Gain range of the mic output (dB).
const GAIN_RANGE_DB: std::ops::RangeInclusive<f32> = -12.0..=12.0;

pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    components::page_header(ui, &tr!("mic-title"), &tr!("mic-subtitle"));
    if !crate::platform::VIRTUAL_MIC_SUPPORTED {
        unavailable(ui);
        return;
    }
    let config = &snapshot.config.mic;
    switch_card(ui, core, snapshot);
    source_card(ui, core, snapshot);
    processing_card(ui, core, snapshot, config);
}

fn unavailable(ui: &mut egui::Ui) {
    components::card(ui, Some(&tr!("mic-unavailable-title")), |ui| {
        let p = theme::palette(ui);
        ui.label(RichText::new(tr!("mic-unavailable-body")).color(p.text_secondary));
        ui.add_space(4.0);
        ui.label(RichText::new(tr!("mic-unavailable-headset-hint")).color(p.text_secondary));
    });
}

fn switch_card(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let config = &snapshot.config.mic;
    components::card(ui, None, |ui| {
        let mut enabled = config.enabled;
        components::row(
            ui,
            &tr!("mic-virtual"),
            Some(&tr!("mic-virtual-hint")),
            |ui| {
                if components::switch(ui, &mut enabled) {
                    core.update_config(|c| c.mic.enabled = enabled);
                }
                let (tone, word) = components::service_tone(&snapshot.audio.mic);
                components::status(ui, tone, &word);
            },
        );
        components::service_problem(ui, &snapshot.audio.mic);
        if let ServiceStatus::Failed(error) = &snapshot.audio.mic
            && error.contains("driver")
        {
            components::banner(ui, Tone::Neutral, &tr!("mic-install-driver-hint"));
        }
    });
}

fn source_card(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let config = &snapshot.config.mic;
    components::card(ui, Some(&tr!("mic-source")), |ui| {
        components::row(ui, &tr!("mic-input"), Some(&tr!("mic-input-hint")), |ui| {
            if let Some(uid) = widgets::device_combo(
                ui,
                "mic-input",
                config.input_device_uid.as_deref(),
                &snapshot.audio.inputs,
            ) {
                core.update_config(|c| c.mic.input_device_uid = uid.into_uid());
            }
        });
        components::divider(ui);
        let mut as_default = config.set_as_default_input;
        components::row(
            ui,
            &tr!("mic-as-default"),
            Some(&tr!("mic-as-default-hint")),
            |ui| {
                if components::switch(ui, &mut as_default) {
                    core.update_config(|c| c.mic.set_as_default_input = as_default);
                }
            },
        );
        if let Some(error) = &snapshot.audio.default_input_error {
            components::banner(
                ui,
                Tone::Warning,
                &tr!("mic-default-input-error", error = error.as_str()),
            );
        }
        egui::CollapsingHeader::new(tr!("mic-backup"))
            .id_salt("backup-inputs")
            .default_open(!config.input_fallback.is_empty())
            .show(ui, |ui| {
                if let Some(list) = widgets::fallback_list(
                    ui,
                    "input",
                    &config.input_fallback,
                    &snapshot.audio.inputs,
                ) {
                    core.update_config(|c| c.mic.input_fallback = list);
                }
            });
    });
}

fn processing_card(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot, config: &MicConfig) {
    components::card(ui, Some(&tr!("mic-processing")), |ui| {
        let mut denoise = config.denoise.is_some();
        components::row(
            ui,
            &tr!("mic-denoise"),
            Some(&tr!("mic-denoise-hint")),
            |ui| {
                if components::switch(ui, &mut denoise) {
                    core.update_config(|c| c.mic.denoise = denoise.then_some(DEFAULT_DENOISE_MIX));
                }
                if let Some(mix) = config.denoise {
                    ui.spacing_mut().slider_width = 160.0;
                    if let Some(new) = components::slider(ui, mix * 100.0, 0.0..=100.0, " %") {
                        core.update_config(|c| c.mic.denoise = Some(new / 100.0));
                    }
                }
            },
        );
        components::divider(ui);
        gate_row(ui, core, config);
        components::divider(ui);
        let mut stabilizer = config.stabilizer;
        components::row(
            ui,
            &tr!("mic-stabilizer"),
            Some(&tr!("mic-stabilizer-hint")),
            |ui| {
                if components::switch(ui, &mut stabilizer) {
                    core.update_config(|c| c.mic.stabilizer = stabilizer);
                }
            },
        );
        components::divider(ui);
        components::row(ui, &tr!("mic-gain"), Some(&tr!("mic-gain-hint")), |ui| {
            if let Some(db) = components::slider(ui, config.gain_db, GAIN_RANGE_DB, " dB") {
                core.update_config(|c| c.mic.gain_db = db);
            }
        });
        if let Some(eq) = audiomix::eq_editor(ui, "mic-eq", &config.eq) {
            core.update_config(|c| c.mic.eq = eq);
        }
        egui::CollapsingHeader::new(tr!("mic-presets"))
            .id_salt("mic-presets")
            .show(ui, |ui| {
                let preset = presets::bar(ui, core, snapshot, presets::Kind::Mic, "mic", || {
                    Some(PresetData::mic_of(config))
                });
                if let Some(PresetData::Mic(preset)) = preset {
                    core.update_config(|c| apply_mic(&mut c.mic, &preset));
                }
            });
    });
}

fn gate_row(ui: &mut egui::Ui, core: &AppCore, config: &MicConfig) {
    components::row(ui, &tr!("mic-gate"), Some(&tr!("mic-gate-hint")), |ui| {
        if let Some(choice) = components::segmented(
            ui,
            Some(gate_mode(config.gate)),
            &[
                (GateChoice::Off, tr!("mic-gate-off").as_str()),
                (GateChoice::Automatic, tr!("mic-gate-automatic").as_str()),
                (GateChoice::Manual, tr!("mic-gate-manual").as_str()),
            ],
        ) {
            core.update_config(|c| {
                c.mic.gate = match choice {
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
    });
    match config.gate {
        Some(GateConfig::Manual { threshold_db }) => {
            components::row(
                ui,
                &tr!("mic-gate-threshold"),
                Some(&tr!("mic-gate-threshold-hint")),
                |ui| {
                    if let Some(db) = components::slider(ui, threshold_db, -80.0..=-20.0, " dBFS") {
                        core.update_config(|c| {
                            c.mic.gate = Some(GateConfig::Manual { threshold_db: db });
                        });
                    }
                },
            );
        }
        Some(GateConfig::Automatic { offset_db }) => {
            components::row(
                ui,
                &tr!("mic-gate-sensitivity"),
                Some(&tr!("mic-gate-sensitivity-hint")),
                |ui| {
                    if let Some(db) = components::slider(ui, offset_db, 3.0..=20.0, " dB") {
                        core.update_config(|c| {
                            c.mic.gate = Some(GateConfig::Automatic { offset_db: db });
                        });
                    }
                },
            );
        }
        None => {}
    }
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
