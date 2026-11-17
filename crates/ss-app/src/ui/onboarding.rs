//! "Getting started" window (onboarding; `docs/architecture/app.md` §2). Shown until the user
//! clicks Done (`app.onboarding_done`), reopened from Settings.
//!
//! [`steps`] derives the checklist from the snapshot (pure, unit-tested); [`show`] draws it with
//! one action per step.

use eframe::egui;

use super::{components, theme};
use crate::audio::supervisor::ServiceStatus;
use crate::core::{AppCore, Snapshot};
use crate::i18n::tr;

/// One checklist entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Base station connected via USB.
    BaseStation,
    /// Headset linked to the base station.
    Headset,
    /// AudioMix on and running.
    AudioMix,
    /// Virtual-mic driver installed (macOS).
    MicDriver,
    /// Virtual microphone running (macOS).
    VirtualMic,
    /// Start at login (optional).
    Autostart,
}

impl Step {
    fn title(self) -> String {
        match self {
            Self::BaseStation => tr!("onboarding-device"),
            Self::Headset => tr!("onboarding-headset"),
            Self::AudioMix => tr!("onboarding-audiomix"),
            Self::MicDriver => tr!("onboarding-mic-driver"),
            Self::VirtualMic => tr!("onboarding-virtual-mic"),
            Self::Autostart => tr!("onboarding-autostart"),
        }
    }

    fn hint(self) -> String {
        match self {
            Self::BaseStation => tr!("onboarding-device-hint"),
            Self::Headset => tr!("onboarding-headset-hint"),
            Self::AudioMix => {
                if cfg!(target_os = "macos") {
                    tr!("onboarding-audiomix-hint-macos")
                } else {
                    tr!("onboarding-audiomix-hint")
                }
            }
            Self::MicDriver => tr!("onboarding-mic-driver-hint"),
            Self::VirtualMic => tr!("onboarding-virtual-mic-hint"),
            Self::Autostart => tr!("onboarding-autostart-hint"),
        }
    }
}

/// The checklist and whether each step is done.
#[must_use]
pub fn steps(
    snapshot: &Snapshot,
    virtual_mic: bool,
    driver_installed: Option<bool>,
) -> Vec<(Step, bool)> {
    let devices = &snapshot.devices;
    let config = &snapshot.config;
    let mut list = vec![(Step::BaseStation, devices.any())];
    // Only devices with a wireless part need to be switched on separately.
    if devices.devices.is_empty() || devices.devices.iter().any(|d| d.linked().is_some()) {
        list.push((
            Step::Headset,
            devices.devices.iter().any(|d| d.linked() == Some(true)),
        ));
    }
    list.extend([(
        Step::AudioMix,
        config.audiomix.enabled && snapshot.audio.mix == ServiceStatus::Running,
    )]);
    if virtual_mic {
        list.push((Step::MicDriver, driver_installed == Some(true)));
        list.push((
            Step::VirtualMic,
            config.mic.enabled && snapshot.audio.mic == ServiceStatus::Running,
        ));
    }
    list.push((Step::Autostart, config.app.autostart));
    list
}

/// Draws the window while onboarding is not finished.
pub fn show(ctx: &egui::Context, core: &AppCore, snapshot: &Snapshot) {
    if snapshot.config.app.onboarding_done {
        return;
    }
    let virtual_mic = crate::platform::VIRTUAL_MIC_SUPPORTED;
    let driver = crate::platform::mic_driver_installed();
    let id = egui::Id::new("onboarding-message");
    // A modal dims the window behind it, so the checklist has the user's full attention.
    egui::Modal::new(egui::Id::new("onboarding")).show(ctx, |ui| {
        let p = theme::palette(ui);
        ui.set_width(DIALOG_WIDTH);
        ui.label(
            egui::RichText::new(tr!("onboarding-welcome"))
                .text_style(egui::TextStyle::Heading)
                .color(p.text),
        );
        ui.label(egui::RichText::new(tr!("onboarding-intro")).color(p.text_secondary));
        ui.add_space(10.0);
        for (index, (step, done)) in steps(snapshot, virtual_mic, driver).into_iter().enumerate() {
            if index > 0 {
                components::divider(ui);
            }
            ui.horizontal(|ui| {
                step_marker(ui, index + 1, done);
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() - ACTION_WIDTH);
                    ui.label(egui::RichText::new(step.title()).color(p.text));
                    if !done {
                        ui.label(
                            egui::RichText::new(step.hint())
                                .small()
                                .color(p.text_secondary),
                        );
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    action(ui, core, snapshot, step, done, id);
                });
            });
        }
        if let Some(message) = ui.data(|d| d.get_temp::<String>(id)) {
            ui.add_space(6.0);
            components::banner(ui, components::Tone::Warning, &message);
        }
        ui.add_space(12.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let done =
                egui::Button::new(egui::RichText::new(tr!("onboarding-done")).color(p.accent_text))
                    .fill(p.accent)
                    .min_size(egui::vec2(96.0, 32.0));
            if ui.add(done).clicked() {
                core.update_config(|c| c.app.onboarding_done = true);
            }
        });
    });
}

/// Width of the dialog.
const DIALOG_WIDTH: f32 = 560.0;
/// Room reserved for a step's action button.
const ACTION_WIDTH: f32 = 140.0;

/// A numbered circle that turns into a check mark when the step is done.
fn step_marker(ui: &mut egui::Ui, number: usize, done: bool) {
    let p = theme::palette(ui);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::hover());
    let centre = rect.center();
    if done {
        ui.painter().circle_filled(centre, 12.0, p.success);
        ui.painter().text(
            centre,
            egui::Align2::CENTER_CENTER,
            "✔",
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
        );
    } else {
        ui.painter()
            .circle_stroke(centre, 11.5, egui::Stroke::new(1.5_f32, p.text_secondary));
        ui.painter().text(
            centre,
            egui::Align2::CENTER_CENTER,
            number.to_string(),
            egui::FontId::proportional(13.0),
            p.text_secondary,
        );
    }
}

/// The button next to a step.
fn action(
    ui: &mut egui::Ui,
    core: &AppCore,
    snapshot: &Snapshot,
    step: Step,
    done: bool,
    message_id: egui::Id,
) {
    let config = &snapshot.config;
    match step {
        Step::BaseStation | Step::Headset => {}
        Step::AudioMix => {
            if !config.audiomix.enabled && ui.button(tr!("onboarding-turn-on")).clicked() {
                core.update_config(|c| c.audiomix.enabled = true);
            }
            if let ServiceStatus::Failed(error) = &snapshot.audio.mix {
                ui.colored_label(ui.visuals().warn_fg_color, error);
            }
        }
        Step::MicDriver => {
            if !done && ui.button(tr!("onboarding-install-driver")).clicked() {
                let message = match crate::platform::install_mic_driver() {
                    Ok(()) => tr!("onboarding-driver-installed"),
                    Err(error) => tr!("onboarding-driver-failed", error = error),
                };
                ui.data_mut(|d| d.insert_temp(message_id, message));
            }
        }
        Step::VirtualMic => {
            if !config.mic.enabled && ui.button(tr!("onboarding-turn-on")).clicked() {
                core.update_config(|c| c.mic.enabled = true);
            }
        }
        Step::Autostart => {
            if !done && ui.button(tr!("onboarding-turn-on")).clicked() {
                core.update_config(|c| c.app.autostart = true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_install_has_nothing_done() {
        let list = steps(&Snapshot::default(), true, Some(false));
        assert_eq!(list.len(), 6);
        assert!(list.iter().all(|(_, done)| !done));
    }

    #[test]
    fn mic_steps_only_where_a_virtual_mic_exists() {
        let list = steps(&Snapshot::default(), false, None);
        assert!(
            !list
                .iter()
                .any(|(s, _)| matches!(s, Step::MicDriver | Step::VirtualMic))
        );
    }

    #[test]
    fn audiomix_counts_only_when_running() {
        let mut snapshot = Snapshot::default();
        snapshot.config.audiomix.enabled = true;
        snapshot.audio.mix = ServiceStatus::Failed("no permission".to_owned());
        let done = |s: &Snapshot| {
            steps(s, false, None)
                .into_iter()
                .find(|(step, _)| *step == Step::AudioMix)
                .map(|(_, done)| done)
        };
        assert_eq!(done(&snapshot), Some(false));
        snapshot.audio.mix = ServiceStatus::Running;
        assert_eq!(done(&snapshot), Some(true));
    }
}
