//! Цвета и оформление: спокойные тёплые тона дерева, без ярких акцентов.

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

/// Фон окна вокруг доски.
pub const BACKGROUND: Color32 = Color32::from_rgb(0xD6, 0xC9, 0xB2);
/// Фон панели ходов.
pub const PANEL: Color32 = Color32::from_rgb(0xEE, 0xE5, 0xD6);
/// Фон диалогов и списка ходов.
pub const PAPER: Color32 = Color32::from_rgb(0xF6, 0xF0, 0xE4);
/// Основной текст.
pub const TEXT: Color32 = Color32::from_rgb(0x3A, 0x2D, 0x22);
/// Второстепенный текст.
pub const TEXT_WEAK: Color32 = Color32::from_rgb(0x7B, 0x6A, 0x58);
/// Линии и рамки элементов управления.
pub const BORDER: Color32 = Color32::from_rgb(0xC4, 0xB4, 0x9B);
const BUTTON: Color32 = Color32::from_rgb(0xE2, 0xD6, 0xC1);
const BUTTON_HOVER: Color32 = Color32::from_rgb(0xD8, 0xC9, 0xAF);
const BUTTON_ACTIVE: Color32 = Color32::from_rgb(0xCB, 0xB9, 0x99);
/// Выделение текущего хода в списке.
pub const SELECTION: Color32 = Color32::from_rgb(0xD3, 0xB3, 0x7C);

/// Рамка доски.
pub const FRAME: Color32 = Color32::from_rgb(0x5E, 0x3F, 0x26);
pub const FRAME_DARK: Color32 = Color32::from_rgb(0x3E, 0x28, 0x17);
pub const FRAME_LIGHT: Color32 = Color32::from_rgb(0x7C, 0x57, 0x36);
/// Подписи координат на рамке.
pub const COORDINATES: Color32 = Color32::from_rgb(0xE6, 0xD2, 0xA8);
/// Светлые и тёмные поля.
pub const SQUARE_LIGHT: Color32 = Color32::from_rgb(0xE9, 0xD5, 0xAC);
pub const SQUARE_DARK: Color32 = Color32::from_rgb(0xA0, 0x6F, 0x40);

/// Белая шашка: верх, тень на краю верха, боковая грань, контур и проточки.
pub const WHITE_FACE: Color32 = Color32::from_rgb(0xF6, 0xEE, 0xDB);
pub const WHITE_FACE_SHADE: Color32 = Color32::from_rgb(0xE2, 0xD6, 0xB9);
pub const WHITE_SIDE: Color32 = Color32::from_rgb(0xBF, 0xAE, 0x88);
pub const WHITE_OUTLINE: Color32 = Color32::from_rgb(0x8E, 0x7D, 0x5C);
pub const WHITE_GROOVE: Color32 = Color32::from_rgb(0xC6, 0xB6, 0x92);
/// Чёрная шашка.
pub const BLACK_FACE: Color32 = Color32::from_rgb(0x4A, 0x38, 0x2D);
pub const BLACK_FACE_SHADE: Color32 = Color32::from_rgb(0x30, 0x23, 0x1B);
pub const BLACK_SIDE: Color32 = Color32::from_rgb(0x1E, 0x15, 0x10);
pub const BLACK_OUTLINE: Color32 = Color32::from_rgb(0x14, 0x0D, 0x09);
pub const BLACK_GROOVE: Color32 = Color32::from_rgb(0x66, 0x51, 0x42);
/// Корона дамки на белой и на чёрной шашке.
pub const CROWN_ON_WHITE: Color32 = Color32::from_rgb(0xA8, 0x79, 0x2A);
pub const CROWN_ON_BLACK: Color32 = Color32::from_rgb(0xDD, 0xB8, 0x62);

/// Подсветка последнего хода, выбранной шашки, доступных полей и шашек, обязанных бить.
pub const LAST_MOVE: Color32 = Color32::from_rgba_premultiplied(0x66, 0x5A, 0x34, 0x6E);
pub const SELECTED: Color32 = Color32::from_rgba_premultiplied(0x4A, 0x58, 0x30, 0x70);
pub const TARGET: Color32 = Color32::from_rgba_premultiplied(0xB4, 0xC8, 0x86, 0xDC);
pub const MUST_CAPTURE: Color32 = Color32::from_rgb(0xC8, 0x6A, 0x3A);

/// Настраивает оформление интерфейса.
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
