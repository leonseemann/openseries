//! Menu-bar icon with quick toggles (`docs/architecture/app.md` §2).
//!
//! Menu clicks are handled on the event handler's thread directly through [`AppCore`], so they
//! work while the window is hidden. The window only mirrors the check marks in [`Tray::sync`].

use std::sync::Arc;

use eframe::egui;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::core::{AppCore, Snapshot};
use crate::i18n::tr;
use crate::platform::NativeWindow;
use crate::update::UpdateState;
use ss_update::Platform;

/// Edge length of the generated template icon in pixels (menu bar icons are 22 pt @2x).
const ICON_SIZE: u32 = 44;

/// The menu-bar icon and the items whose state follows the app.
pub struct Tray {
    _icon: TrayIcon,
    status: MenuItem,
    audiomix: CheckMenuItem,
    mic: CheckMenuItem,
    reconnect: MenuItem,
    update: MenuItem,
    open: MenuItem,
    quit: MenuItem,
    /// The language the fixed items are labelled in.
    language: String,
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
        let audiomix =
            CheckMenuItem::new(tr!("tray-audiomix"), true, config.audiomix.enabled, None);
        let mic = CheckMenuItem::new(tr!("tray-virtual-mic"), true, config.mic.enabled, None);
        let reconnect = MenuItem::new(tr!("tray-reconnect-usb"), true, None);
        let update = MenuItem::new(tr!("tray-check-updates"), true, None);
        let open = MenuItem::new(tr!("tray-open"), true, None);
        let quit = MenuItem::new(tr!("tray-quit"), true, None);

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
            reconnect,
            update,
            open,
            quit,
            language: crate::i18n::current(),
        })
    }

    /// Mirrors the current state into the menu (main thread, once per frame).
    pub fn sync(&mut self, snapshot: &Snapshot) {
        self.relabel();
        self.audiomix.set_checked(snapshot.config.audiomix.enabled);
        self.mic.set_checked(snapshot.config.mic.enabled);
        let primary = snapshot.devices.primary();
        let text = match primary {
            _ if !snapshot.active => tr!("tray-paused"),
            None => tr!("tray-no-device"),
            Some(d) if d.linked() == Some(false) => {
                tr!("tray-device-off", device = d.info.name)
            }
            Some(d) => d.main_battery().map_or_else(
                || tr!("tray-device-connected", device = d.info.name),
                |b| {
                    tr!(
                        "tray-device-battery",
                        device = d.info.name,
                        percent = b.percent()
                    )
                },
            ),
        };
        self.status.set_text(text);
        let state = &snapshot.update;
        let (label, enabled) = match state {
            UpdateState::Checking => (tr!("tray-checking-updates"), false),
            UpdateState::Downloading { update, .. } => (
                tr!(
                    "tray-downloading-update",
                    version = update.version.to_string()
                ),
                false,
            ),
            UpdateState::Installing(version) => (
                tr!("tray-installing-update", version = version.to_string()),
                false,
            ),
            _ => state.update().map_or_else(
                || (tr!("tray-check-updates"), true),
                |update| {
                    (
                        tr!("tray-install-update", version = update.version.to_string()),
                        true,
                    )
                },
            ),
        };
        self.update.set_text(label);
        self.update.set_enabled(enabled);
    }

    /// Re-labels the fixed items after a language change (the others are set in `sync`).
    fn relabel(&mut self) {
        let language = crate::i18n::current();
        if language == self.language {
            return;
        }
        self.audiomix.set_text(tr!("tray-audiomix"));
        self.mic.set_text(tr!("tray-virtual-mic"));
        self.reconnect.set_text(tr!("tray-reconnect-usb"));
        self.open.set_text(tr!("tray-open"));
        self.quit.set_text(tr!("tray-quit"));
        self.language = language;
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
