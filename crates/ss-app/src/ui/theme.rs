//! Look of the window: colours, fonts, sizes (`docs/architecture/ui-design.md` §3).

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle, Theme, ThemePreference, Visuals,
};

/// Font family used for titles (semibold system font where available).
pub const STRONG_FAMILY: &str = "ui-strong";
/// Text style of card titles.
pub const CARD_TITLE: &str = "card-title";
/// Corner radius of cards.
pub const CARD_RADIUS: u8 = 12;
/// Corner radius of buttons, inputs and other controls.
pub const CONTROL_RADIUS: u8 = 8;

/// Colour tokens of one theme.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub sidebar: Color32,
    pub card: Color32,
    pub border: Color32,
    pub control: Color32,
    pub control_hover: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
}

/// Dark theme tokens.
pub const DARK: Palette = Palette {
    background: Color32::from_rgb(0x12, 0x13, 0x17),
    sidebar: Color32::from_rgb(0x17, 0x18, 0x1D),
    card: Color32::from_rgb(0x1D, 0x1F, 0x25),
    border: Color32::from_rgb(0x2A, 0x2D, 0x35),
    control: Color32::from_rgb(0x2A, 0x2D, 0x36),
    control_hover: Color32::from_rgb(0x34, 0x38, 0x43),
    text: Color32::from_rgb(0xE8, 0xEA, 0xEF),
    text_secondary: Color32::from_rgb(0xA0, 0xA6, 0xB2),
    accent: Color32::from_rgb(0x5B, 0x7C, 0xFA),
    accent_text: Color32::WHITE,
    success: Color32::from_rgb(0x3F, 0xB9, 0x7F),
    warning: Color32::from_rgb(0xE3, 0xA4, 0x3B),
    error: Color32::from_rgb(0xE5, 0x60, 0x6B),
};

/// Light theme tokens.
pub const LIGHT: Palette = Palette {
    background: Color32::from_rgb(0xF3, 0xF4, 0xF7),
    sidebar: Color32::from_rgb(0xEB, 0xED, 0xF1),
    card: Color32::WHITE,
    border: Color32::from_rgb(0xDF, 0xE2, 0xE8),
    control: Color32::from_rgb(0xEC, 0xEE, 0xF2),
    control_hover: Color32::from_rgb(0xE1, 0xE4, 0xEA),
    text: Color32::from_rgb(0x1B, 0x1D, 0x22),
    text_secondary: Color32::from_rgb(0x5D, 0x63, 0x70),
    accent: Color32::from_rgb(0x3D, 0x5A, 0xF1),
    accent_text: Color32::WHITE,
    success: Color32::from_rgb(0x1E, 0x8E, 0x5A),
    warning: Color32::from_rgb(0xB2, 0x6F, 0x00),
    error: Color32::from_rgb(0xC9, 0x3D, 0x48),
};

/// The palette that matches the current visuals.
#[must_use]
pub fn palette(ui: &egui::Ui) -> Palette {
    if ui.visuals().dark_mode { DARK } else { LIGHT }
}

/// The palette of the context's current theme (outside a `Ui`).
#[must_use]
pub fn palette_for(ctx: &egui::Context) -> Palette {
    if ctx.style().visuals.dark_mode {
        DARK
    } else {
        LIGHT
    }
}

/// Installs fonts, styles and both themes; follows the OS theme unless `STEALSERIES_THEME`
/// (dark/light, a development hook for screenshots) says otherwise.
pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_visuals_of(Theme::Dark, visuals(&DARK, true));
    ctx.set_visuals_of(Theme::Light, visuals(&LIGHT, false));
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::new(24.0, strong())),
            (
                TextStyle::Name(CARD_TITLE.into()),
                FontId::new(16.0, strong()),
            ),
            (TextStyle::Body, FontId::proportional(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Small, FontId::proportional(12.5)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();
        let spacing = &mut style.spacing;
        spacing.item_spacing = egui::vec2(10.0, 10.0);
        spacing.button_padding = egui::vec2(14.0, 6.0);
        spacing.interact_size = egui::vec2(36.0, 30.0);
        spacing.slider_width = 240.0;
        spacing.combo_width = 240.0;
        spacing.icon_width = 18.0;
        spacing.menu_margin = Margin::same(8);
        style.interaction.tooltip_delay = 0.4;
    });
    let preference = match std::env::var("STEALSERIES_THEME").as_deref() {
        Ok("dark") => ThemePreference::Dark,
        Ok("light") => ThemePreference::Light,
        _ => ThemePreference::System,
    };
    ctx.set_theme(preference);
}

fn strong() -> FontFamily {
    FontFamily::Name(STRONG_FAMILY.into())
}

fn visuals(p: &Palette, dark: bool) -> Visuals {
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.panel_fill = p.background;
    v.window_fill = p.card;
    v.window_stroke = Stroke::new(1.0_f32, p.border);
    v.window_corner_radius = CornerRadius::same(CARD_RADIUS);
    v.menu_corner_radius = CornerRadius::same(CONTROL_RADIUS);
    v.extreme_bg_color = if dark { p.background } else { p.control };
    v.faint_bg_color = p.control;
    v.code_bg_color = p.control;
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warning;
    v.error_fg_color = p.error;
    v.selection.bg_fill = p.accent;
    v.selection.stroke = Stroke::new(1.0_f32, p.accent_text);
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Circle;
    v.striped = false;

    let radius = CornerRadius::same(CONTROL_RADIUS);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.card;
    w.noninteractive.weak_bg_fill = p.card;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.noninteractive.corner_radius = radius;
    for (state, fill) in [
        (&mut w.inactive, p.control),
        (&mut w.hovered, p.control_hover),
        (&mut w.active, p.control_hover),
        (&mut w.open, p.control_hover),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::NONE;
        state.fg_stroke = Stroke::new(1.5_f32, p.text);
        state.corner_radius = radius;
        state.expansion = 0.0;
    }
    w.hovered.bg_stroke = Stroke::new(1.0_f32, p.border);
    w.active.bg_stroke = Stroke::new(1.0_f32, p.accent);
    v
}

/// Uses the system UI font where we know it (Segoe UI on Windows); egui's bundled fonts stay as
/// fallbacks (and provide the icon glyphs).
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let regular = system_font(&["segoeui.ttf"]);
    let semibold = system_font(&["seguisb.ttf", "segoeuib.ttf"]);
    let proportional = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut strong_family = proportional.clone();
    if let Some(bytes) = regular {
        fonts
            .font_data
            .insert("system-ui".into(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "system-ui".into());
        strong_family.insert(0, "system-ui".into());
    }
    if let Some(bytes) = semibold {
        fonts.font_data.insert(
            "system-ui-strong".into(),
            Arc::new(FontData::from_owned(bytes)),
        );
        strong_family.insert(0, "system-ui-strong".into());
    }
    fonts.families.insert(strong(), strong_family);
    ctx.set_fonts(fonts);
}

/// The first readable font file from the Windows fonts folder (`None` elsewhere).
fn system_font(names: &[&str]) -> Option<Vec<u8>> {
    if !cfg!(target_os = "windows") {
        return None;
    }
    let dir = std::env::var_os("WINDIR").map_or_else(
        || std::path::PathBuf::from(r"C:\Windows"),
        std::path::PathBuf::from,
    );
    names
        .iter()
        .find_map(|name| std::fs::read(dir.join("Fonts").join(name)).ok())
}
