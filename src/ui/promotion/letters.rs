//! Four small Roman capitals drawn as vector outlines, in screen-cell coordinates.
//! The outlines are built and triangulated once; no font or texture is needed.

use super::Point;
use std::sync::OnceLock;

const HEIGHT: f32 = 0.09;
/// Outermost ink, leaving room for the light rim inside the original ray's tip.
const OUTER_RADIUS: f32 = 0.915;

pub(super) struct Glyph {
    pub outline: Vec<Point>,
    pub edge_offsets: Vec<Point>,
    pub indices: Vec<u32>,
    pub half_size: Point,
}

#[derive(Clone, Copy)]
pub(super) struct Inlay {
    pub glyph: &'static Glyph,
    pub center: Point,
    pub scale: f32,
}

impl Inlay {
    pub fn all(cell_pixels: f32) -> [Self; 4] {
        static GLYPHS: OnceLock<[Glyph; 4]> = OnceLock::new();
        let glyphs = GLYPHS.get_or_init(|| [north(), east(), south(), west()]);
        // Keep the capitals readable on small boards, growing towards the centre
        // rather than moving their outer edges closer to the board's border.
        let scale = (6.0 / cell_pixels.max(1.0) / HEIGHT).clamp(1.0, 2.0);
        let directions = [(0.0, -1.0), (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0)];
        std::array::from_fn(|index| {
            let glyph = &glyphs[index];
            let (dx, dy) = directions[index];
            let half_length = if dx == 0.0 {
                glyph.half_size.1
            } else {
                glyph.half_size.0
            };
            let radius = OUTER_RADIUS - half_length * scale;
            Self {
                glyph,
                center: (dx * radius, dy * radius),
                scale,
            }
        })
    }

    /// A soft tapered slot interrupts the ray's ridge behind the letter. This only
    /// lowers the existing light's opacity, letting the actual board show through.
    pub fn ray_transmission(self, point: Point) -> f32 {
        let x = (point.0 - self.center.0).abs();
        let y = (point.1 - self.center.1).abs();
        let half = (
            self.glyph.half_size.0 * self.scale,
            self.glyph.half_size.1 * self.scale,
        );
        let horizontal = (x - half.0 - 0.012) / 0.012;
        let vertical = (y - half.1 - 0.010) / 0.012;
        let taper = (x / (half.0 + 0.024) + y / (half.1 + 0.030) - 1.65) / 0.35;
        let outside = horizontal.max(vertical).max(taper);
        0.31 + 0.69 * super::ease(f64::from(outside)) as f32
    }
}

fn north() -> Glyph {
    Glyph::new(vec![
        (0.00, 0.00),
        (0.30, 0.00),
        (0.86, 0.78),
        (0.86, 0.11),
        (0.83, 0.045),
        (0.72, 0.035),
        (0.72, 0.00),
        (1.04, 0.00),
        (1.04, 0.035),
        (0.93, 0.045),
        (0.91, 0.11),
        (0.91, 1.00),
        (0.86, 1.00),
        (0.18, 0.14),
        (0.18, 0.89),
        (0.20, 0.955),
        (0.31, 0.965),
        (0.31, 1.00),
        (0.00, 1.00),
        (0.00, 0.965),
        (0.11, 0.955),
        (0.13, 0.89),
        (0.13, 0.11),
        (0.10, 0.045),
        (0.00, 0.035),
    ])
}

fn east() -> Glyph {
    Glyph::new(vec![
        (0.00, 0.00),
        (0.77, 0.00),
        (0.79, 0.24),
        (0.75, 0.24),
        (0.70, 0.07),
        (0.62, 0.045),
        (0.22, 0.045),
        (0.22, 0.46),
        (0.51, 0.46),
        (0.55, 0.43),
        (0.57, 0.32),
        (0.60, 0.32),
        (0.60, 0.66),
        (0.57, 0.66),
        (0.55, 0.55),
        (0.51, 0.51),
        (0.22, 0.51),
        (0.22, 0.955),
        (0.62, 0.955),
        (0.70, 0.92),
        (0.77, 0.73),
        (0.81, 0.73),
        (0.78, 1.00),
        (0.00, 1.00),
        (0.00, 0.965),
        (0.11, 0.95),
        (0.12, 0.90),
        (0.12, 0.10),
        (0.11, 0.05),
        (0.00, 0.035),
    ])
}

fn south() -> Glyph {
    let mut outline = vec![(0.64, 0.035), (0.64, 0.265), (0.60, 0.265)];
    curve(&mut outline, [(0.57, 0.12), (0.49, 0.06), (0.34, 0.06)]);
    curve(&mut outline, [(0.20, 0.06), (0.12, 0.13), (0.12, 0.23)]);
    curve(&mut outline, [(0.12, 0.34), (0.20, 0.39), (0.39, 0.46)]);
    curve(&mut outline, [(0.62, 0.55), (0.70, 0.63), (0.70, 0.79)]);
    curve(&mut outline, [(0.70, 0.94), (0.57, 1.02), (0.35, 1.02)]);
    curve(&mut outline, [(0.20, 1.02), (0.11, 0.98), (0.07, 0.95)]);
    outline.extend([(0.04, 1.01), (0.01, 1.01), (0.01, 0.72), (0.05, 0.72)]);
    curve(&mut outline, [(0.10, 0.90), (0.20, 0.97), (0.36, 0.97)]);
    curve(&mut outline, [(0.51, 0.97), (0.59, 0.89), (0.59, 0.79)]);
    curve(&mut outline, [(0.59, 0.68), (0.50, 0.62), (0.32, 0.55)]);
    curve(&mut outline, [(0.10, 0.46), (0.02, 0.38), (0.02, 0.22)]);
    curve(&mut outline, [(0.02, 0.08), (0.15, 0.00), (0.35, 0.00)]);
    curve(&mut outline, [(0.48, 0.00), (0.55, 0.04), (0.60, 0.07)]);
    outline.push((0.61, 0.035));
    Glyph::new(outline)
}

fn west() -> Glyph {
    let mut outline = vec![
        (0.00, 0.00),
        (0.40, 0.00),
        (0.40, 0.035),
        (0.29, 0.055),
        (0.28, 0.105),
        (0.51, 0.83),
        (0.71, 0.23),
        (0.67, 0.105),
        (0.63, 0.055),
        (0.55, 0.035),
        (0.55, 0.00),
        (0.98, 0.00),
        (0.98, 0.035),
        (0.86, 0.055),
        (0.85, 0.105),
        (1.09, 0.83),
        (1.32, 0.12),
        (1.31, 0.06),
        (1.20, 0.035),
        (1.20, 0.00),
        (1.53, 0.00),
        (1.53, 0.035),
        (1.43, 0.06),
        (1.39, 0.14),
        (1.10, 1.00),
        (1.04, 1.00),
        (0.75, 0.35),
        (0.53, 1.00),
        (0.47, 1.00),
        (0.16, 0.15),
        (0.12, 0.065),
        (0.00, 0.035),
    ];
    for point in &mut outline {
        point.0 *= 1.36 / 1.53;
    }
    Glyph::new(outline)
}

fn curve(outline: &mut Vec<Point>, controls: [Point; 3]) {
    let start = *outline.last().expect("a curve follows a point");
    let [a, b, end] = controls;
    for step in 1..=8 {
        let t = step as f32 / 8.0;
        let u = 1.0 - t;
        outline.push((
            u.powi(3) * start.0 + 3.0 * u * u * t * a.0 + 3.0 * u * t * t * b.0 + t.powi(3) * end.0,
            u.powi(3) * start.1 + 3.0 * u * u * t * a.1 + 3.0 * u * t * t * b.1 + t.powi(3) * end.1,
        ));
    }
}

impl Glyph {
    fn new(mut outline: Vec<Point>) -> Self {
        let min_x = outline.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let min_y = outline.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        let max_x = outline.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
        let max_y = outline.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
        let indices = triangulate(&outline);
        let scale = HEIGHT / (max_y - min_y);
        let center = (f32::midpoint(min_x, max_x), f32::midpoint(min_y, max_y));
        for p in &mut outline {
            *p = ((p.0 - center.0) * scale, (p.1 - center.1) * scale);
        }
        let edge_offsets = offsets(&outline);
        Self {
            outline,
            edge_offsets,
            indices,
            half_size: ((max_x - min_x) * scale / 2.0, HEIGHT / 2.0),
        }
    }
}

fn cross(a: Point, b: Point, c: Point) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Shared edge joins keep the thin rim continuous without layering separate strokes
/// over each other at the serifs. Limit the miter at their sharp outer corners.
fn offsets(points: &[Point]) -> Vec<Point> {
    let area: f32 = points
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let b = points[(i + 1) % points.len()];
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    let normal = |a: Point, b: Point| {
        let length = (b.0 - a.0).hypot(b.1 - a.1);
        let sign = area.signum();
        ((b.1 - a.1) / length * sign, -(b.0 - a.0) / length * sign)
    };
    points
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let previous = normal(points[(i + points.len() - 1) % points.len()], p);
            let next = normal(p, points[(i + 1) % points.len()]);
            let sum = (previous.0 + next.0, previous.1 + next.1);
            let denominator = sum.0 * next.0 + sum.1 * next.1;
            let offset = if denominator.abs() < 1e-6 {
                next
            } else {
                (sum.0 / denominator, sum.1 / denominator)
            };
            let length = offset.0.hypot(offset.1);
            let limit = 2.0 / length.max(2.0);
            (offset.0 * limit, offset.1 * limit)
        })
        .collect()
}

/// Ear clipping for the four known simple outlines, including the concave serifs.
fn triangulate(points: &[Point]) -> Vec<u32> {
    let mut remaining: Vec<usize> = (0..points.len()).collect();
    let area: f32 = remaining
        .iter()
        .enumerate()
        .map(|(i, &a)| {
            let b = remaining[(i + 1) % remaining.len()];
            points[a].0 * points[b].1 - points[b].0 * points[a].1
        })
        .sum();
    if area < 0.0 {
        remaining.reverse();
    }
    let mut indices = Vec::with_capacity((points.len() - 2) * 3);
    while remaining.len() > 2 {
        let count = remaining.len();
        let ear = (0..count)
            .find(|&i| {
                let [a, b, c] = [
                    remaining[(i + count - 1) % count],
                    remaining[i],
                    remaining[(i + 1) % count],
                ];
                if cross(points[a], points[b], points[c]) <= 1e-8 {
                    return false;
                }
                !remaining.iter().any(|&p| {
                    p != a
                        && p != b
                        && p != c
                        && cross(points[a], points[b], points[p]) >= -1e-8
                        && cross(points[b], points[c], points[p]) >= -1e-8
                        && cross(points[c], points[a], points[p]) >= -1e-8
                })
            })
            .expect("the four letter outlines are simple polygons");
        indices.extend([
            remaining[(ear + count - 1) % count] as u32,
            remaining[ear] as u32,
            remaining[(ear + 1) % count] as u32,
        ]);
        remaining.remove(ear);
    }
    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangulated_letters_cover_their_outlines_without_overlapping() {
        for inlay in Inlay::all(72.0) {
            let glyph = inlay.glyph;
            assert_eq!(glyph.indices.len(), (glyph.outline.len() - 2) * 3);
            let polygon_area = glyph
                .outline
                .iter()
                .enumerate()
                .map(|(i, &a)| {
                    let b = glyph.outline[(i + 1) % glyph.outline.len()];
                    a.0 * b.1 - b.0 * a.1
                })
                .sum::<f32>()
                .abs();
            let triangle_area = glyph
                .indices
                .chunks_exact(3)
                .map(|triangle| {
                    let [a, b, c] =
                        [triangle[0], triangle[1], triangle[2]].map(|i| glyph.outline[i as usize]);
                    let area = cross(a, b, c);
                    assert!(area > 0.0);
                    area
                })
                .sum::<f32>();
            assert!((polygon_area - triangle_area).abs() < 1e-7);
        }
    }
}
