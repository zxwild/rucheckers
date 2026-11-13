//! Colours and styling: calm warm wood tones, without bright accents.

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

/// Window background around the board.
pub const BACKGROUND: Color32 = Color32::from_rgb(0xD6, 0xC9, 0xB2);
/// Background of the move panel.
pub const PANEL: Color32 = Color32::from_rgb(0xEE, 0xE5, 0xD6);
/// Background of dialogs and of the move list.
pub const PAPER: Color32 = Color32::from_rgb(0xF6, 0xF0, 0xE4);
/// Main text.
pub const TEXT: Color32 = Color32::from_rgb(0x3A, 0x2D, 0x22);
/// Secondary text.
pub const TEXT_WEAK: Color32 = Color32::from_rgb(0x7B, 0x6A, 0x58);
/// Lines and borders of controls.
pub const BORDER: Color32 = Color32::from_rgb(0xC4, 0xB4, 0x9B);
const BUTTON: Color32 = Color32::from_rgb(0xE2, 0xD6, 0xC1);
const BUTTON_HOVER: Color32 = Color32::from_rgb(0xD8, 0xC9, 0xAF);
const BUTTON_ACTIVE: Color32 = Color32::from_rgb(0xCB, 0xB9, 0x99);
/// Highlight of the current move in the list.
pub const SELECTION: Color32 = Color32::from_rgb(0xD3, 0xB3, 0x7C);

/// Board frame.
pub const FRAME: Color32 = Color32::from_rgb(0x5E, 0x3F, 0x26);
pub const FRAME_DARK: Color32 = Color32::from_rgb(0x3E, 0x28, 0x17);
pub const FRAME_LIGHT: Color32 = Color32::from_rgb(0x7C, 0x57, 0x36);
/// Coordinate labels on the frame.
pub const COORDINATES: Color32 = Color32::from_rgb(0xE6, 0xD2, 0xA8);
/// Light and dark squares.
pub const SQUARE_LIGHT: Color32 = Color32::from_rgb(0xE9, 0xD5, 0xAC);
pub const SQUARE_DARK: Color32 = Color32::from_rgb(0xA0, 0x6F, 0x40);

/// Side-to-move mark in the corner of the frame: the colours of White and Black and their outline.
pub const WHITE_FACE: Color32 = Color32::from_rgb(0xF6, 0xEE, 0xDB);
pub const WHITE_OUTLINE: Color32 = Color32::from_rgb(0x8E, 0x7D, 0x5C);
pub const BLACK_FACE: Color32 = Color32::from_rgb(0x4A, 0x38, 0x2D);

/// Highlights: the last move, the selected piece, reachable squares and pieces that must capture.
pub const LAST_MOVE: Color32 = Color32::from_rgba_premultiplied(0x66, 0x5A, 0x34, 0x6E);
pub const SELECTED: Color32 = Color32::from_rgba_premultiplied(0x4A, 0x58, 0x30, 0x70);
pub const TARGET: Color32 = Color32::from_rgba_premultiplied(0xB4, 0xC8, 0x86, 0xDC);
pub const MUST_CAPTURE: Color32 = Color32::from_rgb(0xC8, 0x6A, 0x3A);

/// Sets up the look of the interface.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Light);
    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = PANEL;
    visuals.window_fill = PAPER;
    visuals.extreme_bg_color = PAPER;
    visuals.faint_bg_color = BUTTON;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(6);
    visuals.selection.bg_fill = SELECTION;
    visuals.selection.stroke = Stroke::new(1.0_f32, TEXT);
    visuals.hyperlink_color = TEXT;

    let radius = CornerRadius::same(6);
    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = PANEL;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    widgets.noninteractive.corner_radius = radius;
    widgets.inactive.bg_fill = BUTTON;
    widgets.inactive.weak_bg_fill = BUTTON;
    widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    widgets.inactive.corner_radius = radius;
    widgets.hovered.bg_fill = BUTTON_HOVER;
    widgets.hovered.weak_bg_fill = BUTTON_HOVER;
    widgets.hovered.bg_stroke = Stroke::new(1.0_f32, TEXT_WEAK);
    widgets.hovered.fg_stroke = Stroke::new(1.5_f32, TEXT);
    widgets.hovered.corner_radius = radius;
    widgets.active.bg_fill = BUTTON_ACTIVE;
    widgets.active.weak_bg_fill = BUTTON_ACTIVE;
    widgets.active.bg_stroke = Stroke::new(1.0_f32, TEXT);
    widgets.active.fg_stroke = Stroke::new(1.5_f32, TEXT);
    widgets.active.corner_radius = radius;
    widgets.open.bg_fill = BUTTON_HOVER;
    widgets.open.weak_bg_fill = BUTTON_HOVER;
    widgets.open.bg_stroke = Stroke::new(1.0_f32, TEXT_WEAK);
    widgets.open.fg_stroke = Stroke::new(1.0_f32, TEXT);
    widgets.open.corner_radius = radius;
    ctx.set_visuals_of(egui::Theme::Light, visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        style.spacing.button_padding = egui::vec2(9.0, 5.0);
        style.spacing.interact_size.y = 26.0;
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::new(15.0, FontFamily::Proportional));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::new(21.0, FontFamily::Proportional));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::new(12.5, FontFamily::Proportional));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace));
    });
}
