//! "Getting started" window (onboarding; `docs/architecture/app.md` §2). Shown until the user
//! clicks Done (`app.onboarding_done`), reopened from Settings.
//!
//! [`steps`] derives the checklist from the snapshot (pure, unit-tested); [`show`] draws it with
//! one action per step.

use eframe::egui;

use crate::audio::supervisor::ServiceStatus;
use crate::core::{AppCore, Snapshot};

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
    const fn title(self) -> &'static str {
        match self {
            Self::BaseStation => "Connect the base station",
            Self::Headset => "Turn the headset on",
            Self::AudioMix => "Turn on AudioMix",
            Self::MicDriver => "Install the microphone driver",
            Self::VirtualMic => "Turn on the virtual microphone",
            Self::Autostart => "Start StealSeries at login (optional)",
        }
    }

    const fn hint(self) -> &'static str {
        match self {
            Self::BaseStation => {
                "Plug the base station into this computer by USB (on macOS the USB-1 port)."
            }
            Self::Headset => "Switch the headset on; it links to the base station by itself.",
            Self::AudioMix => {
                if cfg!(target_os = "macos") {
                    "Routes app audio through Game / Chat / Media / Aux with EQ and ChatMix. macOS \
                     asks for System Audio Recording permission (Privacy & Security)."
                } else {
                    "Sets each app's volume by its channel (Game / Chat / Media / Aux) and \
                     ChatMix. Per-channel EQ is available on macOS only."
                }
            }
            Self::MicDriver => {
                "Adds \"StealSeries Microphone\" to macOS. Needs your administrator password; \
                 audio pauses for about a second."
            }
            Self::VirtualMic => {
                "Feeds your processed voice into \"StealSeries Microphone\" — select it in \
                 Discord, Zoom or OBS."
            }
            Self::Autostart => "Keeps the headset settings and AudioMix available after login.",
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
    let headset = &snapshot.headset;
    let config = &snapshot.config;
    let mut list = vec![
        (Step::BaseStation, headset.base_station),
        (
            Step::Headset,
            headset.status.is_some_and(|status| status.connected),
        ),
        (
            Step::AudioMix,
            config.audiomix.enabled && snapshot.audio.mix == ServiceStatus::Running,
        ),
    ];
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
    egui::Window::new("Getting started")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label("A few steps to get everything working. You can reopen this from Settings.");
            ui.add_space(6.0);
            for (step, done) in steps(snapshot, virtual_mic, driver) {
                ui.horizontal(|ui| {
                    ui.label(if done { "✅" } else { "⬜" });
                    ui.label(egui::RichText::new(step.title()).strong());
                    action(ui, core, snapshot, step, done, id);
                });
                if !done {
                    ui.label(egui::RichText::new(step.hint()).weak());
                }
                ui.add_space(4.0);
            }
            if let Some(message) = ui.data(|d| d.get_temp::<String>(id)) {
                ui.colored_label(ui.visuals().warn_fg_color, message);
            }
            ui.separator();
            if ui.button("Done").clicked() {
                core.update_config(|c| c.app.onboarding_done = true);
            }
        });
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
            if !config.audiomix.enabled && ui.button("Turn on").clicked() {
                core.update_config(|c| c.audiomix.enabled = true);
            }
            if let ServiceStatus::Failed(error) = &snapshot.audio.mix {
                ui.colored_label(ui.visuals().warn_fg_color, error);
            }
        }
        Step::MicDriver => {
            if !done && ui.button("Install driver…").clicked() {
                let message = match crate::platform::install_mic_driver() {
                    Ok(()) => "driver installed".to_owned(),
                    Err(error) => format!("driver: {error}"),
                };
                ui.data_mut(|d| d.insert_temp(message_id, message));
            }
        }
        Step::VirtualMic => {
            if !config.mic.enabled && ui.button("Turn on").clicked() {
                core.update_config(|c| c.mic.enabled = true);
            }
        }
        Step::Autostart => {
            if !done && ui.button("Turn on").clicked() {
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
