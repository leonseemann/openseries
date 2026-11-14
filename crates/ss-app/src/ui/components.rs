//! Reusable building blocks of every page (`docs/architecture/ui-design.md` §2, §5).

use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Sense, Stroke, TextStyle};

use super::theme::{self, CARD_RADIUS, CARD_TITLE, Palette};
use crate::audio::supervisor::ServiceStatus;

/// Inner padding of cards.
const CARD_PADDING: i8 = 14;
/// Vertical gap after a card.
const CARD_GAP: f32 = 10.0;
/// Width reserved for the description column of a settings row before the control.
const ROW_MIN_LABEL_WIDTH: f32 = 200.0;
/// Sidebar entry: icon size, horizontal centre of the icon, start of the label.
const NAV_ICON_SIZE: f32 = 15.0;
const NAV_ICON_CENTRE_X: f32 = 22.0;
const NAV_TEXT_X: f32 = 40.0;
/// Top offset that centres a one-line row label against a 30 px control.
const ROW_SINGLE_LINE_OFFSET: f32 = 3.0;

/// Page title and a one-line explanation.
pub fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    let p = theme::palette(ui);
    ui.label(
        RichText::new(title)
            .text_style(TextStyle::Heading)
            .color(p.text),
    );
    ui.label(RichText::new(subtitle).color(p.text_secondary));
    ui.add_space(8.0);
}

/// A card with an optional title; the body fills the available width.
pub fn card<R>(
    ui: &mut egui::Ui,
    title: Option<&str>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let p = theme::palette(ui);
    let result = egui::Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(Margin::same(CARD_PADDING))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(title) = title {
                ui.label(
                    RichText::new(title)
                        .text_style(TextStyle::Name(CARD_TITLE.into()))
                        .color(p.text),
                );
                ui.add_space(2.0);
            }
            add_contents(ui)
        })
        .inner;
    ui.add_space(CARD_GAP);
    result
}

/// A settings row: label (and optional description) on the left, `control` on the right.
pub fn row<R>(
    ui: &mut egui::Ui,
    label: &str,
    description: Option<&str>,
    control: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let p = theme::palette(ui);
    ui.horizontal(|ui| {
        let label_width = (ui.available_width() * 0.45).max(ROW_MIN_LABEL_WIDTH);
        ui.vertical(|ui| {
            ui.set_width(label_width.min(ui.available_width()));
            if description.is_none() {
                // Centre a single line against the (taller) control on the right.
                ui.add_space(ROW_SINGLE_LINE_OFFSET);
            }
            ui.label(RichText::new(label).color(p.text));
            if let Some(description) = description {
                ui.add(
                    egui::Label::new(RichText::new(description).small().color(p.text_secondary))
                        .wrap(),
                );
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), control)
            .inner
    })
    .inner
}

/// A thin divider between rows inside a card.
pub fn divider(ui: &mut egui::Ui) {
    let p = theme::palette(ui);
    ui.add_space(2.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, p.border);
    ui.add_space(2.0);
}

/// An iOS-style on/off switch. Returns `true` when it was toggled.
pub fn switch(ui: &mut egui::Ui, on: &mut bool) -> bool {
    let p = theme::palette(ui);
    let size = egui::vec2(36.0, 20.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool_responsive(response.id, *on);
        let track = lerp_color(p.control_hover, p.accent, t);
        let radius = rect.height() / 2.0;
        ui.painter().rect_filled(rect, radius, track);
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.expand(2.0),
                radius + 2.0,
                Stroke::new(2.0_f32, p.accent),
                egui::StrokeKind::Outside,
            );
        }
        let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        ui.painter()
            .circle_filled(egui::pos2(x, rect.center().y), radius - 3.0, Color32::WHITE);
    }
    response.changed()
}

/// A row of mutually exclusive choices. Returns the newly picked value.
pub fn segmented<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    current: Option<T>,
    options: &[(T, &str)],
) -> Option<T> {
    let p = theme::palette(ui);
    let mut picked = None;
    egui::Frame::new()
        .fill(p.control)
        .corner_radius(CornerRadius::same(theme::CONTROL_RADIUS))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                // Inside a right-aligned row egui lays out right to left; keep reading order.
                let ordered: Vec<&(T, &str)> = if right_to_left(ui) {
                    options.iter().rev().collect()
                } else {
                    options.iter().collect()
                };
                for &&(value, label) in &ordered {
                    let selected = current == Some(value);
                    let text =
                        RichText::new(label).color(if selected { p.accent_text } else { p.text });
                    let button = egui::Button::new(text)
                        .fill(if selected {
                            p.accent
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(theme::CONTROL_RADIUS - 2))
                        .min_size(egui::vec2(0.0, 22.0));
                    if ui.add(button).clicked() && !selected {
                        picked = Some(value);
                    }
                }
            });
        });
    picked
}

/// Colour of a state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Working.
    Good,
    /// Needs attention.
    Warning,
    /// Broken.
    Error,
    /// Off / inactive.
    Neutral,
}

impl Tone {
    fn color(self, p: &Palette) -> Color32 {
        match self {
            Self::Good => p.success,
            Self::Warning => p.warning,
            Self::Error => p.error,
            Self::Neutral => p.text_secondary,
        }
    }
}

/// A coloured dot followed by text (colour never carries the meaning alone).
pub fn status(ui: &mut egui::Ui, tone: Tone, text: &str) {
    let p = theme::palette(ui);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let dot = |ui: &mut egui::Ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), Sense::hover());
            ui.painter()
                .circle_filled(rect.center(), 4.0, tone.color(&p));
        };
        // Dot first in reading order, also inside right-aligned rows.
        if right_to_left(ui) {
            ui.label(RichText::new(text).color(p.text));
            dot(ui);
        } else {
            dot(ui);
            ui.label(RichText::new(text).color(p.text));
        }
    });
}

/// Whether widgets added to `ui` go from right to left (inside a right-aligned row).
fn right_to_left(ui: &egui::Ui) -> bool {
    ui.layout().main_dir() == egui::Direction::RightToLeft
}

/// The tone and a short word for a service state.
#[must_use]
pub fn service_tone(status: &ServiceStatus) -> (Tone, &'static str) {
    match status {
        ServiceStatus::Off => (Tone::Neutral, "Off"),
        ServiceStatus::Running => (Tone::Good, "On"),
        ServiceStatus::Blocked(_) => (Tone::Warning, "Paused"),
        ServiceStatus::Failed(_) => (Tone::Error, "Error"),
    }
}

/// An inline message box (warning or error) with an explanation.
pub fn banner(ui: &mut egui::Ui, tone: Tone, text: &str) {
    let p = theme::palette(ui);
    let color = tone.color(&p);
    egui::Frame::new()
        .fill(color.gamma_multiply(0.14))
        .stroke(Stroke::new(1.0_f32, color.gamma_multiply(0.5)))
        .corner_radius(CornerRadius::same(theme::CONTROL_RADIUS))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(text).color(p.text));
        });
}

/// Shows the problem behind a paused or failed service, if any.
pub fn service_problem(ui: &mut egui::Ui, status: &ServiceStatus) {
    match status {
        ServiceStatus::Blocked(reason) => banner(ui, Tone::Warning, &format!("Paused: {reason}")),
        ServiceStatus::Failed(error) => banner(ui, Tone::Error, error),
        ServiceStatus::Off | ServiceStatus::Running => {}
    }
}

/// A horizontal battery gauge with the percentage next to it.
pub fn battery(ui: &mut egui::Ui, percent: Option<u8>, width: f32) {
    let p = theme::palette(ui);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 6.0), Sense::hover());
        ui.painter().rect_filled(rect, 3.0, p.control_hover);
        if let Some(percent) = percent {
            let fraction = f32::from(percent.min(100)) / 100.0;
            let color = match percent {
                0..=20 => p.error,
                21..=40 => p.warning,
                _ => p.success,
            };
            let mut fill = rect;
            fill.set_width(rect.width() * fraction);
            ui.painter().rect_filled(fill, 3.0, color);
        }
        ui.label(
            RichText::new(percent.map_or_else(|| "–".to_owned(), |v| format!("{v} %")))
                .color(p.text),
        );
    });
}

/// A sidebar navigation entry. Returns its response (clicked = navigate).
pub fn nav_item(ui: &mut egui::Ui, icon: &str, label: &str, selected: bool) -> egui::Response {
    let p = theme::palette(ui);
    let height = 32.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
    let fill = if selected {
        p.accent.gamma_multiply(0.18)
    } else if response.hovered() {
        p.control
    } else {
        Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(theme::CONTROL_RADIUS), fill);
    if selected {
        let bar = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(0.0, 9.0),
            egui::vec2(3.0, height - 18.0),
        );
        ui.painter().rect_filled(bar, 2.0, p.accent);
    }
    let text_color = if selected { p.text } else { p.text_secondary };
    let font = egui::FontId::proportional(13.5);
    let painter = ui.painter();
    painter.text(
        rect.left_center() + egui::vec2(NAV_TEXT_X, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        font.clone(),
        text_color,
    );
    // Icons come from a different font (emoji) with other metrics, so centring both on the line
    // height looks off. Centre the icon's actual glyph shape on the optical middle of the text
    // (halfway between baseline and cap height, measured on "H").
    let reference = painter.layout_no_wrap("H".to_owned(), font, text_color);
    let optical_middle =
        rect.center().y + (reference.mesh_bounds.center().y - reference.rect.center().y);
    let icon_galley = painter.layout_no_wrap(
        icon.to_owned(),
        egui::FontId::proportional(NAV_ICON_SIZE),
        if selected { p.accent } else { p.text_secondary },
    );
    let target = egui::pos2(rect.left() + NAV_ICON_CENTRE_X, optical_middle);
    let origin = target - icon_galley.mesh_bounds.center().to_vec2();
    painter.galley(origin, icon_galley, text_color);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, label)
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A slider that spans the given width and reports a new value only when the user changed it.
pub fn slider(
    ui: &mut egui::Ui,
    value: f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
) -> Option<f32> {
    let mut edited = value;
    // Whole percent read better; dB values keep one decimal.
    let decimals = usize::from(suffix.trim() != "%");
    let response = ui.add(
        egui::Slider::new(&mut edited, range)
            .suffix(suffix)
            .fixed_decimals(decimals),
    );
    (response.changed() && (edited - value).abs() > f32::EPSILON).then_some(edited)
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let mix = |x: u8, y: u8| {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // within 0..=255
        let v = (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
        v
    };
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}
