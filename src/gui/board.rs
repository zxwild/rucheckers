//! Drawing of the board and pieces. The board is drawn with vector shapes and meshes,
//! the pieces with images computed for the exact cell size, so the picture stays sharp
//! at any window size and screen density.

use super::theme;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke,
    StrokeKind,
};
use rucheckers::games::{Color, Kind, Piece, Square};
use rucheckers::ui::fx::{self, DischargeFrame};
use rucheckers::ui::geometry::BoardLayout;
use rucheckers::ui::promotion::PromotionFrame;
use rucheckers::ui::sprite::{self, PIECE_RADIUS};
use rucheckers::ui::wood::{self, BoardStyle, ColorMesh, Look};
use std::f32::consts::{FRAC_1_SQRT_2, TAU};
use std::sync::Arc;

/// A colour with a changed opacity (0…1).
fn faded(color: Color32, alpha: f32) -> Color32 {
    if alpha >= 0.999 {
        color
    } else {
        color.gamma_multiply(alpha.clamp(0.0, 1.0))
    }
}

/// A deterministic pseudo-random number 0…1 for an integer key (for the wood pattern).
fn noise(key: u32) -> f32 {
    let mut x = key.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFFFF) as f32 / 65_535.0
}

/// Rectangle of a cell.
pub fn cell_rect(layout: &BoardLayout, file: u8, rank: u8) -> Rect {
    let (x, y) = layout.cell_origin(file, rank);
    Rect::from_min_size(pos2(x, y), vec2(layout.cell, layout.cell))
}

/// Rectangle of the whole board with its frame.
pub fn outer_rect(layout: &BoardLayout) -> Rect {
    Rect::from_min_size(
        pos2(layout.outer_x, layout.outer_y),
        vec2(layout.outer_size, layout.outer_size),
    )
}

/// Centre of a square on the screen.
pub fn square_center(layout: &BoardLayout, sq: Square) -> Pos2 {
    let (x, y) = layout.square_center(sq);
    pos2(x, y)
}

/// Thin veins of wood inside a rectangle.
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

/// The board: the frame with coordinates and the 64 cells, in the chosen look.
///
/// `art` keeps the meshes of a wooden board between frames; they are computed again only
/// when the look, the placement or the screen density changes.
pub fn draw_board(
    painter: &Painter,
    layout: &BoardLayout,
    pixels_per_point: f32,
    style: BoardStyle,
    art: &mut BoardArt,
) {
    match style.look() {
        None => draw_standard_board(painter, layout, pixels_per_point),
        Some(look) => draw_wooden_board(
            painter,
            layout,
            pixels_per_point,
            look,
            art.meshes(style, look, layout, pixels_per_point),
        ),
    }
}

/// The meshes of a wooden board for one look and one placement. Computing them takes
/// milliseconds; drawing them again takes no computation at all.
#[derive(Default)]
pub struct BoardArt {
    key: Option<(BoardStyle, BoardLayout, u32)>,
    frame: Arc<Mesh>,
    cells: Arc<Mesh>,
}

impl BoardArt {
    /// The frame and cell meshes for `style` at `layout`.
    fn meshes(
        &mut self,
        style: BoardStyle,
        look: &Look,
        layout: &BoardLayout,
        pixels_per_point: f32,
    ) -> WoodMeshes {
        let key = (style, *layout, pixels_per_point.to_bits());
        if self.key != Some(key) {
            self.frame = Arc::new(egui_mesh(look.frame_mesh(layout, pixels_per_point)));
            self.cells = Arc::new(egui_mesh(look.cells_mesh(layout, pixels_per_point)));
            self.key = Some(key);
        }
        WoodMeshes {
            frame: Arc::clone(&self.frame),
            cells: Arc::clone(&self.cells),
        }
    }
}

/// The two meshes of a wooden board, shared with the cache.
struct WoodMeshes {
    frame: Arc<Mesh>,
    cells: Arc<Mesh>,
}

/// A mesh of the window from the mesh computed for the board.
fn egui_mesh(mesh: ColorMesh) -> Mesh {
    let vertices = mesh
        .positions
        .iter()
        .zip(&mesh.colors)
        .map(|(&[x, y], &[red, green, blue])| egui::epaint::Vertex {
            pos: pos2(x, y),
            uv: egui::epaint::WHITE_UV,
            color: Color32::from_rgb(red, green, blue),
        })
        .collect();
    Mesh {
        indices: mesh.indices,
        vertices,
        texture_id: egui::TextureId::default(),
    }
}

const fn solid(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

/// A band of `width` around `inner`, made of four rectangles. Meshes are not smoothed,
/// so the band stays crisp: its edges lie on pixel borders.
fn band_around(mesh: &mut Mesh, inner: Rect, width: f32, color: Color32) {
    let outer = inner.expand(width);
    mesh.add_colored_rect(
        Rect::from_min_max(outer.min, pos2(outer.right(), inner.top())),
        color,
    );
    mesh.add_colored_rect(
        Rect::from_min_max(pos2(outer.left(), inner.bottom()), outer.max),
        color,
    );
    mesh.add_colored_rect(
        Rect::from_min_max(
            pos2(outer.left(), inner.top()),
            pos2(inner.left(), inner.bottom()),
        ),
        color,
    );
    mesh.add_colored_rect(
        Rect::from_min_max(
            pos2(inner.right(), inner.top()),
            pos2(outer.right(), inner.bottom()),
        ),
        color,
    );
}

/// A wooden board: the mitred frame, the cells, the edging of the field and the coordinates.
fn draw_wooden_board(
    painter: &Painter,
    layout: &BoardLayout,
    pixels_per_point: f32,
    look: &Look,
    meshes: WoodMeshes,
) {
    let pixel = 1.0 / pixels_per_point.max(0.1);
    let outer = outer_rect(layout);
    let frame = layout.frame();
    let field = Rect::from_min_size(
        pos2(layout.field_x, layout.field_y),
        vec2(layout.field_size(), layout.field_size()),
    );
    let radius = wood::corner_radius(frame);
    let corner = CornerRadius::same(radius as u8);

    // The shadow under the board, as under the standard one.
    painter.rect_filled(
        outer.translate(vec2(0.0, frame * 0.10)).expand(frame * 0.03),
        corner,
        Color32::from_black_alpha(46),
    );
    // A solid base: where the strips of the frame meet, it shows instead of the window,
    // and its rounded edge is smooth.
    painter.rect_filled(outer, corner, solid(look.frame_base()));
    painter.add(Shape::Mesh(meshes.frame));
    // The joints of the mitred corners.
    let joint = Stroke::new(pixel, Color32::from_black_alpha(40));
    let cut = radius * (1.0 - FRAC_1_SQRT_2);
    for (inner, outer_corner) in [
        (field.left_top(), outer.left_top() + vec2(cut, cut)),
        (field.right_top(), outer.right_top() + vec2(-cut, cut)),
        (field.left_bottom(), outer.left_bottom() + vec2(cut, -cut)),
        (field.right_bottom(), outer.right_bottom() + vec2(-cut, -cut)),
    ] {
        painter.line_segment([inner, outer_corner], joint);
    }
    // A smooth outline over the steps of the mesh along the rounded corners.
    painter.rect_stroke(
        outer,
        corner,
        Stroke::new(pixel.max(1.0), solid(look.edge)),
        StrokeKind::Inside,
    );
    painter.add(Shape::Mesh(meshes.cells));

    // The edging of the field: a dark line, or a light inlay between two dark lines.
    let mut lines = Mesh::default();
    let edge = (frame * pixels_per_point * 0.045).round().max(1.0) * pixel;
    band_around(&mut lines, field, edge, solid(look.edge));
    if let Some(inlay) = look.inlay {
        let strip = (frame * pixels_per_point * 0.05).round().max(1.0) * pixel;
        band_around(&mut lines, field.expand(edge), strip, solid(inlay));
        band_around(&mut lines, field.expand(edge + strip), edge, solid(look.edge));
    }
    painter.add(Shape::mesh(lines));
    draw_coordinates(painter, layout, solid(look.coordinates));
}

/// The standard board: plain light and dark cells with a few fine veins.
fn draw_standard_board(painter: &Painter, layout: &BoardLayout, pixels_per_point: f32) {
    let pixel = 1.0 / pixels_per_point.max(0.1);
    let outer = outer_rect(layout);
    let frame = layout.frame();
    let field = Rect::from_min_size(
        pos2(layout.field_x, layout.field_y),
        vec2(layout.field_size(), layout.field_size()),
    );

    // The shadow under the board and the frame.
    let radius = CornerRadius::same(wood::corner_radius(frame) as u8);
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

    // The cells.
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
    // The edging around the playing field.
    painter.rect_stroke(
        field,
        0.0,
        Stroke::new((frame * 0.05).max(pixel), theme::FRAME_DARK),
        StrokeKind::Outside,
    );
    draw_coordinates(painter, layout, theme::COORDINATES);
}

/// Coordinates on the frame: letters above and below, digits on the left and right.
fn draw_coordinates(painter: &Painter, layout: &BoardLayout, color: Color32) {
    let outer = outer_rect(layout);
    let frame = layout.frame();
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
                painter.text(pos2(x, cy), Align2::CENTER_CENTER, &letter, font.clone(), color);
            }
            for cx in [outer.left() + frame / 2.0, outer.right() - frame / 2.0] {
                painter.text(pos2(cx, y), Align2::CENTER_CENTER, &digit, font.clone(), color);
            }
        }
    }
}

/// Fills a square with a semi-transparent colour (the last move, the selected piece).
pub fn tint_square(painter: &Painter, layout: &BoardLayout, sq: Square, color: Color32) {
    painter.rect_filled(cell_rect(layout, sq.file(), sq.rank()), 0.0, color);
}

/// Mark of a square a piece can go to.
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

/// Mark of the final square of a multi-capture: a click on it makes all the remaining
/// jumps. It differs from an ordinary reachable square by a ring.
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

/// A glow: the colour is added to the background and dims it slightly at the same time.
/// Pure addition gives an almost white colour on light warm squares and loses the tint.
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

/// A colour with the given opacity that covers the background.
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

/// A round glow: bright in the centre and fading to nothing at the edge.
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

/// A glow in a ring: brightest at the radius `peak`, fading to nothing at `inner` and `outer`.
fn glow_ring(center: Pos2, inner: f32, peak: f32, outer: f32, intensity: f32) -> Shape {
    const SEGMENTS: u32 = 40;
    let mut mesh = egui::Mesh::default();
    for (radius, color) in [
        (inner, Color32::TRANSPARENT),
        (peak, glow(fx::GLOW, 0.80 * intensity)),
        (f32::midpoint(peak, outer), glow(fx::GLOW, 0.26 * intensity)),
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

/// The lower layer of the discharge: flashes on the captured pieces, the lightning
/// behind the flying piece and the halo around it. It is drawn over the standing pieces
/// but under the flying one.
///
/// Everything is drawn with plain triangles through the same hardware-accelerated
/// rendering (OpenGL) as the rest of the interface, so the effect works the same on
/// macOS and Linux and needs no shaders of its own.
pub fn draw_lightning(
    painter: &Painter,
    layout: &BoardLayout,
    frame: &DischargeFrame,
    pixels_per_point: f32,
) {
    // A dark underlay keeps the bolt visible on light squares as well.
    const SHADE: [f32; 3] = [0.10, 0.13, 0.0];
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
    // The bolt: the dark underlay, two layers of glow and a bright core.
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
    // The halo: it starts under the edge of the piece and shows from under it.
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

/// The promotion sign goes under every piece, so the new king naturally covers its
/// centre. Rays may cover the coordinate frame, stopping at the board's outer edge.
pub fn draw_promotion(painter: &Painter, layout: &BoardLayout, frame: &PromotionFrame) {
    let mut mesh = egui::Mesh::default();
    mesh.indices.clone_from(&frame.indices);
    mesh.vertices.reserve(frame.vertices.len());
    for vertex in &frame.vertices {
        let (x, y) = layout.cell_to_point(vertex.point);
        let [red, green, blue, alpha] = vertex.rgba.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
        mesh.colored_vertex(
            pos2(x, y),
            Color32::from_rgba_premultiplied(red, green, blue, alpha),
        );
    }
    painter.with_clip_rect(outer_rect(layout)).add(Shape::mesh(mesh));
}

/// The upper layer of the discharge: the sparks. They are drawn over all pieces.
pub fn draw_sparks(painter: &Painter, layout: &BoardLayout, frame: &DischargeFrame, pixels_per_point: f32) {
    let cell = layout.cell;
    let pixel = 1.0 / pixels_per_point.max(0.1);
    let on_screen = |p: fx::Point| {
        let (x, y) = layout.cell_to_point(p);
        pos2(x, y)
    };
    let mut shapes: Vec<Shape> = Vec::new();
    // A spark is a short glowing stroke with a bright head.
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

/// The ring around a piece that must capture.
pub fn draw_obligation_ring(painter: &Painter, layout: &BoardLayout, sq: Square) {
    let center = square_center(layout, sq);
    let width = (layout.cell * 0.045).max(1.5);
    painter.circle_stroke(
        center,
        layout.cell * (PIECE_RADIUS + 0.045),
        Stroke::new(width, theme::MUST_CAPTURE),
    );
}

/// Images of the pieces for the current cell size.
///
/// The pieces are computed pixel by pixel (`rucheckers::ui::sprite`) for exactly the
/// cell size in physical pixels and kept as textures: the edges and the relief are
/// smooth, and a resting piece is drawn pixel for pixel, without stretching.
pub struct PieceSprites {
    size_px: usize,
    pieces: [egui::TextureHandle; 4],
    lift_shadow: egui::TextureHandle,
}

impl PieceSprites {
    /// Computes the images for a cell with the side of `size_px` physical pixels.
    pub fn new(ctx: &egui::Context, size_px: usize) -> Self {
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
        Self {
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

    /// The side of the cell in physical pixels the images were computed for.
    pub const fn size_px(&self) -> usize {
        self.size_px
    }

    const fn texture(&self, piece: Piece) -> &egui::TextureHandle {
        let index = match (piece.color, piece.kind) {
            (Color::White, Kind::Man) => 0,
            (Color::White, Kind::King) => 1,
            (Color::Black, Kind::Man) => 2,
            (Color::Black, Kind::King) => 3,
        };
        &self.pieces[index]
    }
}

/// A piece centred at the point `center` (the centre of the image is the centre of the piece).
///
/// `alpha` is the opacity (captured pieces dim); `lifted` means the piece is held with
/// the mouse: it is slightly larger and casts a longer soft shadow.
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

/// The mark of the side to move, in a corner of the frame; while the computer is
/// thinking, an arc rotates around it.
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

/// The radius within which a point counts as being on a piece (for the cursor).
pub fn piece_hit_radius(cell: f32) -> f32 {
    cell * (PIECE_RADIUS + 0.04)
}

/// Converts an interface point into coordinates for the geometry.
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

/// The "hand" cursor over a piece that can move and over a reachable square.
pub const fn cursor_for(hover: bool, dragging: bool) -> egui::CursorIcon {
    if dragging {
        egui::CursorIcon::Grabbing
    } else if hover {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rucheckers::ui::promotion::Promotion;

    #[test]
    fn wooden_meshes_are_kept_until_the_look_or_the_placement_changes() {
        let layout = BoardLayout::fit(0.0, 0.0, 640.0, 640.0, false, 1.0);
        let oak = BoardStyle::Oak;
        let mut art = BoardArt::default();
        let first = art.meshes(oak, oak.look().unwrap(), &layout, 1.0);
        assert!(first.frame.is_valid() && first.cells.is_valid());
        assert!(!first.frame.vertices.is_empty() && !first.cells.vertices.is_empty());
        let again = art.meshes(oak, oak.look().unwrap(), &layout, 1.0);
        assert!(Arc::ptr_eq(&first.frame, &again.frame) && Arc::ptr_eq(&first.cells, &again.cells));

        let maple = BoardStyle::MapleWalnut;
        let moved = BoardLayout::fit(10.0, 0.0, 640.0, 640.0, false, 1.0);
        let larger = BoardLayout::fit(0.0, 0.0, 700.0, 700.0, false, 1.0);
        for (style, layout, ppp) in [
            (maple, layout, 1.0),
            (oak, moved, 1.0),
            (oak, larger, 1.0),
            (oak, layout, 1.5),
        ] {
            let other = art.meshes(style, style.look().unwrap(), &layout, ppp);
            assert!(
                !Arc::ptr_eq(&first.cells, &other.cells),
                "{style:?} {layout:?} {ppp}"
            );
            assert!(
                !Arc::ptr_eq(&first.frame, &other.frame),
                "{style:?} {layout:?} {ppp}"
            );
        }
    }

    #[test]
    fn the_mesh_for_the_window_keeps_every_vertex_and_colour() {
        let look = BoardStyle::AmberBirch.look().unwrap();
        let layout = BoardLayout::fit(0.0, 0.0, 300.0, 300.0, false, 1.0);
        let computed = look.cells_mesh(&layout, 1.0);
        let mesh = egui_mesh(computed.clone());
        assert!(mesh.is_valid());
        assert_eq!(mesh.indices, computed.indices);
        assert_eq!(mesh.vertices.len(), computed.positions.len());
        for (vertex, (position, rgb)) in mesh
            .vertices
            .iter()
            .zip(computed.positions.iter().zip(&computed.colors))
        {
            assert_eq!([vertex.pos.x, vertex.pos.y], *position);
            assert_eq!(vertex.color, solid(*rgb));
            assert_eq!(vertex.uv, egui::epaint::WHITE_UV);
        }
    }

    #[test]
    fn only_wooden_boards_compute_meshes_and_each_draws_its_coordinates() {
        let ctx = egui::Context::default();
        let layout = BoardLayout::fit(0.0, 0.0, 640.0, 640.0, false, 1.0);
        let mut art = BoardArt::default();
        let draw = |style: BoardStyle, art: &mut BoardArt| {
            let output = ctx.run(egui::RawInput::default(), |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::background());
                draw_board(&painter, &layout, 1.0, style, art);
            });
            let meshes = output
                .shapes
                .iter()
                .filter(|clipped| matches!(&clipped.shape, Shape::Mesh(mesh) if mesh.vertices.len() > 1000))
                .count();
            let letters: Vec<Color32> = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    Shape::Text(text) if text.galley.text() == "a" => Some(text.fallback_color),
                    _ => None,
                })
                .collect();
            (meshes, letters)
        };
        let (meshes, letters) = draw(BoardStyle::Standard, &mut art);
        assert_eq!(meshes, 0);
        assert!(art.key.is_none(), "the standard board keeps nothing");
        assert_eq!(letters, [theme::COORDINATES; 2]);
        for style in BoardStyle::ALL.into_iter().skip(1) {
            let (meshes, letters) = draw(style, &mut art);
            assert_eq!(meshes, 2, "{style:?}: the frame and the cells");
            assert_eq!(art.key, Some((style, layout, 1.0_f32.to_bits())));
            assert_eq!(
                letters,
                [solid(style.look().unwrap().coordinates); 2],
                "{style:?}"
            );
        }
    }

    #[test]
    fn promotion_covers_the_coordinate_frame_but_stops_at_the_outer_board_edge() {
        for name in ["a1", "h8", "d8", "e1"] {
            for flipped in [false, true] {
                let layout = BoardLayout::fit(0.0, 0.0, 640.0, 640.0, flipped, 1.0);
                let field = Rect::from_min_size(
                    pos2(layout.field_x, layout.field_y),
                    vec2(layout.field_size(), layout.field_size()),
                );
                let rose = Promotion::new(
                    Square::parse(name).unwrap().draughts().unwrap(),
                    Color::White,
                    0.0,
                );
                let frame = rose.frame(0.25, flipped).unwrap();
                let ctx = egui::Context::default();
                let output = ctx.run(egui::RawInput::default(), |ctx| {
                    let painter = ctx.layer_painter(egui::LayerId::background());
                    draw_promotion(&painter, &layout, &frame);
                });
                let clipped = output
                    .shapes
                    .iter()
                    .find(|shape| matches!(&shape.shape, Shape::Mesh(mesh) if mesh.is_valid()))
                    .expect("the rose is a valid OpenGL triangle mesh");
                assert_eq!(clipped.clip_rect, outer_rect(&layout));
                let Shape::Mesh(mesh) = &clipped.shape else {
                    unreachable!()
                };
                assert!(
                    mesh.vertices.iter().any(|vertex| {
                        !field.contains(vertex.pos)
                            && clipped.clip_rect.contains(vertex.pos)
                            && vertex.color.a() > 0
                    }),
                    "the rose remains visible over the coordinate frame"
                );
            }
        }
    }

    #[test]
    fn all_four_letters_fit_at_promotion_corners_on_either_board_orientation() {
        for (name, color) in [("h8", Color::White), ("a1", Color::Black)] {
            for flipped in [false, true] {
                for size in [320.0, 440.0, 640.0] {
                    for ppp in [1.0, 1.25, 2.0] {
                        let layout = BoardLayout::fit(17.0, 23.0, size, size, flipped, ppp);
                        let frame =
                            Promotion::new(Square::parse(name).unwrap().draughts().unwrap(), color, 0.0)
                                .frame_at_scale(0.36, flipped, layout.cell * ppp)
                                .unwrap();
                        let mut letters = [0; 4];
                        let mut bounds = [Rect::NOTHING; 4];
                        // Only the engraved ink reaches this opacity; rays and light
                        // rims remain translucent. Check the actual rendered vertices.
                        for vertex in frame.vertices.iter().filter(|v| v.rgba[3] > 0.90) {
                            let (x, y) = layout.cell_to_point(vertex.point);
                            assert!(
                                outer_rect(&layout).contains(pos2(x, y)),
                                "letter clipped at {name}, flipped={flipped}, size={size}, ppp={ppp}"
                            );
                            let dx = vertex.point.0 - frame.center.0;
                            let dy = vertex.point.1 - frame.center.1;
                            let direction = if dy.abs() > dx.abs() {
                                if dy < 0.0 {
                                    0
                                } else {
                                    2
                                }
                            } else if dx > 0.0 {
                                1
                            } else {
                                3
                            };
                            letters[direction] += 1;
                            bounds[direction].extend_with(pos2(x, y));
                        }
                        assert!(
                            letters.iter().all(|&count| count >= 20),
                            "N, E, S and W are all present"
                        );
                        assert!(
                            bounds.iter().all(|letter| letter.height() * ppp >= 5.999),
                            "the letter faces stay at least six physical pixels tall"
                        );
                    }
                }
            }
        }
    }
}
