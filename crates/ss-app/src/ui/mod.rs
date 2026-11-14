//! The egui window (`docs/architecture/app.md` §2). A pure view: it renders the latest
//! [`Snapshot`] and turns edits into [`AppCore`] calls. Closing the window only hides it; the
//! app keeps running in the menu bar.

mod audiomix;
mod headset;
mod mic;
mod settings;
mod widgets;

use std::sync::Arc;

use eframe::egui;

use crate::audio::supervisor::ServiceStatus;
use crate::core::{AppCore, Snapshot};
use crate::hotkeys::Hotkeys;
use crate::tray::Tray;

/// Initial window size in points.
const WINDOW_SIZE: [f32; 2] = [820.0, 620.0];

/// The pages of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    AudioMix,
    Mic,
    Headset,
    Settings,
}

/// The eframe application.
pub struct App {
    core: Arc<AppCore>,
    tray: Option<Tray>,
    _hotkeys: Option<Hotkeys>,
    hotkey_problems: Vec<String>,
    tab: Tab,
    /// Shows unassigned apps that are not playing in the AudioMix list.
    show_idle_apps: bool,
}

impl App {
    /// Creates the window state; the tray icon is created here because macOS needs the running
    /// event loop for it.
    pub fn new(cc: &eframe::CreationContext<'_>, core: Arc<AppCore>) -> Self {
        let tray = match Tray::new(&cc.egui_ctx, Arc::clone(&core)) {
            Ok(tray) => Some(tray),
            Err(error) => {
                eprintln!("warning: no menu-bar icon: {error}");
                None
            }
        };
        let (hotkeys, hotkey_problems) =
            match Hotkeys::register(&core.config().app.hotkeys, Arc::clone(&core)) {
                Ok((hotkeys, problems)) => (Some(hotkeys), problems),
                Err(error) => (None, vec![format!("hotkeys unavailable: {error}")]),
            };
        Self {
            core,
            tray,
            _hotkeys: hotkeys,
            hotkey_problems,
            tab: Tab::AudioMix,
            show_idle_apps: false,
        }
    }

    /// Native window options.
    pub fn options() -> eframe::NativeOptions {
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(WINDOW_SIZE)
                .with_min_inner_size([640.0, 480.0])
                .with_title("StealSeries"),
            ..eframe::NativeOptions::default()
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            // Keep running in the menu bar.
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        let snapshot = self.core.snapshot();
        if let Some(tray) = self.tray.as_mut() {
            tray.sync(&snapshot);
        }

        egui::TopBottomPanel::top("status").show(ctx, |ui| {
            ui.add_space(4.0);
            status_bar(ui, &snapshot);
            for problem in &self.hotkey_problems {
                ui.colored_label(ui.visuals().warn_fg_color, problem);
            }
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::AudioMix, "AudioMix");
                ui.selectable_value(&mut self.tab, Tab::Mic, "Microphone");
                ui.selectable_value(&mut self.tab, Tab::Headset, "Headset");
                ui.selectable_value(&mut self.tab, Tab::Settings, "Settings");
            });
            ui.add_space(2.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                Tab::AudioMix => {
                    audiomix::show(ui, &self.core, &snapshot, &mut self.show_idle_apps);
                }
                Tab::Mic => mic::show(ui, &self.core, &snapshot),
                Tab::Headset => headset::show(ui, &self.core, &snapshot),
                Tab::Settings => settings::show(ui, &self.core, &snapshot),
            });
        });
    }
}

/// One-line summary of headset, AudioMix and mic.
fn status_bar(ui: &mut egui::Ui, snapshot: &Snapshot) {
    ui.horizontal_wrapped(|ui| {
        let headset = &snapshot.headset;
        let headset_text = match headset.status {
            Some(status) if status.connected => match status.headset_battery {
                Some(battery) => format!("Headset · {} %", battery.percent()),
                None => "Headset · connected".to_owned(),
            },
            Some(_) => "Headset · base station only".to_owned(),
            None if headset.base_station => "Headset · reading…".to_owned(),
            None => "Headset · not connected".to_owned(),
        };
        widgets::pill(
            ui,
            &headset_text,
            headset.status.is_some_and(|s| s.connected),
        );
        widgets::pill(
            ui,
            &format!("AudioMix · {}", status_word(&snapshot.audio.mix)),
            snapshot.audio.mix == ServiceStatus::Running,
        );
        widgets::pill(
            ui,
            &format!("Mic · {}", status_word(&snapshot.audio.mic)),
            snapshot.audio.mic == ServiceStatus::Running,
        );
    });
    for problem in [&snapshot.config_warning, &snapshot.save_error]
        .into_iter()
        .flatten()
    {
        ui.colored_label(ui.visuals().warn_fg_color, problem);
    }
}

/// Short word for a service status.
fn status_word(status: &ServiceStatus) -> &'static str {
    match status {
        ServiceStatus::Off => "off",
        ServiceStatus::Running => "on",
        ServiceStatus::Blocked(_) => "paused",
        ServiceStatus::Failed(_) => "error",
    }
}
