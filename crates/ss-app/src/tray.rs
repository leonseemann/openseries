//! Menu-bar icon with quick toggles (`docs/architecture/app.md` §2).
//!
//! Menu clicks are handled on the event handler's thread directly through [`AppCore`], so they
//! work while the window is hidden. The window only mirrors the check marks in [`Tray::sync`].

use std::sync::Arc;

use eframe::egui;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::core::{AppCore, Snapshot};
use crate::platform::NativeWindow;
use crate::update::UpdateState;
use ss_update::Platform;

/// Tray item text while no update is on offer.
const CHECK_FOR_UPDATES: &str = "Check for updates";

/// Edge length of the generated template icon in pixels (menu bar icons are 22 pt @2x).
const ICON_SIZE: u32 = 44;

/// The menu-bar icon and the items whose state follows the app.
pub struct Tray {
    _icon: TrayIcon,
    status: MenuItem,
    audiomix: CheckMenuItem,
    mic: CheckMenuItem,
    update: MenuItem,
}

impl Tray {
    /// Creates the icon and installs the menu handler. `window` is the main window's native
    /// handle where the OS needs it to show the window again (Windows).
    ///
    /// # Errors
    /// If the menu or icon cannot be created.
    pub fn new(
        ctx: &egui::Context,
        core: Arc<AppCore>,
        window: Option<NativeWindow>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = core.config();
        let status = MenuItem::new("OpenSeries", false, None);
        let audiomix = CheckMenuItem::new("AudioMix", true, config.audiomix.enabled, None);
        let mic = CheckMenuItem::new("Virtual microphone", true, config.mic.enabled, None);
        let reconnect = MenuItem::new("Reconnect USB (fixes crackling)", true, None);
        let update = MenuItem::new(CHECK_FOR_UPDATES, true, None);
        let open = MenuItem::new("Open OpenSeries…", true, None);
        let quit = MenuItem::new("Quit OpenSeries", true, None);

        let menu = Menu::new();
        menu.append_items(&[
            &status,
            &PredefinedMenuItem::separator(),
            &audiomix,
            &mic,
            &PredefinedMenuItem::separator(),
        ])?;
        if crate::platform::USB_RESET_SUPPORTED {
            menu.append_items(&[&reconnect, &PredefinedMenuItem::separator()])?;
        }
        if Platform::current().is_some() {
            menu.append_items(&[&update, &PredefinedMenuItem::separator()])?;
        }
        menu.append_items(&[&open, &quit])?;
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("OpenSeries")
            .with_icon(template_icon()?)
            .with_icon_as_template(true)
            .build()?;

        let (audiomix_id, mic_id, reconnect_id, update_id, open_id, quit_id) = (
            audiomix.id().clone(),
            mic.id().clone(),
            reconnect.id().clone(),
            update.id().clone(),
            open.id().clone(),
            quit.id().clone(),
        );
        let ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let id = event.id();
            if *id == audiomix_id {
                core.update_config(|c| c.audiomix.enabled = !c.audiomix.enabled);
            } else if *id == mic_id {
                core.update_config(|c| c.mic.enabled = !c.mic.enabled);
            } else if *id == reconnect_id {
                core.reconnect_usb(None);
            } else if *id == update_id {
                // The item reads "Install update …" while one is on offer (see `sync`).
                if core.snapshot().update.update().is_some() {
                    core.install_update();
                } else {
                    core.check_for_updates();
                }
            } else if *id == open_id {
                // On Windows a hidden window runs no UI frames, so the viewport command alone
                // would never be applied; show it natively first.
                if let Some(window) = window {
                    window.show();
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else if *id == quit_id {
                crate::quit(&core);
            }
            ctx.request_repaint();
        }));

        Ok(Self {
            _icon: icon,
            status,
            audiomix,
            mic,
            update,
        })
    }

    /// Mirrors the current state into the menu (main thread, once per frame).
    pub fn sync(&mut self, snapshot: &Snapshot) {
        self.audiomix.set_checked(snapshot.config.audiomix.enabled);
        self.mic.set_checked(snapshot.config.mic.enabled);
        let primary = snapshot.devices.primary();
        let text = match primary {
            _ if !snapshot.active => "Paused · no supported device connected".to_owned(),
            None => "No device connected".to_owned(),
            Some(d) if d.linked() == Some(false) => format!("{} · off", d.info.name),
            Some(d) => d.main_battery().map_or_else(
                || format!("{} connected", d.info.name),
                |b| format!("{} · ≈{} %", d.info.name, b.percent()),
            ),
        };
        self.status.set_text(text);
        let state = &snapshot.update;
        let (label, enabled) = match state {
            UpdateState::Checking => ("Checking for updates…".to_owned(), false),
            UpdateState::Downloading { update, .. } => {
                (format!("Downloading OpenSeries {}…", update.version), false)
            }
            UpdateState::Installing(version) => {
                (format!("Installing OpenSeries {version}…"), false)
            }
            _ => state.update().map_or_else(
                || (CHECK_FOR_UPDATES.to_owned(), true),
                |update| (format!("Install update {}…", update.version), true),
            ),
        };
        self.update.set_text(label);
        self.update.set_enabled(enabled);
    }
}

/// Colour of the tray glyph: black template on macOS (the system tints it); on other systems
/// the accent colour, which stays visible on light and dark taskbars.
const TRAY_COLOR: [u8; 3] = if cfg!(target_os = "macos") {
    [0, 0, 0]
} else {
    crate::icon::ACCENT
};

/// The tray icon.
fn template_icon() -> Result<Icon, tray_icon::BadIcon> {
    Icon::from_rgba(
        crate::icon::headphone_rgba(ICON_SIZE, TRAY_COLOR),
        ICON_SIZE,
        ICON_SIZE,
    )
}
