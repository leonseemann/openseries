//! Menu-bar icon with quick toggles (`docs/architecture/app.md` §2).
//!
//! Menu clicks are handled on the event handler's thread directly through [`AppCore`], so they
//! work while the window is hidden. The window only mirrors the check marks in [`Tray::sync`].

use std::sync::Arc;

use eframe::egui;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::core::{AppCore, Snapshot};

/// Edge length of the generated template icon in pixels (menu bar icons are 22 pt @2x).
const ICON_SIZE: u32 = 44;

/// The menu-bar icon and the items whose state follows the app.
pub struct Tray {
    _icon: TrayIcon,
    status: MenuItem,
    audiomix: CheckMenuItem,
    mic: CheckMenuItem,
}

impl Tray {
    /// Creates the icon and installs the menu handler.
    ///
    /// # Errors
    /// If the menu or icon cannot be created.
    pub fn new(
        ctx: &egui::Context,
        core: Arc<AppCore>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = core.config();
        let status = MenuItem::new("StealSeries", false, None);
        let audiomix = CheckMenuItem::new("AudioMix", true, config.audiomix.enabled, None);
        let mic = CheckMenuItem::new("Virtual microphone", true, config.mic.enabled, None);
        let open = MenuItem::new("Open StealSeries…", true, None);
        let quit = MenuItem::new("Quit StealSeries", true, None);

        let menu = Menu::new();
        menu.append_items(&[
            &status,
            &PredefinedMenuItem::separator(),
            &audiomix,
            &mic,
            &PredefinedMenuItem::separator(),
            &open,
            &quit,
        ])?;
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("StealSeries")
            .with_icon(template_icon()?)
            .with_icon_as_template(true)
            .build()?;

        let (audiomix_id, mic_id, open_id, quit_id) = (
            audiomix.id().clone(),
            mic.id().clone(),
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
            } else if *id == open_id {
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
        })
    }

    /// Mirrors the current state into the menu (main thread, once per frame).
    pub fn sync(&mut self, snapshot: &Snapshot) {
        self.audiomix.set_checked(snapshot.config.audiomix.enabled);
        self.mic.set_checked(snapshot.config.mic.enabled);
        let text = match snapshot.headset.status {
            Some(status) if status.connected => status.headset_battery.map_or_else(
                || "Headset connected".to_owned(),
                |b| format!("Headset · ≈{} %", b.percent()),
            ),
            _ => "Headset not connected".to_owned(),
        };
        self.status.set_text(text);
    }
}

/// A simple headphone glyph as a monochrome template image (macOS tints it for light/dark).
fn template_icon() -> Result<Icon, tray_icon::BadIcon> {
    let size = ICON_SIZE;
    let mut rgba = vec![0_u8; (size * size * 4) as usize];
    #[allow(clippy::cast_precision_loss)] // 44 px
    let extent = size as f32;
    let (centre, outer, inner) = (extent / 2.0, extent * 0.42, extent * 0.32);
    for y in 0..size {
        for x in 0..size {
            #[allow(clippy::cast_precision_loss)]
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let distance = ((fx - centre).powi(2) + (fy - centre).powi(2)).sqrt();
            let headband = fy <= centre && distance <= outer && distance >= inner;
            let cups = fy >= centre - 2.0
                && fy <= extent * 0.9
                && ((fx - (centre - outer)).abs() <= extent * 0.09
                    || (fx - (centre + outer)).abs() <= extent * 0.09);
            if headband || cups {
                let index = ((y * size + x) * 4) as usize;
                rgba[index + 3] = 255; // black, alpha = shape
            }
        }
    }
    Icon::from_rgba(rgba, size, size)
}
