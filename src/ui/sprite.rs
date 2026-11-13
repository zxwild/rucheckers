//! The picture of a piece: seen straight from above, computed pixel by pixel with anti-aliasing.
//!
//! A piece is a circle whose centre coincides with the centre of its cell. Its volume is
//! rendered by light and shade (a rounded edge, a rim, grooves), not by shifting parts,
//! so the silhouette is symmetric about both axes. The edge of the circle is smoothed by
//! the area of a pixel it covers; the relief and the crown, by averaging several
//! samples per pixel.
//!
//! The module only computes pixels; the graphical part makes textures from them.

use crate::board::{Color, Piece};
use std::f32::consts::{FRAC_PI_2, PI};

/// Radius of a piece as a fraction of the cell side.
pub const PIECE_RADIUS: f32 = 0.40;

/// A square image: RGBA with premultiplied alpha, row by row from the top.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PieceImage {
    /// Side of the image in pixels.
    pub size: usize,
    /// The pixels: four bytes each (R, G, B, A).
    pub rgba: Vec<u8>,
}

impl PieceImage {
    /// The pixel in the column `x` and the row `y`; transparent outside the image.
    #[must_use]
    pub fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        if x >= self.size || y >= self.size {
            return [0; 4];
        }
        let i = 4 * (y * self.size + x);
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }

    /// Opacity of a pixel (0…255).
    #[must_use]
    pub fn alpha(&self, x: usize, y: usize) -> u8 {
        self.pixel(x, y)[3]
    }
}

/// The colours of pieces of one colour.
struct Palette {
    /// Main tone of the top face.
    base: [f32; 3],
    /// Tone of the lit slopes.
    light: [f32; 3],
    /// Tone of the shaded slopes.
    dark: [f32; 3],
    /// A thin outline along the edge.
    outline: [f32; 3],
    /// The crown and the ring of a king.
    crown: [f32; 3],
    /// Highlight on the crown.
    crown_light: [f32; 3],
}

const WHITE: Palette = Palette {
    base: [0.955, 0.925, 0.850],
    light: [1.000, 0.990, 0.955],
    dark: [0.640, 0.575, 0.440],
    outline: [0.470, 0.405, 0.285],
    crown: [0.640, 0.455, 0.150],
    crown_light: [0.780, 0.600, 0.260],
};

const BLACK: Palette = Palette {
    base: [0.285, 0.215, 0.172],
    light: [0.520, 0.425, 0.350],
    dark: [0.085, 0.058, 0.042],
    outline: [0.045, 0.030, 0.022],
    crown: [0.870, 0.720, 0.380],
    crown_light: [0.960, 0.850, 0.560],
};

/// Direction to the light source: from the top left (the Y axis points down).
const LIGHT: (f32, f32) = (-0.555, -0.832);

/// Fractions of the radius that define the relief of the top face.
const EDGE_START: f32 = 0.875; // start of the rounded edge
const RIM_INNER: f32 = 0.780; // inner border of the flat rim
const STEP_WIDTH: f32 = 0.050; // width of the step from the rim down to the sunken middle
/// Grooves of a man: the radius of the middle and the half-width.
const GROOVES: [(f32, f32); 2] = [(0.560, 0.040), (0.330, 0.036)];
/// Ring of a king: the inner and outer radii.
const KING_RING: (f32, f32) = (0.655, 0.715);
/// Size of the crown as a fraction of the radius of the piece.
const CROWN_SCALE: f32 = 0.98;
/// The crown is not symmetric in height; the shift puts its middle at the centre of the piece.
const CROWN_SHIFT: f32 = 0.0475;

/// Shadow of a piece lying on the board: offset of the centre and width of the blur in
/// fractions of a cell, density.
const SHADOW_OFFSET: (f32, f32) = (0.008, 0.014);
const SHADOW_BLUR: f32 = 0.070;
const SHADOW_ALPHA: f32 = 0.42;

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Lighting of a point of the top face: from −1 (shade) to +1 (highlight).
///
/// `r` is the distance from the centre as a fraction of the radius; `toward_light` is the
/// cosine of the angle between the direction from the centre to the point and the direction
/// to the light.
fn relief(r: f32, toward_light: f32, king: bool) -> f32 {
    let r = r.clamp(0.0, 1.0);
    // A barely noticeable overall gradient: the side facing the light is slightly lighter.
    let mut shade = 0.10 * toward_light * r;
    if r > EDGE_START {
        // The rounded edge: a slope outwards; at the very edge the surface turns away from
        // the viewer.
        let e = (r - EDGE_START) / (1.0 - EDGE_START);
        shade += 0.80 * toward_light * (e * FRAC_PI_2).sin() - 0.20 * e * e;
    } else if r > RIM_INNER {
        // The flat rim.
        shade += 0.04;
    } else if r > RIM_INNER - STEP_WIDTH {
        // The step down to the middle: the slope faces the centre.
        let q = (r - (RIM_INNER - STEP_WIDTH)) / STEP_WIDTH;
        shade -= 0.55 * toward_light * (PI * q).sin();
    } else if !king {
        for (middle, half_width) in GROOVES {
            let q = (r - middle) / half_width;
            if q.abs() < 1.0 {
                // A groove: the inner slope faces outwards, the outer one faces the centre.
                let depth = (FRAC_PI_2 * q).cos();
                shade -= 0.50 * toward_light * (PI * q).sin() + 0.17 * depth * depth;
            }
        }
    }
    shade.clamp(-1.0, 1.0)
}

/// Whether a point belongs to the crown. Coordinates are fractions of the crown size,
/// centred at zero.
///
/// The crown is defined by its right half and mirrored about the vertical axis, so it
/// is symmetric exactly, not just to within rounding.
fn crown_contains(x: f32, y: f32) -> bool {
    fn in_triangle(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
        let side = |u: (f32, f32), v: (f32, f32)| (p.0 - v.0) * (u.1 - v.1) - (u.0 - v.0) * (p.1 - v.1);
        let (d1, d2, d3) = (side(a, b), side(b, c), side(c, a));
        let negative = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
        let positive = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
        !(negative && positive)
    }
    let x = x.abs();
    if x > 0.575 {
        return false;
    }
    // The band and the base.
    if x <= 0.50 && ((0.08..=0.26).contains(&y) || (0.33..=0.42).contains(&y)) {
        return true;
    }
    // Half of the middle tooth and a side tooth.
    let p = (x, y);
    if in_triangle(p, (0.00, 0.10), (0.00, -0.44), (0.28, 0.10))
        || in_triangle(p, (0.10, 0.10), (0.50, -0.34), (0.50, 0.10))
    {
        return true;
    }
    // The balls on the tips of the teeth.
    x.hypot(y + 0.44) <= 0.075 || (x - 0.50).hypot(y + 0.34) <= 0.075
}

/// Fraction of the pixel centred at `(px, py)` that the crown covers (16 samples).
fn crown_coverage(px: f32, py: f32, center: f32, size: f32) -> f32 {
    const N: usize = 4;
    let mut inside = 0;
    for sy in 0..N {
        for sx in 0..N {
            let x = px + (sx as f32 + 0.5) / N as f32 - 0.5;
            let y = py + (sy as f32 + 0.5) / N as f32 - 0.5;
            if crown_contains((x - center) / size, (y - center) / size - CROWN_SHIFT) {
                inside += 1;
            }
        }
    }
    inside as f32 / (N * N) as f32
}

/// Number of relief samples per pixel along each axis: small pieces are computed more
/// precisely; on large ones the details of the relief are wider than a pixel anyway.
const fn relief_samples(size: usize) -> usize {
    match size {
        0..=63 => 3,
        64..=159 => 2,
        _ => 1,
    }
}

/// Draws a piece in a square of `size × size` pixels (the side of a board cell).
///
/// The centre of the piece is at the centre of the image. `shadow` adds a soft shadow
/// under the piece; the piece itself does not change.
#[must_use]
pub fn render_piece(piece: Piece, size: usize, shadow: bool) -> PieceImage {
    let mut rgba = vec![0u8; size * size * 4];
    if size == 0 {
        return PieceImage { size, rgba };
    }
    let palette = match piece.color {
        Color::White => &WHITE,
        Color::Black => &BLACK,
    };
    let king = piece.is_king();
    let n = size as f32;
    let center = n / 2.0;
    let radius = PIECE_RADIUS * n;
    let outline_width = (0.035 * radius).max(1.0);
    let crown_size = CROWN_SCALE * radius;
    let samples = relief_samples(size);
    let shadow_center = (center + SHADOW_OFFSET.0 * n, center + SHADOW_OFFSET.1 * n);
    let shadow_blur = (SHADOW_BLUR * n).max(1.0);

    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let (dx, dy) = (px - center, py - center);
            let distance = dx.hypot(dy);
            // Anti-aliasing of the edge: the fraction of the pixel covered by the circle.
            let coverage = (radius - distance + 0.5).clamp(0.0, 1.0);

            let shadow_alpha = if shadow {
                let ds = (px - shadow_center.0).hypot(py - shadow_center.1);
                let fade = (ds - (radius - 0.3 * shadow_blur)) / (1.3 * shadow_blur);
                SHADOW_ALPHA * (1.0 - smoothstep(fade))
            } else {
                0.0
            };

            let mut color = [0.0f32; 3];
            if coverage > 0.0 {
                // The relief is averaged over several samples inside the pixel.
                let mut sum = [0.0f32; 3];
                for sy in 0..samples {
                    for sx in 0..samples {
                        let ox = dx + (sx as f32 + 0.5) / samples as f32 - 0.5;
                        let oy = dy + (sy as f32 + 0.5) / samples as f32 - 0.5;
                        let d = ox.hypot(oy);
                        let toward_light = if d > 1e-3 {
                            (ox * LIGHT.0 + oy * LIGHT.1) / d
                        } else {
                            0.0
                        };
                        let shade = relief(d / radius, toward_light, king);
                        let tone = if shade >= 0.0 {
                            mix(palette.base, palette.light, shade)
                        } else {
                            mix(palette.base, palette.dark, -shade)
                        };
                        sum[0] += tone[0];
                        sum[1] += tone[1];
                        sum[2] += tone[2];
                    }
                }
                let count = (samples * samples) as f32;
                color = [sum[0] / count, sum[1] / count, sum[2] / count];

                if king {
                    // The ring in the colour of the crown.
                    let ring = (distance - KING_RING.0 * radius + 0.5).clamp(0.0, 1.0)
                        * (KING_RING.1 * radius - distance + 0.5).clamp(0.0, 1.0);
                    color = mix(color, palette.crown, ring);
                    // The crown with a light shadow under it.
                    if distance < 0.64 * radius {
                        let lift = 0.035 * crown_size;
                        let under = crown_coverage(px - lift * 0.6, py - lift, center, crown_size);
                        let crown = crown_coverage(px, py, center, crown_size);
                        color = mix(color, palette.dark, 0.35 * under * (1.0 - crown));
                        let glow = 0.5 - 0.5 * (dx * LIGHT.0 + dy * LIGHT.1) / (0.6 * radius);
                        let crown_color = mix(palette.crown_light, palette.crown, glow);
                        color = mix(color, crown_color, crown);
                    }
                }

                // A thin dark outline separates the piece from the square.
                let edge = (distance - (radius - outline_width) + 0.5).clamp(0.0, 1.0);
                color = mix(color, palette.outline, 0.55 * edge);
            }

            let alpha = coverage + shadow_alpha * (1.0 - coverage);
            let i = 4 * (y * size + x);
            for channel in 0..3 {
                rgba[i + channel] = (color[channel].clamp(0.0, 1.0) * coverage * 255.0).round() as u8;
            }
            rgba[i + 3] = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    PieceImage { size, rgba }
}

/// The soft shadow under a piece held with the mouse: a blurred dark circle without the piece.
#[must_use]
pub fn render_lift_shadow(size: usize) -> PieceImage {
    let mut rgba = vec![0u8; size * size * 4];
    let n = size as f32;
    let center = n / 2.0;
    for y in 0..size {
        for x in 0..size {
            let distance = (x as f32 + 0.5 - center).hypot(y as f32 + 0.5 - center);
            let fade = (distance - 0.28 * n) / (0.21 * n);
            let alpha = 0.36 * (1.0 - smoothstep(fade));
            rgba[4 * (y * size + x) + 3] = (alpha * 255.0).round() as u8;
        }
    }
    PieceImage { size, rgba }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Kind;

    const SIZES: [usize; 8] = [24, 31, 48, 57, 64, 83, 100, 166];

    fn pieces() -> [Piece; 4] {
        [
            Piece::new(Color::White, Kind::Man),
            Piece::new(Color::White, Kind::King),
            Piece::new(Color::Black, Kind::Man),
            Piece::new(Color::Black, Kind::King),
        ]
    }

    #[test]
    fn silhouette_is_symmetric_about_both_axes() {
        // The piece is not "skewed": its outline is the same left and right, top and bottom.
        for piece in pieces() {
            for size in SIZES {
                let image = render_piece(piece, size, false);
                for y in 0..size {
                    for x in 0..size {
                        let a = image.alpha(x, y);
                        assert_eq!(a, image.alpha(size - 1 - x, y), "{piece:?} {size}: left-right");
                        assert_eq!(a, image.alpha(x, size - 1 - y), "{piece:?} {size}: top-bottom");
                        assert_eq!(a, image.alpha(y, x), "{piece:?} {size}: diagonal");
                    }
                }
            }
        }
    }

    #[test]
    fn piece_is_centered_in_its_cell() {
        for piece in pieces() {
            for size in SIZES {
                let image = render_piece(piece, size, false);
                let (mut sum, mut sum_x, mut sum_y) = (0.0f64, 0.0f64, 0.0f64);
                let (mut min_x, mut max_x, mut min_y, mut max_y) = (size, 0, size, 0);
                for y in 0..size {
                    for x in 0..size {
                        let a = f64::from(image.alpha(x, y));
                        sum += a;
                        sum_x += a * (x as f64 + 0.5);
                        sum_y += a * (y as f64 + 0.5);
                        if a > 0.0 {
                            min_x = min_x.min(x);
                            max_x = max_x.max(x);
                            min_y = min_y.min(y);
                            max_y = max_y.max(y);
                        }
                    }
                }
                let middle = size as f64 / 2.0;
                assert!(
                    (sum_x / sum - middle).abs() < 1e-6,
                    "{piece:?} {size}: centre along X"
                );
                assert!(
                    (sum_y / sum - middle).abs() < 1e-6,
                    "{piece:?} {size}: centre along Y"
                );
                // The margins to the cell borders are the same on all sides.
                assert_eq!(min_x, size - 1 - max_x, "{piece:?} {size}");
                assert_eq!(min_y, size - 1 - max_y, "{piece:?} {size}");
                assert_eq!(min_x, min_y, "{piece:?} {size}");
                // The diameter matches the given radius.
                let diameter = (max_x - min_x + 1) as f32;
                assert!(
                    (diameter - 2.0 * PIECE_RADIUS * size as f32).abs() <= 2.0,
                    "{piece:?} {size}: {diameter}"
                );
            }
        }
    }

    #[test]
    fn edge_is_antialiased() {
        for piece in pieces() {
            for size in SIZES {
                let image = render_piece(piece, size, false);
                let radius = PIECE_RADIUS * size as f32;
                let mut partial = 0;
                let mut hard_steps = 0;
                for y in 0..size {
                    for x in 0..size {
                        let a = image.alpha(x, y);
                        if a > 0 && a < 255 {
                            partial += 1;
                        }
                        if a == 255 {
                            let neighbours = [
                                (x.wrapping_sub(1), y),
                                (x + 1, y),
                                (x, y.wrapping_sub(1)),
                                (x, y + 1),
                            ];
                            hard_steps += neighbours
                                .iter()
                                .filter(|&&(nx, ny)| image.alpha(nx, ny) == 0)
                                .count();
                        }
                    }
                }
                // A band of semi-transparent pixels runs along the whole circle…
                assert!(
                    partial as f32 > 0.8 * 2.0 * PI * radius,
                    "{piece:?} {size}: {partial} semi-transparent"
                );
                // …and an opaque pixel is never next to an empty one (without anti-aliasing
                // there would be about eight radii of such transitions).
                assert!(hard_steps <= 8, "{piece:?} {size}: {hard_steps} hard transitions");
            }
        }
    }

    #[test]
    fn shading_has_no_hard_steps_inside_the_piece() {
        // Neighbouring pixels of the top face of a man are close in brightness: the
        // grooves and the rim are drawn smoothly, without "stair steps".
        for color in [Color::White, Color::Black] {
            for size in [48usize, 64, 100, 166] {
                let image = render_piece(Piece::new(color, Kind::Man), size, false);
                let radius = PIECE_RADIUS * size as f32;
                let middle = size as f32 / 2.0;
                let mut worst = 0i32;
                for y in 0..size {
                    for x in 0..size - 1 {
                        let d = (x as f32 + 1.0 - middle).hypot(y as f32 + 0.5 - middle);
                        if d > radius - 2.5 {
                            continue;
                        }
                        let (a, b) = (image.pixel(x, y), image.pixel(x + 1, y));
                        for channel in 0..3 {
                            worst = worst.max((i32::from(a[channel]) - i32::from(b[channel])).abs());
                        }
                    }
                }
                assert!(worst <= 70, "{color:?} {size}: difference {worst}");
            }
        }
    }

    #[test]
    fn shadow_lies_under_the_piece_and_does_not_change_it() {
        for piece in pieces() {
            let size = 96;
            let plain = render_piece(piece, size, false);
            let shaded = render_piece(piece, size, true);
            let mut shadow_pixels = 0;
            for y in 0..size {
                for x in 0..size {
                    let (p, s) = (plain.pixel(x, y), shaded.pixel(x, y));
                    assert_eq!(
                        &p[..3],
                        &s[..3],
                        "the shadow does not change the colour of the piece"
                    );
                    assert!(s[3] >= p[3]);
                    if p[3] == 255 {
                        assert_eq!(p, s);
                    }
                    if p[3] == 0 && s[3] > 0 {
                        shadow_pixels += 1;
                        assert!(s[3] < 128, "the shadow is semi-transparent");
                    }
                }
            }
            assert!(
                shadow_pixels > 100,
                "{piece:?}: the shadow is visible around the piece"
            );
            // The image is not cut off by the border of the cell.
            for i in 0..size {
                for (x, y) in [(i, 0), (i, size - 1), (0, i), (size - 1, i)] {
                    assert_eq!(shaded.alpha(x, y), 0, "{piece:?}: border ({x}, {y})");
                }
            }
        }
        let lift = render_lift_shadow(96);
        assert!(lift.alpha(48, 48) > 60 && lift.alpha(48, 48) < 128);
        assert_eq!(lift.alpha(0, 48), 0);
        assert!(lift.rgba.chunks(4).all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0));
    }

    /// Number of golden pixels (the colour of the crown and the ring of a king).
    fn golden_pixels(image: &PieceImage) -> usize {
        image
            .rgba
            .chunks(4)
            .filter(|p| p[3] == 255 && i32::from(p[0]) - i32::from(p[2]) > 95 && p[0] > 130)
            .count()
    }

    #[test]
    fn king_is_clearly_different_from_a_man() {
        for color in [Color::White, Color::Black] {
            for size in [32usize, 64, 100] {
                let man = render_piece(Piece::new(color, Kind::Man), size, false);
                let king = render_piece(Piece::new(color, Kind::King), size, false);
                // The outline is the same; the drawing differs.
                let alphas = |i: &PieceImage| i.rgba.chunks(4).map(|p| p[3]).collect::<Vec<u8>>();
                assert_eq!(alphas(&man), alphas(&king));
                let differing = man
                    .rgba
                    .chunks(4)
                    .zip(king.rgba.chunks(4))
                    .filter(|(a, b)| a != b)
                    .count();
                let area = PI * (PIECE_RADIUS * size as f32).powi(2);
                assert!(
                    differing as f32 > 0.25 * area,
                    "{color:?} {size}: {differing} pixels differ"
                );
                assert_eq!(golden_pixels(&man), 0, "{color:?} {size}");
                assert!(
                    golden_pixels(&king) as f32 > 0.08 * area,
                    "{color:?} {size}: the crown is noticeable"
                );
            }
        }
    }

    #[test]
    fn white_and_black_pieces_differ_in_brightness() {
        let brightness = |piece: Piece| {
            let image = render_piece(piece, 64, false);
            let p = image.pixel(32, 20);
            u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])
        };
        let white = brightness(Piece::new(Color::White, Kind::Man));
        let black = brightness(Piece::new(Color::Black, Kind::Man));
        assert!(white > 600 && black < 300, "{white} and {black}");
    }

    #[test]
    fn crown_is_centered_on_the_piece() {
        // The crown is symmetric left to right and sits in the middle of the piece in height.
        let size = 200;
        let half = CROWN_SCALE * PIECE_RADIUS * size as f32;
        let (mut top, mut bottom) = (f32::MAX, f32::MIN);
        for step in 0..2000 {
            let y = -1.0 + step as f32 / 1000.0;
            let row_has_crown = (0..400).any(|i| crown_contains(-1.0 + i as f32 / 200.0, y));
            if row_has_crown {
                top = top.min(y);
                bottom = bottom.max(y);
            }
            for i in 0..200 {
                let x = i as f32 / 200.0;
                assert_eq!(crown_contains(x, y), crown_contains(-x, y), "({x}, {y})");
            }
        }
        let middle = f32::midpoint(top, bottom) + CROWN_SHIFT;
        assert!(
            (middle * half).abs() < 0.5,
            "the middle of the crown is off by {} px",
            middle * half
        );
    }

    #[test]
    fn pixels_are_valid_premultiplied_colours() {
        for piece in pieces() {
            for shadow in [false, true] {
                let image = render_piece(piece, 57, shadow);
                assert_eq!(image.rgba.len(), 57 * 57 * 4);
                for p in image.rgba.chunks(4) {
                    assert!(p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3], "{p:?}");
                }
            }
        }
    }

    #[test]
    fn any_size_is_rendered_without_panicking() {
        for piece in pieces() {
            for size in [0usize, 1, 2, 3, 5, 8, 13, 400] {
                let image = render_piece(piece, size, true);
                assert_eq!(image.size, size);
                assert_eq!(image.rgba.len(), size * size * 4);
                assert_eq!(image.pixel(size, size), [0; 4]);
            }
            assert_eq!(
                render_piece(piece, 40, true),
                render_piece(piece, 40, true),
                "the computation is deterministic"
            );
        }
        assert_eq!(render_lift_shadow(0).rgba.len(), 0);
        assert_eq!(render_lift_shadow(1).rgba.len(), 4);
    }
}
