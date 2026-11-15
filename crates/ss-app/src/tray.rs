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
        let status = MenuItem::new("StealSeries", false, None);
        let audiomix = CheckMenuItem::new("AudioMix", true, config.audiomix.enabled, None);
        let mic = CheckMenuItem::new("Virtual microphone", true, config.mic.enabled, None);
        let reconnect = MenuItem::new("Reconnect USB (fixes crackling)", true, None);
        let update = MenuItem::new(CHECK_FOR_UPDATES, true, None);
        let open = MenuItem::new("Open StealSeries…", true, None);
        let quit = MenuItem::new("Quit StealSeries", true, None);

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
            .with_tooltip("StealSeries")
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
                core.reset_headset_usb();
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
        let text = match snapshot.headset.status {
            _ if !snapshot.active => "Paused · base station not connected".to_owned(),
            Some(status) if status.connected => status.headset_battery.map_or_else(
                || "Headset connected".to_owned(),
                |b| format!("Headset · ≈{} %", b.percent()),
            ),
            _ => "Headset not connected".to_owned(),
        };
        self.status.set_text(text);
        let state = &snapshot.update;
        let (label, enabled) = match state {
            UpdateState::Checking => ("Checking for updates…".to_owned(), false),
            UpdateState::Downloading { update, .. } => (
                format!("Downloading StealSeries {}…", update.version),
                false,
            ),
            UpdateState::Installing(version) => {
                (format!("Installing StealSeries {version}…"), false)
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
    [0x5B, 0x7C, 0xFA]
};

/// The tray icon.
fn template_icon() -> Result<Icon, tray_icon::BadIcon> {
    Icon::from_rgba(headphone_rgba(ICON_SIZE, TRAY_COLOR), ICON_SIZE, ICON_SIZE)
}

/// A simple headphone glyph (RGBA, `size`×`size`, transparent background) in `color`; used for
/// the tray and the window icon.
#[must_use]
pub fn headphone_rgba(size: u32, color: [u8; 3]) -> Vec<u8> {
    let mut rgba = vec![0_u8; (size * size * 4) as usize];
    #[allow(clippy::cast_precision_loss)] // small icon sizes
    let extent = size as f32;
    let (centre, outer, inner) = (extent / 2.0, extent * 0.42, extent * 0.32);
    for y in 0..size {
        for x in 0..size {
            #[allow(clippy::cast_precision_loss)]
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let distance = ((fx - centre).powi(2) + (fy - centre).powi(2)).sqrt();
            let headband = fy <= centre && distance <= outer && distance >= inner;
            let cups = fy >= centre - extent * 0.05
                && fy <= extent * 0.9
                && ((fx - (centre - outer)).abs() <= extent * 0.09
                    || (fx - (centre + outer)).abs() <= extent * 0.09);
            if headband || cups {
                let index = ((y * size + x) * 4) as usize;
                rgba[index..index + 3].copy_from_slice(&color);
                rgba[index + 3] = 255;
            }
        }
    }
    rgba
}
