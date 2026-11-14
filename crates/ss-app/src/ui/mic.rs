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

/// Gain range of the mic output (dB).
const GAIN_RANGE_DB: std::ops::RangeInclusive<f32> = -12.0..=12.0;

pub fn show(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    components::page_header(
        ui,
        "Microphone",
        "A cleaner voice for Discord, Zoom and OBS: noise suppression, gate, stabilizer and EQ.",
    );
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
    components::card(ui, Some("Not available on this computer"), |ui| {
        let p = theme::palette(ui);
        ui.label(
            RichText::new(
                "The processed \"StealSeries Microphone\" needs a virtual audio device. It is \
                 available on macOS. On Windows it would require a paid, signed audio driver, \
                 and StealSeries stays free — so this page is empty here.",
            )
            .color(p.text_secondary),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Microphone volume, sidetone and the mute light are on the Headset page.",
            )
            .color(p.text_secondary),
        );
    });
}

fn switch_card(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let config = &snapshot.config.mic;
    components::card(ui, None, |ui| {
        let mut enabled = config.enabled;
        components::row(
            ui,
            "Virtual microphone",
            Some("Select \"StealSeries Microphone\" as the input in your apps."),
            |ui| {
                if components::switch(ui, &mut enabled) {
                    core.update_config(|c| c.mic.enabled = enabled);
                }
                let (tone, word) = components::service_tone(&snapshot.audio.mic);
                components::status(ui, tone, word);
            },
        );
        components::service_problem(ui, &snapshot.audio.mic);
        if let ServiceStatus::Failed(error) = &snapshot.audio.mic {
            if error.contains("driver") {
                components::banner(
                    ui,
                    Tone::Neutral,
                    "Install the microphone driver once: Settings → Show getting started → \
                     Install driver.",
                );
            }
        }
    });
}

fn source_card(ui: &mut egui::Ui, core: &AppCore, snapshot: &Snapshot) {
    let config = &snapshot.config.mic;
    components::card(ui, Some("Source"), |ui| {
        components::row(ui, "Microphone", Some("Usually your headset."), |ui| {
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
            "Use as system input",
            Some("Make \"StealSeries Microphone\" the default input while it runs."),
            |ui| {
                if components::switch(ui, &mut as_default) {
                    core.update_config(|c| c.mic.set_as_default_input = as_default);
                }
            },
        );
        if let Some(error) = &snapshot.audio.default_input_error {
            components::banner(ui, Tone::Warning, &format!("Default input: {error}"));
        }
        egui::CollapsingHeader::new("Backup microphones")
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
    components::card(ui, Some("Processing"), |ui| {
        let mut denoise = config.denoise.is_some();
        components::row(
            ui,
            "Noise suppression",
            Some("Removes fans, keyboard hum and background noise."),
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
            "Volume stabilizer",
            Some("Evens out quiet and loud speech."),
            |ui| {
                if components::switch(ui, &mut stabilizer) {
                    core.update_config(|c| c.mic.stabilizer = stabilizer);
                }
            },
        );
        components::divider(ui);
        components::row(
            ui,
            "Output gain",
            Some("Louder or quieter overall."),
            |ui| {
                if let Some(db) = components::slider(ui, config.gain_db, GAIN_RANGE_DB, " dB") {
                    core.update_config(|c| c.mic.gain_db = db);
                }
            },
        );
        if let Some(eq) = audiomix::eq_editor(ui, "mic-eq", &config.eq) {
            core.update_config(|c| c.mic.eq = eq);
        }
        egui::CollapsingHeader::new("Presets")
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
    components::row(
        ui,
        "Noise gate",
        Some("Silence between sentences. Automatic adapts to your room."),
        |ui| {
            if let Some(choice) = components::segmented(
                ui,
                Some(gate_mode(config.gate)),
                &[
                    (GateChoice::Off, "Off"),
                    (GateChoice::Automatic, "Automatic"),
                    (GateChoice::Manual, "Manual"),
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
        },
    );
    match config.gate {
        Some(GateConfig::Manual { threshold_db }) => {
            components::row(ui, "Threshold", Some("Quieter sounds are muted."), |ui| {
                if let Some(db) = components::slider(ui, threshold_db, -80.0..=-20.0, " dBFS") {
                    core.update_config(|c| {
                        c.mic.gate = Some(GateConfig::Manual { threshold_db: db });
                    });
                }
            });
        }
        Some(GateConfig::Automatic { offset_db }) => {
            components::row(
                ui,
                "Sensitivity",
                Some("How far above the room noise your voice must be."),
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
