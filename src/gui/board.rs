//! Отрисовка доски и шашек. Доска рисуется векторными фигурами, шашки — изображениями,
//! рассчитанными под точный размер клетки, поэтому картинка остаётся чёткой при любом
//! размере окна и плотности экрана.

use super::theme;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind,
};
use rucheckers::board::{Color, Kind, Piece, Square};
use rucheckers::ui::fx::{self, DischargeFrame};
use rucheckers::ui::geometry::BoardLayout;
use rucheckers::ui::sprite::{self, PIECE_RADIUS};
use std::f32::consts::TAU;

/// Цвет с изменённой непрозрачностью (0…1).
fn faded(color: Color32, alpha: f32) -> Color32 {
    if alpha >= 0.999 {
        color
    } else {
        color.gamma_multiply(alpha.clamp(0.0, 1.0))
    }
}

/// Детерминированное псевдослучайное число 0…1 по целому ключу (для рисунка древесины).
fn noise(key: u32) -> f32 {
    let mut x = key.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFFFF) as f32 / 65_535.0
}

/// Прямоугольник клетки.
pub fn cell_rect(layout: &BoardLayout, file: u8, rank: u8) -> Rect {
    let (x, y) = layout.cell_origin(file, rank);
    Rect::from_min_size(pos2(x, y), vec2(layout.cell, layout.cell))
}

/// Прямоугольник всей доски вместе с рамкой.
pub fn outer_rect(layout: &BoardLayout) -> Rect {
    Rect::from_min_size(
        pos2(layout.outer_x, layout.outer_y),
        vec2(layout.outer_size, layout.outer_size),
    )
}

/// Центр поля на экране.
pub fn square_center(layout: &BoardLayout, sq: Square) -> Pos2 {
    let (x, y) = layout.square_center(sq);
    pos2(x, y)
}

/// Тонкие прожилки древесины внутри прямоугольника.
fn wood_grain(painter: &Painter, rect: Rect, key: u32, color: Color32, count: u32, pixel: f32) {
    for i in 0..count {
        let k = key.wrapping_mul(31).wrapping_add(i * 7 + 1);
        let y = rect.top() + rect.height() * (0.08 + 0.84 * noise(k));
        let start = rect.left() + rect.width() * 0.45 * noise(k.wrapping_add(11));
        let length = rect.width() * (0.35 + 0.55 * noise(k.wrapping_add(23)));
        let end = (start + length).min(rect.right());
        let alpha = 0.35 + 0.65 * noise(k.wrapping_add(37));
        painter.line_segment(
            [pos2(start, y), pos2(end, y)],
            Stroke::new(pixel, faded(color, alpha)),
        );
    }
}

/// Доска: рамка с координатами и 64 клетки.
pub fn draw_board(painter: &Painter, layout: &BoardLayout, pixels_per_point: f32) {
    let pixel = 1.0 / pixels_per_point.max(0.1);
    let outer = outer_rect(layout);
    let frame = layout.frame();
    let field = Rect::from_min_size(
        pos2(layout.field_x, layout.field_y),
        vec2(layout.field_size(), layout.field_size()),
    );

    // Тень под доской и рамка.
    let radius = CornerRadius::same((frame * 0.28).clamp(2.0, 14.0) as u8);
    painter.rect_filled(
        outer.translate(vec2(0.0, frame * 0.10)).expand(frame * 0.03),
        radius,
        Color32::from_black_alpha(46),
    );
    painter.rect_filled(outer, radius, theme::FRAME);
    wood_grain(
        painter,
        outer.shrink(frame * 0.2),
        7,
        theme::FRAME_DARK.gamma_multiply(0.5),
        46,
        pixel,
    );
    painter.rect_stroke(
        outer,
        radius,
        Stroke::new(pixel.max(1.0), theme::FRAME_DARK),
        StrokeKind::Inside,
    );
    painter.rect_stroke(
        outer.shrink(pixel.max(1.0)),
        radius,
        Stroke::new(pixel.max(1.0), theme::FRAME_LIGHT.gamma_multiply(0.7)),
        StrokeKind::Inside,
    );

    // Клетки.
    for rank in 0..8u8 {
        for file in 0..8u8 {
            let rect = cell_rect(layout, file, rank);
            let dark = (file + rank) % 2 == 0;
            let key = u32::from(rank) * 8 + u32::from(file) + 100;
            if dark {
                painter.rect_filled(rect, 0.0, theme::SQUARE_DARK);
                wood_grain(painter, rect, key, Color32::from_black_alpha(15), 4, pixel);
            } else {
                painter.rect_filled(rect, 0.0, theme::SQUARE_LIGHT);
                wood_grain(
                    painter,
                    rect,
                    key,
                    Color32::from_rgba_unmultiplied(0x7A, 0x5A, 0x30, 16),
                    4,
                    pixel,
                );
            }
        }
    }
    // Кант вокруг игрового поля.
    painter.rect_stroke(
        field,
        0.0,
        Stroke::new((frame * 0.05).max(pixel), theme::FRAME_DARK),
        StrokeKind::Outside,
    );

    // Координаты: буквы сверху и снизу, цифры слева и справа.
    let font_size = frame * 0.52;
    if font_size >= 6.0 {
        let font = FontId::proportional(font_size);
        for i in 0..8u8 {
            let letter = ((b'a' + layout.file_of_column(i)) as char).to_string();
            let digit = ((b'1' + layout.rank_of_row(i)) as char).to_string();
            let along = (f32::from(i) + 0.5) * layout.cell;
            let x = layout.field_x + along;
            let y = layout.field_y + along;
            for cy in [outer.top() + frame / 2.0, outer.bottom() - frame / 2.0] {
                painter.text(
                    pos2(x, cy),
                    Align2::CENTER_CENTER,
                    &letter,
                    font.clone(),
                    theme::COORDINATES,
                );
            }
            for cx in [outer.left() + frame / 2.0, outer.right() - frame / 2.0] {
                painter.text(
                    pos2(cx, y),
                    Align2::CENTER_CENTER,
                    &digit,
                    font.clone(),
                    theme::COORDINATES,
                );
            }
        }
    }
}

/// Заливка поля полупрозрачным цветом (последний ход, выбранная шашка).
pub fn tint_square(painter: &Painter, layout: &BoardLayout, sq: Square, color: Color32) {
    painter.rect_filled(cell_rect(layout, sq.file(), sq.rank()), 0.0, color);
}

/// Отметка поля, на которое можно пойти.
pub fn draw_target(painter: &Painter, layout: &BoardLayout, sq: Square, hovered: bool) {
    let center = square_center(layout, sq);
    let radius = layout.cell * if hovered { 0.20 } else { 0.15 };
    painter.circle_filled(
        center,
        radius + layout.cell * 0.015,
        Color32::from_black_alpha(50),
    );
    painter.circle_filled(center, radius, theme::TARGET);
}

/// Отметка конечного поля серийного взятия: щелчок по нему выполняет все оставшиеся
/// прыжки. От обычного доступного поля отличается кольцом.
pub fn draw_final_target(painter: &Painter, layout: &BoardLayout, sq: Square, hovered: bool) {
    draw_target(painter, layout, sq, hovered);
    let center = square_center(layout, sq);
    let radius = layout.cell * if hovered { 0.30 } else { 0.25 };
    let width = (layout.cell * 0.03).max(1.5);
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(width + 1.5, Color32::from_black_alpha(40)),
    );
    painter.circle_stroke(center, radius, Stroke::new(width, theme::TARGET));
}

/// Свечение: цвет прибавляется к фону и при этом слегка его приглушает. Чистое
/// сложение на светлых тёплых полях даёт почти белый цвет и теряет оттенок.
fn glow(color: [f32; 3], intensity: f32) -> Color32 {
    let intensity = intensity.clamp(0.0, 1.0);
    let channel = |v: f32| (v * intensity * 255.0).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(
        channel(color[0]),
        channel(color[1]),
        channel(color[2]),
        (0.55 * intensity * 255.0).round() as u8,
    )
}

/// Цвет с заданной непрозрачностью, перекрывающий фон.
fn blended(color: [f32; 3], alpha: f32) -> Color32 {
    let alpha = alpha.clamp(0.0, 1.0);
    let channel = |v: f32| (v * alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(
        channel(color[0]),
        channel(color[1]),
        channel(color[2]),
        (alpha * 255.0).round() as u8,
    )
}

/// Круглое свечение: яркое в центре и сходящее на нет к краю.
fn glow_disc(center: Pos2, radius: f32, intensity: f32) -> Shape {
    const SEGMENTS: u32 = 32;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, glow(fx::BOLT_CORE, 0.85 * intensity));
    for ring in [
        (0.4, glow(fx::GLOW, 0.55 * intensity)),
        (1.0, Color32::TRANSPARENT),
    ] {
        for i in 0..SEGMENTS {
            let angle = TAU * i as f32 / SEGMENTS as f32;
            mesh.colored_vertex(center + vec2(angle.cos(), angle.sin()) * radius * ring.0, ring.1);
        }
    }
    for i in 0..SEGMENTS {
        let next = (i + 1) % SEGMENTS;
        let (inner, inner_next) = (1 + i, 1 + next);
        let (outer, outer_next) = (1 + SEGMENTS + i, 1 + SEGMENTS + next);
        mesh.add_triangle(0, inner, inner_next);
        mesh.add_triangle(inner, outer, outer_next);
        mesh.add_triangle(inner, outer_next, inner_next);
    }
    Shape::mesh(mesh)
}

/// Свечение кольцом: ярче всего на радиусе `peak`, сходит на нет к `inner` и `outer`.
fn glow_ring(center: Pos2, inner: f32, peak: f32, outer: f32, intensity: f32) -> Shape {
    const SEGMENTS: u32 = 40;
    let mut mesh = egui::Mesh::default();
    for (radius, color) in [
        (inner, Color32::TRANSPARENT),
        (peak, glow(fx::GLOW, 0.80 * intensity)),
        ((peak + outer) / 2.0, glow(fx::GLOW, 0.26 * intensity)),
        (outer, Color32::TRANSPARENT),
    ] {
        for i in 0..SEGMENTS {
            let angle = TAU * i as f32 / SEGMENTS as f32;
            mesh.colored_vertex(center + vec2(angle.cos(), angle.sin()) * radius, color);
        }
    }
    for ring in 0..3 {
        for i in 0..SEGMENTS {
            let next = (i + 1) % SEGMENTS;
            let (a, a_next) = (ring * SEGMENTS + i, ring * SEGMENTS + next);
            let (b, b_next) = (a + SEGMENTS, a_next + SEGMENTS);
            mesh.add_triangle(a, b, b_next);
            mesh.add_triangle(a, b_next, a_next);
        }
    }
    Shape::mesh(mesh)
}

/// Нижний слой разряда: вспышки на побитых шашках, молния за летящей шашкой и ореол
/// вокруг неё. Рисуется поверх стоящих шашек, но под летящей.
///
/// Всё рисуется обычными треугольниками через ту же аппаратно ускоренную отрисовку
/// (OpenGL), что и остальной интерфейс, поэтому эффект одинаково работает на macOS
/// и Linux и не требует собственных шейдеров.
pub fn draw_lightning(
    painter: &Painter,
    layout: &BoardLayout,
    frame: &DischargeFrame,
    pixels_per_point: f32,
) {
    let cell = layout.cell;
    let pixel = 1.0 / pixels_per_point.max(0.1);
    let on_screen = |p: fx::Point| {
        let (x, y) = layout.cell_to_point(p);
        pos2(x, y)
    };
    let mut shapes: Vec<Shape> = Vec::new();
    for flash in &frame.flashes {
        shapes.push(glow_disc(
            on_screen(flash.center),
            flash.radius * cell,
            flash.intensity,
        ));
    }
    // Молния: тёмная подложка (чтобы её было видно и на светлых полях), два слоя
    // свечения и яркая сердцевина.
    const SHADE: [f32; 3] = [0.10, 0.13, 0.0];
    for bolt in &frame.bolts {
        let points: Vec<Pos2> = bolt.points.iter().map(|&p| on_screen(p)).collect();
        let (width, level) = (bolt.width, bolt.intensity);
        for (thickness, color) in [
            (cell * 0.15 * width, blended(SHADE, 0.34 * level)),
            (cell * 0.26 * width, glow(fx::GLOW, 0.22 * level)),
            (cell * 0.11 * width, glow(fx::GLOW, 0.60 * level)),
            (
                (cell * 0.036 * width).max(1.2 * pixel),
                blended(fx::BOLT_CORE, level),
            ),
        ] {
            shapes.push(Shape::line(points.clone(), Stroke::new(thickness, color)));
        }
    }
    // Ореол: начинается под краем шашки и выступает из-под неё.
    if let Some(halo) = &frame.halo {
        let edge = PIECE_RADIUS * cell;
        shapes.push(glow_ring(
            on_screen(halo.center),
            edge * 0.8,
            edge,
            halo.radius * cell,
            halo.intensity,
        ));
    }
    painter.extend(shapes);
}

/// Верхний слой разряда: искры. Рисуются поверх всех шашек.
pub fn draw_sparks(painter: &Painter, layout: &BoardLayout, frame: &DischargeFrame, pixels_per_point: f32) {
    let cell = layout.cell;
    let pixel = 1.0 / pixels_per_point.max(0.1);
    let on_screen = |p: fx::Point| {
        let (x, y) = layout.cell_to_point(p);
        pos2(x, y)
    };
    let mut shapes: Vec<Shape> = Vec::new();
    // Искра — короткий светящийся штрих с яркой головкой.
    for spark in &frame.sparks {
        let (head, tail) = (on_screen(spark.head), on_screen(spark.tail));
        let color = fx::spark_color(spark.heat);
        let level = spark.intensity;
        shapes.push(Shape::line_segment(
            [tail, head],
            Stroke::new(cell * 0.085 * spark.size, glow(fx::GLOW, 0.30 * level)),
        ));
        let core = (cell * 0.032 * spark.size).max(1.2 * pixel);
        shapes.push(Shape::line_segment(
            [tail, head],
            Stroke::new(core, blended(color, 0.95 * level)),
        ));
        shapes.push(Shape::circle_filled(
            head,
            (core * 0.75).max(0.8 * pixel),
            blended(color, level),
        ));
    }
    painter.extend(shapes);
}

/// Кольцо вокруг шашки, обязанной бить.
pub fn draw_obligation_ring(painter: &Painter, layout: &BoardLayout, sq: Square) {
    let center = square_center(layout, sq);
    let width = (layout.cell * 0.045).max(1.5);
    painter.circle_stroke(
        center,
        layout.cell * (PIECE_RADIUS + 0.045),
        Stroke::new(width, theme::MUST_CAPTURE),
    );
}

/// Изображения шашек для текущего размера клетки.
///
/// Шашки рассчитываются попиксельно (`rucheckers::ui::sprite`) ровно под размер клетки
/// в физических пикселях и хранятся как текстуры: края и рельеф сглажены, а на месте
/// шашка выводится пиксель в пиксель, без растяжения.
pub struct PieceSprites {
    size_px: usize,
    pieces: [egui::TextureHandle; 4],
    lift_shadow: egui::TextureHandle,
}

impl PieceSprites {
    /// Рассчитывает изображения для клетки со стороной `size_px` физических пикселей.
    pub fn new(ctx: &egui::Context, size_px: usize) -> PieceSprites {
        let size_px = size_px.max(2);
        let upload = |name: &str, image: sprite::PieceImage| {
            let pixels = egui::ColorImage::from_rgba_premultiplied([image.size, image.size], &image.rgba);
            ctx.load_texture(name, pixels, egui::TextureOptions::LINEAR)
        };
        let piece = |color: Color, kind: Kind| {
            let name = format!("piece-{color:?}-{kind:?}");
            upload(
                &name,
                sprite::render_piece(Piece::new(color, kind), size_px, true),
            )
        };
        PieceSprites {
            size_px,
            pieces: [
                piece(Color::White, Kind::Man),
                piece(Color::White, Kind::King),
                piece(Color::Black, Kind::Man),
                piece(Color::Black, Kind::King),
            ],
            lift_shadow: upload("piece-lift-shadow", sprite::render_lift_shadow(size_px)),
        }
    }

    /// Сторона клетки в физических пикселях, под которую рассчитаны изображения.
    pub fn size_px(&self) -> usize {
        self.size_px
    }

    fn texture(&self, piece: Piece) -> &egui::TextureHandle {
        let index = match (piece.color, piece.kind) {
            (Color::White, Kind::Man) => 0,
            (Color::White, Kind::King) => 1,
            (Color::Black, Kind::Man) => 2,
            (Color::Black, Kind::King) => 3,
        };
        &self.pieces[index]
    }
}

/// Шашка с центром в точке `center` (центр изображения совпадает с центром шашки).
///
/// `alpha` — непрозрачность (побитые шашки бледнеют), `lifted` — шашку держат мышью:
/// она чуть крупнее и отбрасывает более длинную мягкую тень.
pub fn draw_piece(
    painter: &Painter,
    sprites: &PieceSprites,
    center: Pos2,
    cell: f32,
    piece: Piece,
    alpha: f32,
    lifted: bool,
) {
    let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    let tint = Color32::from_white_alpha((alpha.clamp(0.0, 1.0) * 255.0).round() as u8);
    let scale = if lifted { 1.06 } else { 1.0 };
    if lifted {
        let shadow = Rect::from_center_size(center + vec2(cell * 0.03, cell * 0.07), vec2(cell, cell) * 1.16);
        painter.image(sprites.lift_shadow.id(), shadow, whole, tint);
    }
    let rect = Rect::from_center_size(center, vec2(cell, cell) * scale);
    painter.image(sprites.texture(piece).id(), rect, whole, tint);
}

/// Отметка стороны, чья очередь хода, в углу рамки; пока компьютер думает,
/// вокруг неё вращается дуга.
pub fn draw_turn_marker(
    painter: &Painter,
    center: Pos2,
    size: f32,
    color: Color,
    thinking: bool,
    time: f64,
    pixels_per_point: f32,
) {
    let pixel = 1.0 / pixels_per_point.max(0.1);
    let radius = size * 0.26;
    let (fill, outline) = match color {
        Color::White => (theme::WHITE_FACE, theme::WHITE_OUTLINE),
        Color::Black => (theme::BLACK_FACE, theme::COORDINATES),
    };
    painter.circle_filled(center, radius, fill);
    painter.circle_stroke(center, radius, Stroke::new(pixel.max(1.0), outline));
    if thinking {
        let orbit = size * 0.40;
        let start = (time * 4.2) as f32;
        let points: Vec<Pos2> = (0..=18)
            .map(|i| {
                let angle = start + TAU * 0.62 * i as f32 / 18.0;
                center + vec2(angle.cos(), angle.sin()) * orbit
            })
            .collect();
        painter.add(Shape::line(
            points,
            Stroke::new((size * 0.09).max(1.5), theme::COORDINATES),
        ));
    }
}

/// Радиус, в пределах которого точка считается попавшей в шашку (для курсора).
pub fn piece_hit_radius(cell: f32) -> f32 {
    cell * (PIECE_RADIUS + 0.04)
}

/// Преобразование точки интерфейса в координаты для геометрии.
pub fn layout_for(rect: Rect, flipped: bool, pixels_per_point: f32) -> BoardLayout {
    BoardLayout::fit(
        rect.left(),
        rect.top(),
        rect.width(),
        rect.height(),
        flipped,
        pixels_per_point,
    )
}

/// Курсор «рука» над шашкой, которой можно ходить, и над доступным полем.
pub fn cursor_for(hover: bool, dragging: bool) -> egui::CursorIcon {
    if dragging {
        egui::CursorIcon::Grabbing
    } else if hover {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Default
    }
}
