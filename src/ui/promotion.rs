//! A compass rose that lights up beneath a newly crowned king.
//!
//! Like the lightning, this effect computes coloured triangles on the CPU and leaves
//! drawing to the board's OpenGL renderer. There are no images, shaders or frame-rate
//! dependent updates. The geometry stays still: light travels outwards through it,
//! then the entire sign fades together. Coordinates are screen cells.

use super::fx::{ease, Point};
use super::geometry::screen_cell;
use crate::board::{Color, Square};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

mod letters;
use letters::Inlay;

/// Total lifetime, including the outward ignition and the fade, s.
pub const DURATION_SECONDS: f64 = 1.2;
/// Time for the light to reach the tips, s.
pub const REVEAL_SECONDS: f64 = 0.32;
/// A brief fully lit interval before the uniform fade, s.
pub const HOLD_SECONDS: f64 = 0.08;
/// Maximum extent, including the soft glow: half a cell into neighbouring cells.
pub const RADIUS: f32 = 1.0;
const TIP_RADIUS: f32 = RADIUS - 0.012;
const DIAGONAL_TIP_SCALE: f32 = 0.85;
const ROOT_RADIUS: f32 = 0.30;
const FRONT_SOFTNESS: f32 = 0.07;
const RAYS: u32 = 8;
const RADIAL_STEPS: u32 = 24;
const EDGE_STEPS: u32 = 4;

/// One vertex, with premultiplied RGBA. Emissive edges add a little more light than
/// their alpha removes from the background, exactly as the lightning's glow does.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Vertex {
    pub point: Point,
    pub rgba: [f32; 4],
}

/// A complete frame, drawn beneath every piece and clipped to the board's outer edge.
#[derive(Clone, PartialEq, Debug)]
pub struct PromotionFrame {
    pub center: Point,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

/// The promotion square, colour and absolute starting time. Kept separately from the
/// move animation so a king can continue a capture while the rose gently fades out.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Promotion {
    pub square: Square,
    pub color: Color,
    pub start: f64,
}

impl Promotion {
    #[must_use]
    pub const fn new(square: Square, color: Color, start: f64) -> Self {
        Self { square, color, start }
    }

    #[must_use]
    pub fn is_finished(self, now: f64) -> bool {
        now >= self.start + DURATION_SECONDS
    }

    /// The same time always gives the same mesh, regardless of previous frames.
    #[must_use]
    pub fn frame(self, now: f64, flipped: bool) -> Option<PromotionFrame> {
        self.frame_at_scale(now, flipped, 72.0)
    }

    /// A frame for the actual physical cell size. On small boards the inscriptions
    /// keep a six-pixel cap height while staying inside the original ray tips.
    #[must_use]
    pub fn frame_at_scale(self, now: f64, flipped: bool, cell_pixels: f32) -> Option<PromotionFrame> {
        let age = now - self.start;
        if !age.is_finite() || now < self.start || self.is_finished(now) {
            return None;
        }
        let center = screen_cell(
            f32::from(self.square.file()),
            f32::from(self.square.rank()),
            flipped,
        );
        let fade_start = REVEAL_SECONDS + HOLD_SECONDS;
        let intensity = (1.0 - ease((age - fade_start) / (DURATION_SECONDS - fade_start))) as f32;
        let front = -FRONT_SOFTNESS + (RADIUS + 2.0 * FRONT_SOFTNESS) * ease(age / REVEAL_SECONDS) as f32;
        let palette = Palette::for_color(self.color);
        let mut mesh = MeshBuilder {
            frame: PromotionFrame {
                center,
                vertices: Vec::with_capacity(18_000),
                indices: Vec::with_capacity(40_000),
            },
            intensity,
            front,
            inlays: Inlay::all(cell_pixels),
            engraving: false,
        };

        // An engraved circle and small azimuth marks make this a compass rose rather
        // than a plain star. They stay behind the faceted points.
        mesh.ring(0.63, 0.032, palette.light, 0.12, true);
        mesh.ring(0.63, 0.018, palette.light, 0.26, true);
        for ray in 0..RAYS {
            let angle = TAU * (ray as f32 + 0.5) / RAYS as f32 - FRAC_PI_2;
            mesh.stroke(
                polar(0.67, angle),
                polar(0.71, angle),
                0.007,
                palette.light,
                0.46,
                true,
            );
        }

        // Four long cardinal points alternate with slightly shorter diagonal points.
        // Splitting each point into two differently lit facets gives a fine ridge.
        for ray in 0..RAYS {
            let angle = TAU * ray as f32 / RAYS as f32 - FRAC_PI_2;
            let tip = polar(tip_radius(ray), angle);
            let left = polar(ROOT_RADIUS, angle - PI / RAYS as f32);
            let right = polar(ROOT_RADIUS, angle + PI / RAYS as f32);
            mesh.facet(left, tip, palette.face_light);
            mesh.facet(tip, right, palette.face_dark);
        }
        // Soft feathered ribbons, a contrasting hairline and a luminous inner edge.
        // Every stroke is subdivided so the reveal is circular even on a long edge.
        for ray in 0..RAYS {
            let angle = TAU * ray as f32 / RAYS as f32 - FRAC_PI_2;
            let tip = polar(tip_radius(ray), angle);
            for side in [-1.0, 1.0] {
                let root = polar(ROOT_RADIUS, angle + side * PI / RAYS as f32);
                for segment in 0..12 {
                    let a = mix(root, tip, segment as f32 / 12.0);
                    let b = mix(root, tip, (segment + 1) as f32 / 12.0);
                    mesh.stroke(a, b, 0.055, palette.light, 0.24, true);
                    mesh.stroke(a, b, 0.018, palette.ink, 0.16, false);
                    mesh.stroke(a, b, 0.014, palette.light, 0.34, true);
                }
            }
            // A narrow ridge between the two facets, mostly hidden by the piece.
            for segment in 0..12 {
                let a = mix((0.0, 0.0), tip, segment as f32 / 12.0);
                let b = mix((0.0, 0.0), tip, (segment + 1) as f32 / 12.0);
                mesh.stroke(a, b, 0.006, palette.light, 0.18, true);
            }
        }
        // Upright Roman capitals live inside the ends of the four cardinal rays.
        // Their outlines, light rims and glow are triangles in the same mesh, so
        // they ignite with the rays and share the sign's uniform fade.
        mesh.engraving = true;
        for inlay in mesh.inlays {
            mesh.letter(inlay, &palette);
        }
        Some(mesh.frame)
    }
}

struct Palette {
    face_light: [f32; 3],
    face_dark: [f32; 3],
    ink: [f32; 3],
    light: [f32; 3],
    letter_ink: [f32; 3],
    letter_edge: [f32; 3],
}

impl Palette {
    const fn for_color(color: Color) -> Self {
        match color {
            Color::White => Self {
                face_light: [0.98, 0.88, 0.65],
                face_dark: [0.69, 0.51, 0.27],
                ink: [0.36, 0.25, 0.12],
                light: [1.00, 0.91, 0.68],
                letter_ink: [0.58, 0.39, 0.145],
                letter_edge: [1.00, 0.925, 0.72],
            },
            Color::Black => Self {
                face_light: [0.51, 0.36, 0.25],
                face_dark: [0.19, 0.13, 0.10],
                ink: [0.10, 0.065, 0.045],
                light: [0.92, 0.72, 0.48],
                letter_ink: [0.32, 0.21, 0.115],
                letter_edge: [0.937, 0.765, 0.498],
            },
        }
    }
}

fn tip_radius(ray: u32) -> f32 {
    TIP_RADIUS * if ray % 2 == 0 { 1.0 } else { DIAGONAL_TIP_SCALE }
}

fn polar(radius: f32, angle: f32) -> Point {
    (radius * angle.cos(), radius * angle.sin())
}

fn mix(a: Point, b: Point, t: f32) -> Point {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// A small mesh builder; all points supplied to it are relative to the centre.
struct MeshBuilder {
    frame: PromotionFrame,
    intensity: f32,
    front: f32,
    inlays: [Inlay; 4],
    engraving: bool,
}

impl MeshBuilder {
    fn vertex(&mut self, point: Point, color: [f32; 3], opacity: f32, emissive: bool) -> u32 {
        let radius = point.0.hypot(point.1);
        let revealed = ease(f64::from(
            (self.front + FRONT_SOFTNESS - radius) / (2.0 * FRONT_SOFTNESS),
        )) as f32;
        let transmission = if self.engraving {
            1.0
        } else {
            self.inlays
                .iter()
                .fold(1.0f32, |light, inlay| light.min(inlay.ray_transmission(point)))
        };
        let alpha = opacity * transmission * revealed * self.intensity;
        let index = self.frame.vertices.len() as u32;
        self.frame.vertices.push(Vertex {
            point: (self.frame.center.0 + point.0, self.frame.center.1 + point.1),
            rgba: [
                color[0] * alpha,
                color[1] * alpha,
                color[2] * alpha,
                alpha * if emissive { 0.55 } else { 1.0 },
            ],
        });
        index
    }

    fn quad(&mut self, corners: [u32; 4]) {
        let [a, b, c, d] = corners;
        self.frame.indices.extend_from_slice(&[a, b, c, a, c, d]);
    }

    fn facet(&mut self, a: Point, b: Point, color: [f32; 3]) {
        for edge in 0..EDGE_STEPS {
            let from = mix(a, b, edge as f32 / EDGE_STEPS as f32);
            let to = mix(a, b, (edge + 1) as f32 / EDGE_STEPS as f32);
            for ring in 0..RADIAL_STEPS {
                let inner = ring as f32 / RADIAL_STEPS as f32;
                let outer = (ring + 1) as f32 / RADIAL_STEPS as f32;
                let corners = [
                    mix((0.0, 0.0), from, inner),
                    mix((0.0, 0.0), from, outer),
                    mix((0.0, 0.0), to, outer),
                    mix((0.0, 0.0), to, inner),
                ]
                .map(|p| self.vertex(p, color, 0.48, false));
                self.quad(corners);
            }
        }
    }

    fn stroke(&mut self, a: Point, b: Point, width: f32, color: [f32; 3], opacity: f32, emissive: bool) {
        let length = (b.0 - a.0).hypot(b.1 - a.1).max(1e-6);
        let normal = (-(b.1 - a.1) / length, (b.0 - a.0) / length);
        let mut previous: Option<[u32; 2]> = None;
        // Fine contours need a broad core to stay continuous at ordinary board sizes;
        // the wider glow ribbons keep a narrow core and a long, soft falloff.
        let core = if width < 0.02 { 0.65 } else { 0.22 };
        for (offset, strength) in [(-1.0, 0.0), (-core, 1.0), (core, 1.0), (1.0, 0.0)] {
            let edge = [a, b].map(|p| {
                self.vertex(
                    (p.0 + normal.0 * width * offset, p.1 + normal.1 * width * offset),
                    color,
                    opacity * strength,
                    emissive,
                )
            });
            if let Some([left, right]) = previous {
                self.quad([left, right, edge[1], edge[0]]);
            }
            previous = Some(edge);
        }
    }

    fn ring(&mut self, radius: f32, width: f32, color: [f32; 3], opacity: f32, emissive: bool) {
        const SEGMENTS: u32 = 128;
        for segment in 0..SEGMENTS {
            self.stroke(
                polar(radius, TAU * segment as f32 / SEGMENTS as f32),
                polar(radius, TAU * (segment + 1) as f32 / SEGMENTS as f32),
                width,
                color,
                opacity,
                emissive,
            );
        }
    }

    fn letter(&mut self, inlay: Inlay, palette: &Palette) {
        let points: Vec<Point> = inlay
            .glyph
            .outline
            .iter()
            .map(|p| {
                (
                    p.0 * inlay.scale + inlay.center.0,
                    p.1 * inlay.scale + inlay.center.1,
                )
            })
            .collect();
        self.letter_band(
            &points,
            &inlay.glyph.edge_offsets,
            0.009,
            palette.letter_edge,
            0.22,
            true,
        );
        self.letter_band(
            &points,
            &inlay.glyph.edge_offsets,
            0.004,
            palette.letter_edge,
            0.74,
            false,
        );
        self.letter_band(
            &points,
            &inlay.glyph.edge_offsets,
            0.0015,
            palette.letter_ink,
            0.92,
            false,
        );
        let first = self.frame.vertices.len() as u32;
        for point in points {
            self.vertex(point, palette.letter_ink, 0.92, false);
        }
        self.frame
            .indices
            .extend(inlay.glyph.indices.iter().map(|&i| first + i));
    }

    fn letter_band(
        &mut self,
        points: &[Point],
        offsets: &[Point],
        width: f32,
        color: [f32; 3],
        opacity: f32,
        emissive: bool,
    ) {
        let first = self.frame.vertices.len() as u32;
        for (&p, &offset) in points.iter().zip(offsets) {
            self.vertex(p, color, opacity, emissive);
            self.vertex(
                (p.0 + offset.0 * width, p.1 + offset.1 * width),
                color,
                0.0,
                emissive,
            );
        }
        for i in 0..points.len() as u32 {
            let a = first + i * 2;
            let b = first + ((i + 1) % points.len() as u32) * 2;
            self.quad([a, a + 1, b + 1, b]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effect(color: Color) -> Promotion {
        Promotion::new(Square::parse("d8").unwrap(), color, 10.0)
    }

    #[test]
    fn lifetime_has_no_frames_before_or_after_the_effect() {
        let rose = effect(Color::White);
        assert!(rose.frame(9.99, false).is_none());
        assert!(rose
            .frame(10.0, false)
            .unwrap()
            .vertices
            .iter()
            .all(|v| v.rgba[3] == 0.0));
        assert!(rose.frame(10.0 + DURATION_SECONDS, false).is_none());
        assert!(rose.is_finished(10.0 + DURATION_SECONDS));
        assert!(rose.frame(f64::NAN, false).is_none());
    }

    #[test]
    fn the_light_reaches_the_middle_before_the_tips() {
        let rose = effect(Color::White);
        let early = rose.frame(10.0 + REVEAL_SECONDS * 0.5, false).unwrap();
        let full = rose.frame(10.0 + REVEAL_SECONDS, false).unwrap();
        let mut lit_middle = false;
        let mut lit_tip = false;
        for (a, b) in early.vertices.iter().zip(&full.vertices) {
            let radius = (a.point.0 - early.center.0).hypot(a.point.1 - early.center.1);
            if (0.40..0.45).contains(&radius) && a.rgba[3] > 0.01 {
                lit_middle = true;
            }
            if radius > 0.85 {
                assert_eq!(a.rgba[3], 0.0);
                lit_tip |= b.rgba[3] > 0.01;
            }
        }
        assert!(lit_middle && lit_tip);
    }

    #[test]
    fn after_reveal_every_vertex_fades_by_the_same_factor() {
        for color in [Color::White, Color::Black] {
            let rose = effect(color);
            let full = rose.frame(10.0 + REVEAL_SECONDS, false).unwrap();
            let fade_start = REVEAL_SECONDS + HOLD_SECONDS;
            let half = rose
                .frame(10.0 + f64::midpoint(fade_start, DURATION_SECONDS), false)
                .unwrap();
            assert_eq!(full.indices, half.indices);
            for (a, b) in full.vertices.iter().zip(&half.vertices) {
                assert_eq!(a.point, b.point, "the sign does not shrink or drift");
                for channel in 0..4 {
                    assert!((a.rgba[channel] * 0.5 - b.rgba[channel]).abs() < 1e-5);
                }
            }
        }
    }

    #[test]
    fn engraved_letters_ignite_with_the_outer_part_of_the_rays() {
        let rose = effect(Color::White);
        let early = rose.frame(10.0 + REVEAL_SECONDS * 0.5, false).unwrap();
        let full = rose.frame(10.0 + REVEAL_SECONDS, false).unwrap();
        let mut ink_vertices = 0;
        for (early, lit) in early.vertices.iter().zip(&full.vertices) {
            if lit.rgba[3] > 0.90 {
                assert_eq!(early.rgba[3], 0.0, "letters do not precede the outward ignition");
                ink_vertices += 1;
            }
        }
        assert!(ink_vertices >= 100);
    }

    #[test]
    fn mesh_is_finite_bounded_and_reproducible_in_both_orientations() {
        for color in [Color::White, Color::Black] {
            let rose = effect(color);
            for flipped in [false, true] {
                let frame = rose.frame(10.25, flipped).unwrap();
                assert_eq!(frame, rose.frame(10.25, flipped).unwrap());
                assert!(frame.vertices.len() < 18_000);
                assert_eq!(frame.indices.len() % 3, 0);
                assert!(frame.indices.iter().all(|&i| (i as usize) < frame.vertices.len()));
                for vertex in &frame.vertices {
                    let radius = (vertex.point.0 - frame.center.0).hypot(vertex.point.1 - frame.center.1);
                    assert!(radius <= RADIUS + 1e-5);
                    assert!(vertex
                        .rgba
                        .iter()
                        .all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
                }
                let extent = |angle: f32| {
                    frame.vertices.iter().fold(0.0f32, |furthest, vertex| {
                        let x = vertex.point.0 - frame.center.0;
                        let y = vertex.point.1 - frame.center.1;
                        furthest.max(x * angle.cos() + y * angle.sin())
                    })
                };
                let cardinal = extent(0.0);
                let diagonal = extent(PI / 4.0);
                assert!((0.80..0.90).contains(&(diagonal / cardinal)));
                for ray in 0..RAYS {
                    let expected = if ray % 2 == 0 { cardinal } else { diagonal };
                    assert!((extent(TAU * ray as f32 / RAYS as f32) - expected).abs() < 1e-5);
                }
            }
        }
    }
}
