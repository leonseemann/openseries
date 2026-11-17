//! The egui window (`docs/architecture/app.md` §2, design: `docs/architecture/ui-design.md`).
//! A pure view: it renders the latest [`Snapshot`] and turns edits into [`AppCore`] calls.
//! Closing the window only hides it; the app keeps running in the menu bar / tray.

mod audiomix;
mod components;
mod device;
mod mic;
mod onboarding;
mod overview;
mod presets;
mod settings;
mod theme;
mod widgets;

use std::sync::Arc;

use eframe::egui::{self, RichText};

use self::components::{Tone, service_tone};
use crate::core::{AppCore, Snapshot};
use crate::hotkeys::Hotkeys;
use crate::i18n::tr;
use crate::tray::Tray;
use crate::update::UpdateState;
use ss_store::config::HotkeysConfig;

/// Initial window size in points.
const WINDOW_SIZE: [f32; 2] = [920.0, 660.0];
/// Smallest usable window.
const MIN_WINDOW_SIZE: [f32; 2] = [720.0, 480.0];
/// Width of the navigation sidebar.
const SIDEBAR_WIDTH: f32 = 196.0;
/// Widest the page content grows; wider windows centre it.
const CONTENT_MAX_WIDTH: f32 = 720.0;
/// Padding around the page content.
const PAGE_PADDING: f32 = 20.0;
/// Size of the window icon in pixels.
const WINDOW_ICON_SIZE: u32 = 64;

/// The pages of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Overview,
    /// The page of the connected device at this index (ADR 0008).
    Device(usize),
    AudioMix,
    Mic,
    Settings,
}

impl Page {
    /// Fixed pages after the device list.
    const TOOLS: [Self; 3] = [Self::AudioMix, Self::Mic, Self::Settings];

    /// Whether the page has anything to offer on this OS (no virtual mic on Windows).
    const fn available(self) -> bool {
        !matches!(self, Self::Mic) || crate::platform::VIRTUAL_MIC_SUPPORTED
    }

    const fn icon(self) -> &'static str {
        match self {
            Self::Overview => "🏠",
            Self::Device(_) => "🎧",
            Self::AudioMix => "🔊",
            Self::Mic => "🎤",
            Self::Settings => "⚙",
        }
    }

    fn label(self) -> String {
        match self {
            Self::Overview => tr!("nav-overview"),
            Self::Device(_) => tr!("nav-device"),
            Self::AudioMix => tr!("nav-audiomix"),
            Self::Mic => tr!("nav-mic"),
            Self::Settings => tr!("nav-settings"),
        }
    }

    /// Start page from `OPENSERIES_START_PAGE` (development hook for screenshots).
    fn from_env() -> Self {
        match std::env::var("OPENSERIES_START_PAGE").as_deref() {
            Ok("headset" | "device") => Self::Device(0),
            Ok("audiomix") => Self::AudioMix,
            Ok("mic") if Self::Mic.available() => Self::Mic,
            Ok("settings") => Self::Settings,
            _ => Self::Overview,
        }
    }
}

/// The eframe application.
pub struct App {
    core: Arc<AppCore>,
    tray: Option<Tray>,
    hotkeys: Option<Hotkeys>,
    hotkey_problems: Vec<String>,
    /// The hotkey config currently registered; `None` while suspended for recording.
    applied_hotkeys: Option<HotkeysConfig>,
    hotkey_capture: settings::HotkeyCapture,
    page: Page,
    /// Shows unassigned apps that are not playing in the AudioMix list.
    show_idle_apps: bool,
}

impl App {
    /// Creates the window state; the tray icon is created here because macOS needs the running
    /// event loop for it.
    pub fn new(cc: &eframe::CreationContext<'_>, core: Arc<AppCore>) -> Self {
        theme::install(&cc.egui_ctx);
        let tray = match Tray::new(
            &cc.egui_ctx,
            Arc::clone(&core),
            crate::platform::NativeWindow::of(cc),
        ) {
            Ok(tray) => Some(tray),
            Err(error) => {
                eprintln!("warning: no menu-bar icon: {error}");
                None
            }
        };
        let hotkey_config = core.config().app.hotkeys;
        let (hotkeys, hotkey_problems) = match Hotkeys::register(&hotkey_config, Arc::clone(&core))
        {
            Ok((hotkeys, problems)) => (Some(hotkeys), problems),
            Err(error) => (
                None,
                vec![tr!("nav-hotkeys-unavailable", error = error.to_string())],
            ),
        };
        Self {
            core,
            tray,
            hotkeys,
            hotkey_problems,
            applied_hotkeys: Some(hotkey_config),
            hotkey_capture: settings::HotkeyCapture::default(),
            page: Page::from_env(),
            show_idle_apps: false,
        }
    }

    /// Keeps the registered hotkeys in line with the config; suspends them while the editor
    /// records a combination (the OS would otherwise swallow it).
    /// While the base station is missing (`active == false`) they are unregistered too, so other
    /// apps can use the key combinations (`docs/architecture/app.md` §8).
    fn sync_hotkeys(&mut self, wanted: &HotkeysConfig, active: bool) {
        let Some(hotkeys) = self.hotkeys.as_mut() else {
            return;
        };
        if self.hotkey_capture.recording.is_some() || !active {
            if self.applied_hotkeys.take().is_some() {
                hotkeys.suspend();
            }
        } else if self.applied_hotkeys.as_ref() != Some(wanted) {
            self.hotkey_problems = hotkeys.apply(wanted);
            self.applied_hotkeys = Some(wanted.clone());
        }
    }

    /// Native window options.
    pub fn options() -> eframe::NativeOptions {
        let icon = egui::IconData {
            rgba: crate::icon::app_icon_rgba(WINDOW_ICON_SIZE),
            width: WINDOW_ICON_SIZE,
            height: WINDOW_ICON_SIZE,
        };
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(window_size_from_env().unwrap_or(WINDOW_SIZE))
                .with_min_inner_size(MIN_WINDOW_SIZE)
                .with_title("OpenSeries")
                .with_icon(Arc::new(icon)),
            ..eframe::NativeOptions::default()
        }
    }

    fn sidebar(&mut self, ctx: &egui::Context, snapshot: &Snapshot) {
        let p = theme::palette_for(ctx);
        egui::SidePanel::left("navigation")
            .exact_width(SIDEBAR_WIDTH)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(p.sidebar)
                    .inner_margin(egui::Margin::symmetric(10, 14)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new("🎧").size(18.0).color(p.accent));
                    ui.label(
                        RichText::new("OpenSeries")
                            .text_style(egui::TextStyle::Name(theme::CARD_TITLE.into()))
                            .color(p.text),
                    );
                });
                ui.add_space(12.0);
                ui.spacing_mut().item_spacing.y = 2.0;
                if components::nav_item(
                    ui,
                    Page::Overview.icon(),
                    &Page::Overview.label(),
                    self.page == Page::Overview,
                )
                .clicked()
                {
                    self.page = Page::Overview;
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(tr!("nav-devices-heading"))
                            .small()
                            .color(p.text_secondary),
                    );
                });
                if snapshot.devices.devices.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(tr!("nav-no-devices"))
                                .small()
                                .color(p.text_secondary),
                        );
                    });
                }
                for (index, d) in snapshot.devices.devices.iter().enumerate() {
                    let page = Page::Device(index);
                    if components::nav_item(
                        ui,
                        device::class_icon(d.info.class),
                        d.info.name,
                        self.page == page,
                    )
                    .clicked()
                    {
                        self.page = page;
                    }
                }
                ui.add_space(8.0);
                for page in Page::TOOLS.into_iter().filter(|p| p.available()) {
                    if components::nav_item(ui, page.icon(), &page.label(), self.page == page)
                        .clicked()
                    {
                        self.page = page;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.add_space(4.0);
                    if crate::platform::VIRTUAL_MIC_SUPPORTED {
                        let (tone, word) = service_tone(&snapshot.audio.mic);
                        sidebar_status(ui, tone, &tr!("nav-status-mic", state = word));
                    }
                    let (tone, word) = service_tone(&snapshot.audio.mix);
                    sidebar_status(ui, tone, &tr!("nav-status-audiomix", state = word));
                    for d in snapshot.devices.devices.iter().rev() {
                        let (tone, text) = device_summary(d);
                        sidebar_status(ui, tone, &text);
                    }
                    if snapshot.devices.devices.is_empty() {
                        sidebar_status(ui, Tone::Error, &tr!("nav-no-device-connected"));
                    }
                });
            });
    }

    fn page(&mut self, ui: &mut egui::Ui, snapshot: &Snapshot) {
        let problems: Vec<&String> = [&snapshot.config_warning, &snapshot.save_error]
            .into_iter()
            .flatten()
            .chain(&self.hotkey_problems)
            .collect();
        for problem in problems {
            components::banner(ui, Tone::Warning, problem);
            ui.add_space(8.0);
        }
        if !snapshot.active {
            components::banner(ui, Tone::Neutral, &tr!("nav-paused-banner"));
            ui.add_space(8.0);
        }
        if let (UpdateState::Available(update), false) =
            (&snapshot.update, self.page == Page::Settings)
        {
            ui.horizontal(|ui| {
                components::status(
                    ui,
                    Tone::Good,
                    &tr!("nav-update-available", version = update.version.to_string()),
                );
                if ui.small_button(tr!("nav-install-update")).clicked() {
                    self.core.install_update();
                }
                if ui.small_button(tr!("nav-update-details")).clicked() {
                    self.page = Page::Settings;
                }
            });
            ui.add_space(8.0);
        }
        match self.page {
            Page::Overview => overview::show(ui, &self.core, snapshot, &mut self.page),
            Page::Device(index) => device::show(ui, &self.core, snapshot, index),
            Page::AudioMix => {
                audiomix::show(ui, &self.core, snapshot, &mut self.show_idle_apps);
            }
            Page::Mic => mic::show(ui, &self.core, snapshot),
            Page::Settings => {
                settings::show(ui, &self.core, snapshot, &mut self.hotkey_capture);
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            // Keep running in the menu bar / tray.
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        let snapshot = self.core.snapshot();
        crate::i18n::apply(snapshot.config.app.language.as_deref());
        if let Some(tray) = self.tray.as_mut() {
            tray.sync(&snapshot);
        }
        self.sync_hotkeys(&snapshot.config.app.hotkeys, snapshot.active);
        onboarding::show(ctx, &self.core, &snapshot);

        self.sidebar(ctx, &snapshot);
        let p = theme::palette_for(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.background))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let width = ui.available_width();
                        let content = (width - 2.0 * PAGE_PADDING).clamp(0.0, CONTENT_MAX_WIDTH);
                        let side = ((width - content) / 2.0).max(0.0);
                        ui.add_space(PAGE_PADDING - 8.0);
                        ui.horizontal(|ui| {
                            ui.add_space(side);
                            ui.vertical(|ui| {
                                ui.set_width(content);
                                self.page(ui, &snapshot);
                            });
                        });
                        ui.add_space(PAGE_PADDING);
                    });
            });
    }
}

fn sidebar_status(ui: &mut egui::Ui, tone: Tone, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        components::status(ui, tone, text);
    });
}

/// Tone and text of a device's line in the sidebar.
fn device_summary(d: &crate::devices::DeviceSnapshot) -> (Tone, String) {
    let word = device::class_word(d.info.class);
    if !d.ready {
        return (Tone::Neutral, tr!("nav-device-reading", device = word));
    }
    match (d.linked(), d.main_battery()) {
        (Some(false), _) => (Tone::Neutral, tr!("nav-device-off", device = word)),
        (_, Some(battery)) => {
            let percent = battery.percent();
            let tone = if percent <= crate::alerts::LOW_BATTERY_PERCENT {
                Tone::Warning
            } else {
                Tone::Good
            };
            (
                tone,
                tr!("nav-device-battery", device = word, percent = percent),
            )
        }
        _ => (Tone::Good, tr!("nav-device-connected", device = word)),
    }
}

/// Window size from `OPENSERIES_WINDOW_SIZE` ("1000x1400"; development hook for screenshots).
fn window_size_from_env() -> Option<[f32; 2]> {
    let value = std::env::var("OPENSERIES_WINDOW_SIZE").ok()?;
    let (width, height) = value.split_once('x')?;
    Some([width.trim().parse().ok()?, height.trim().parse().ok()?])
}
