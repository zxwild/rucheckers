//! The looks of the board: the standard one and five lacquered woods.
//!
//! A wooden board is drawn without pictures: its cells and the four strips of its frame
//! are meshes with a colour at every vertex, and the graphics card blends the colours
//! between the vertices. This module computes those meshes; the window only puts them
//! on the screen.
//!
//! The colour of a vertex comes from a model of a piece of wood cut from a log: growth
//! rings around the pith, each darkening towards its end (the late wood) and of uneven
//! width, fine fibres along the grain, broader lighter and darker zones, and for some
//! woods ribbon stripes. Every cell is cut from its own place in the log and differs a
//! little in tone. Over all of it lies the lacquer: a soft reflection from the top left
//! and a slight fall of the light towards the bottom right.
//!
//! Coordinates are those of the interface, in points, as in [`BoardLayout`].

use crate::i18n::Text;
use crate::ui::geometry::BoardLayout;
use std::f32::consts::{FRAC_PI_2, TAU};

/// Direction to the light: from the top left (the Y axis points down), as for the pieces.
const LIGHT: (f32, f32) = (-0.555, -0.832);

/// The look of the board, chosen in the settings.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum BoardStyle {
    /// Plain light and dark cells with a few fine veins: the board of the earlier versions.
    #[default]
    Standard,
    /// Light maple and walnut, a satin lacquer.
    MapleWalnut,
    /// Beech and mahogany with ribbon stripes, a high gloss.
    BeechMahogany,
    /// Oak and smoked oak with a pronounced grain, nearly matt.
    Oak,
    /// Ash and cherry laid like parquet, with a light inlay around the field.
    AshCherry,
    /// Birch under an amber lacquer, as on old folding boards.
    AmberBirch,
}

impl BoardStyle {
    /// All looks in the order of the list in the settings.
    pub const ALL: [Self; 6] = [
        Self::Standard,
        Self::MapleWalnut,
        Self::BeechMahogany,
        Self::Oak,
        Self::AshCherry,
        Self::AmberBirch,
    ];

    /// Name in the settings file.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::MapleWalnut => "maple_walnut",
            Self::BeechMahogany => "beech_mahogany",
            Self::Oak => "oak",
            Self::AshCherry => "ash_cherry",
            Self::AmberBirch => "amber_birch",
        }
    }

    /// The look with the given name; an unknown name gives `None`.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        let code = code.trim();
        Self::ALL
            .into_iter()
            .find(|style| style.code().eq_ignore_ascii_case(code))
    }

    /// The name shown in the settings.
    #[must_use]
    pub const fn text_key(self) -> Text {
        match self {
            Self::Standard => Text::BoardStandard,
            Self::MapleWalnut => Text::BoardMapleWalnut,
            Self::BeechMahogany => Text::BoardBeechMahogany,
            Self::Oak => Text::BoardOak,
            Self::AshCherry => Text::BoardAshCherry,
            Self::AmberBirch => Text::BoardAmberBirch,
        }
    }

    /// The woods of a wooden board; `None` for the standard board, which is drawn with
    /// plain colours.
    #[must_use]
    pub const fn look(self) -> Option<&'static Look> {
        match self {
            Self::Standard => None,
            Self::MapleWalnut => Some(&MAPLE_WALNUT),
            Self::BeechMahogany => Some(&BEECH_MAHOGANY),
            Self::Oak => Some(&OAK),
            Self::AshCherry => Some(&ASH_CHERRY),
            Self::AmberBirch => Some(&AMBER_BIRCH),
        }
    }
}

/// One kind of wood.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Wood {
    /// Colour of the early (light) wood of a growth ring.
    early: [u8; 3],
    /// Colour of the late (dark) wood at the end of a ring.
    late: [u8; 3],
    /// Growth rings per cell width.
    rings: f32,
    /// How far the late wood goes from `early` towards `late` (0…1).
    contrast: f32,
    /// Narrowness of the late wood: the power of the ring profile.
    sharp: f32,
    /// Fine fibres along the grain: relative brightness amplitude.
    fibre: f32,
    /// 0: nearly straight lines (quarter sawn), 1: arches (flat sawn).
    arch: f32,
    /// Waviness of the grain, in cell widths.
    meander: f32,
    /// Broad light and dark stripes along the grain, as in mahogany.
    ribbon: f32,
    /// Random brightness difference between pieces of the wood.
    tone: f32,
    /// Broad lighter and darker zones that follow the rings: relative amplitude.
    figure: f32,
}

/// A wood of the given colours with average properties; the looks below change some.
const fn wood(early: [u8; 3], late: [u8; 3]) -> Wood {
    Wood {
        early,
        late,
        rings: 10.0,
        contrast: 0.4,
        sharp: 3.0,
        fibre: 0.03,
        arch: 0.3,
        meander: 0.03,
        ribbon: 0.0,
        tone: 0.03,
        figure: 0.04,
    }
}

/// A wooden board: the woods of the cells and of the frame, the lacquer and the lines.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Look {
    light: Wood,
    dark: Wood,
    frame: Wood,
    /// Light cells have the grain across the files, dark ones along them.
    parquet: bool,
    /// Strength of the reflection of the lacquer.
    gloss: f32,
    /// Seed of the patterns: each look has its own pieces of wood.
    seed: u32,
    /// The thin dark line around the playing field and around the board.
    pub edge: [u8; 3],
    /// A light inlay strip around the field, between two dark lines.
    pub inlay: Option<[u8; 3]>,
    /// The colour of the coordinates on the frame.
    pub coordinates: [u8; 3],
}

const MAPLE_WALNUT: Look = Look {
    light: Wood {
        rings: 12.0,
        contrast: 0.30,
        sharp: 5.0,
        fibre: 0.022,
        arch: 0.3,
        figure: 0.035,
        tone: 0.022,
        ..wood([0xEC, 0xD6, 0xAA], [0xD0, 0xAB, 0x76])
    },
    dark: Wood {
        rings: 9.0,
        contrast: 0.45,
        sharp: 3.0,
        fibre: 0.035,
        arch: 0.4,
        meander: 0.04,
        figure: 0.07,
        tone: 0.035,
        ..wood([0x93, 0x62, 0x3D], [0x5C, 0x38, 0x1F])
    },
    frame: Wood {
        rings: 14.0,
        contrast: 0.45,
        sharp: 3.0,
        fibre: 0.04,
        arch: 0.4,
        meander: 0.05,
        figure: 0.08,
        ..wood([0x60, 0x3D, 0x25], [0x38, 0x22, 0x12])
    },
    parquet: false,
    gloss: 0.045,
    seed: 11,
    edge: [0x2E, 0x1B, 0x0E],
    inlay: None,
    coordinates: [0xE6, 0xD2, 0xA8],
};

const BEECH_MAHOGANY: Look = Look {
    light: Wood {
        rings: 12.0,
        contrast: 0.25,
        sharp: 4.0,
        fibre: 0.025,
        arch: 0.2,
        figure: 0.03,
        tone: 0.022,
        ..wood([0xEA, 0xCB, 0xA2], [0xD3, 0xA6, 0x76])
    },
    dark: Wood {
        rings: 11.0,
        contrast: 0.25,
        sharp: 2.0,
        fibre: 0.03,
        arch: 0.0,
        meander: 0.02,
        ribbon: 0.065,
        figure: 0.04,
        tone: 0.03,
        ..wood([0x9E, 0x50, 0x30], [0x6C, 0x2E, 0x17])
    },
    frame: Wood {
        rings: 14.0,
        contrast: 0.30,
        sharp: 2.0,
        fibre: 0.035,
        arch: 0.05,
        meander: 0.03,
        ribbon: 0.06,
        figure: 0.05,
        ..wood([0x6E, 0x32, 0x20], [0x48, 0x1D, 0x10])
    },
    parquet: false,
    gloss: 0.07,
    seed: 23,
    edge: [0x30, 0x12, 0x08],
    inlay: None,
    coordinates: [0xEC, 0xD0, 0xA6],
};

const OAK: Look = Look {
    light: Wood {
        rings: 10.0,
        contrast: 0.50,
        sharp: 3.0,
        fibre: 0.05,
        arch: 0.7,
        meander: 0.04,
        figure: 0.05,
        ..wood([0xDF, 0xC6, 0x9C], [0xB3, 0x92, 0x62])
    },
    dark: Wood {
        rings: 10.0,
        contrast: 0.50,
        sharp: 3.0,
        fibre: 0.05,
        arch: 0.7,
        meander: 0.04,
        figure: 0.06,
        tone: 0.035,
        ..wood([0x8C, 0x6E, 0x50], [0x58, 0x41, 0x2D])
    },
    frame: Wood {
        rings: 14.0,
        contrast: 0.50,
        sharp: 3.0,
        fibre: 0.05,
        arch: 0.5,
        meander: 0.05,
        figure: 0.07,
        ..wood([0x5E, 0x47, 0x31], [0x38, 0x28, 0x1A])
    },
    parquet: false,
    gloss: 0.02,
    seed: 37,
    edge: [0x2A, 0x1E, 0x13],
    inlay: None,
    coordinates: [0xE4, 0xD4, 0xB4],
};

const ASH_CHERRY: Look = Look {
    light: Wood {
        rings: 9.0,
        contrast: 0.45,
        sharp: 3.5,
        fibre: 0.04,
        arch: 0.6,
        meander: 0.04,
        figure: 0.04,
        ..wood([0xE9, 0xD6, 0xB2], [0xC4, 0xA5, 0x78])
    },
    dark: Wood {
        rings: 11.0,
        contrast: 0.30,
        sharp: 3.0,
        fibre: 0.03,
        arch: 0.3,
        figure: 0.06,
        tone: 0.035,
        ..wood([0xB0, 0x71, 0x4A], [0x84, 0x48, 0x2B])
    },
    frame: Wood {
        rings: 14.0,
        contrast: 0.35,
        sharp: 3.0,
        fibre: 0.035,
        arch: 0.3,
        meander: 0.05,
        figure: 0.06,
        ..wood([0x7A, 0x44, 0x29], [0x55, 0x2F, 0x1B])
    },
    parquet: true,
    gloss: 0.045,
    seed: 41,
    edge: [0x3A, 0x1F, 0x10],
    inlay: Some([0xE2, 0xCC, 0xA2]),
    coordinates: [0xF0, 0xDC, 0xB8],
};

const AMBER_BIRCH: Look = Look {
    light: Wood {
        rings: 12.0,
        contrast: 0.22,
        sharp: 2.0,
        fibre: 0.025,
        arch: 0.15,
        meander: 0.06,
        figure: 0.06,
        tone: 0.035,
        ..wood([0xEE, 0xC8, 0x86], [0xDA, 0xAB, 0x66])
    },
    dark: Wood {
        rings: 12.0,
        contrast: 0.22,
        sharp: 2.0,
        fibre: 0.025,
        arch: 0.15,
        meander: 0.06,
        figure: 0.06,
        tone: 0.045,
        ..wood([0xAA, 0x60, 0x2E], [0x84, 0x45, 0x20])
    },
    frame: Wood {
        rings: 14.0,
        contrast: 0.30,
        sharp: 2.0,
        fibre: 0.035,
        arch: 0.2,
        meander: 0.07,
        figure: 0.07,
        ..wood([0x70, 0x3C, 0x1D], [0x4C, 0x28, 0x12])
    },
    parquet: false,
    gloss: 0.075,
    seed: 53,
    edge: [0x36, 0x1C, 0x0B],
    inlay: None,
    coordinates: [0xF2, 0xD6, 0xA0],
};

/// Corner radius of the board for a frame of the given width (points). Whole points, as
/// the rounded rectangles of the window take them.
#[must_use]
pub fn corner_radius(frame: f32) -> f32 {
    (frame * 0.28).clamp(2.0, 14.0).floor()
}

/// Triangles with a colour at every vertex, in the coordinates of the interface.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ColorMesh {
    pub positions: Vec<[f32; 2]>,
    pub colors: Vec<[u8; 3]>,
    /// Three vertex indices per triangle.
    pub indices: Vec<u32>,
}

impl ColorMesh {
    /// A grid of `columns × rows` quads whose vertex `(i, j)` is made by `vertex`.
    fn add_grid(&mut self, columns: u32, rows: u32, mut vertex: impl FnMut(u32, u32) -> ([f32; 2], [u8; 3])) {
        let base = self.positions.len() as u32;
        for j in 0..=rows {
            for i in 0..=columns {
                let (position, color) = vertex(i, j);
                self.positions.push(position);
                self.colors.push(color);
            }
        }
        let row = columns + 1;
        for j in 0..rows {
            for i in 0..columns {
                let v = base + j * row + i;
                self.indices
                    .extend_from_slice(&[v, v + 1, v + row + 1, v, v + row + 1, v + row]);
            }
        }
    }
}

/// Smallest distance between neighbouring vertices across the grain, physical pixels:
/// the late wood of a ring is a line about one such step wide.
const ACROSS_SPACING_PX: f32 = 1.25;
/// Smallest distance between vertices along the grain, physical pixels.
const ALONG_SPACING_PX: f32 = 2.0;
/// Most vertices per cell width across the grain. On large boards the steps grow, and
/// the averaging over a step keeps the thin lines smooth.
const ACROSS_PER_CELL: f32 = 64.0;
/// Vertices per cell length along straight grain; arches need more.
const ALONG_PER_CELL: f32 = 14.0;
const ALONG_PER_CELL_ARCHED: f32 = 20.0;

/// The number of steps on a stretch of `length` points.
fn steps(length: f32, pixels_per_point: f32, spacing_px: f32, per_cell: f32, cell: f32) -> u32 {
    let by_pixels = (length * pixels_per_point / spacing_px).ceil();
    let by_pattern = (length / cell * per_cell).ceil();
    by_pixels.min(by_pattern).max(2.0) as u32
}

impl Wood {
    fn along_per_cell(&self) -> f32 {
        ALONG_PER_CELL + ALONG_PER_CELL_ARCHED * self.arch
    }
}

impl Look {
    /// The four mitred strips of the frame, the grain along each side, rounded at the
    /// outer corners. The rounded outer edge is lighter on the lit sides.
    #[must_use]
    pub fn frame_mesh(&self, layout: &BoardLayout, pixels_per_point: f32) -> ColorMesh {
        let board = Board::new(layout, pixels_per_point, self.gloss);
        let mut mesh = ColorMesh::default();
        for side in 0..4u8 {
            let pattern = Pattern::new(
                &self.frame,
                hash(self.seed * 100 + u32::from(side)),
                layout.outer_size / layout.cell,
            );
            board.frame_side(&mut mesh, side, &pattern);
        }
        mesh
    }

    /// The 64 cells, each from its own piece of wood.
    #[must_use]
    pub fn cells_mesh(&self, layout: &BoardLayout, pixels_per_point: f32) -> ColorMesh {
        let board = Board::new(layout, pixels_per_point, self.gloss);
        let mut mesh = ColorMesh::default();
        for row in 0..8u8 {
            for column in 0..8u8 {
                let file = layout.file_of_column(column);
                let rank = layout.rank_of_row(row);
                let dark = (file + rank) % 2 == 0;
                let wood = if dark { &self.dark } else { &self.light };
                // The pieces belong to the places on the screen: the board looks the
                // same whichever side is at the bottom.
                let seed = hash(self.seed * 1000 + u32::from(row) * 8 + u32::from(column) + 7);
                let pattern = Pattern::new(wood, seed, 1.0);
                let (x, y) = layout.cell_origin(file, rank);
                board.cell(&mut mesh, (x, y), &pattern, dark || !self.parquet);
            }
        }
        mesh
    }

    /// The solid colour under the frame strips: where the strips meet, it shows instead
    /// of the window.
    #[must_use]
    pub fn frame_base(&self) -> [u8; 3] {
        mix(rgb(self.frame.early), rgb(self.frame.late), 0.3)
            .map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8)
    }
}

/// The placement of the board and the lacquer over it.
struct Board {
    outer: (f32, f32, f32),
    frame: f32,
    cell: f32,
    pixels_per_point: f32,
    gloss: f32,
}

impl Board {
    fn new(layout: &BoardLayout, pixels_per_point: f32, gloss: f32) -> Self {
        Self {
            outer: (layout.outer_x, layout.outer_y, layout.outer_size.max(1e-3)),
            frame: layout.frame(),
            cell: layout.cell.max(1e-3),
            pixels_per_point: if pixels_per_point.is_finite() && pixels_per_point > 0.01 {
                pixels_per_point
            } else {
                1.0
            },
            gloss,
        }
    }

    /// The colour of wood at a point after the lacquer and an extra `shade`.
    fn lacquered(&self, x: f32, y: f32, color: [f32; 3], shade: f32) -> [u8; 3] {
        let (left, top, size) = self.outer;
        let u = (x - left) / size;
        let v = (y - top) / size;
        let falloff = 1.0 - 0.035 * (u + v - 1.0);
        // The reflection adds the same light to every colour, as a clear coat does.
        let sheen = self.gloss * (-((u - 0.30).powi(2) + 1.3 * (v - 0.22).powi(2)) / 0.14).exp();
        color.map(|value| ((value * shade * falloff + sheen).clamp(0.0, 1.0) * 255.0).round() as u8)
    }

    /// A cell with the top left corner at `origin`; the grain runs along Y (`vertical`)
    /// or along X.
    fn cell(&self, mesh: &mut ColorMesh, origin: (f32, f32), pattern: &Pattern, vertical: bool) {
        let size = self.cell;
        let ppp = self.pixels_per_point;
        let columns = steps(size, ppp, ACROSS_SPACING_PX, ACROSS_PER_CELL, size);
        let rows = steps(size, ppp, ALONG_SPACING_PX, pattern.wood.along_per_cell(), size);
        let step = (1.0 / columns as f32, 1.0 / rows as f32);
        mesh.add_grid(columns, rows, |i, j| {
            let across = i as f32 * step.0;
            let along = j as f32 * step.1;
            let (x, y) = if vertical {
                (origin.0 + across * size, origin.1 + along * size)
            } else {
                (origin.0 + along * size, origin.1 + across * size)
            };
            let color = pattern.sample(across, along, step);
            ([x, y], self.lacquered(x, y, color, 1.0))
        });
    }

    /// One side of the frame: a strip from the outer edge to the field, mitred at the
    /// corners and rounded at the outer ones. `side`: 0 top, 1 right, 2 bottom, 3 left.
    fn frame_side(&self, mesh: &mut ColorMesh, side: u8, pattern: &Pattern) {
        let (left, top, length) = self.outer;
        let (right, bottom) = (left + length, top + length);
        let frame = self.frame;
        let radius = corner_radius(frame);
        let ppp = self.pixels_per_point;
        let columns = steps(frame, ppp, ACROSS_SPACING_PX, ACROSS_PER_CELL, self.cell);
        let rows = steps(
            length,
            ppp,
            ALONG_SPACING_PX,
            pattern.wood.along_per_cell() / 2.0,
            self.cell,
        );
        let normal = match side {
            0 => (0.0, -1.0),
            1 => (1.0, 0.0),
            2 => (0.0, 1.0),
            _ => (-1.0, 0.0),
        };
        let facing = normal.0 * LIGHT.0 + normal.1 * LIGHT.1;
        // The rounded outer edge catches the light on the lit sides.
        let bevel = (frame * 0.16).max(1e-3);
        let step = (
            frame / columns as f32 / self.cell,
            length / rows as f32 / self.cell,
        );
        mesh.add_grid(rows, columns, |along, across| {
            // The distance from the outer edge, and where a line at that distance begins:
            // at the mitre, or on the arc of the rounded corner near the outer edge.
            let depth = frame * across as f32 / columns as f32;
            let start = if depth < radius {
                depth.max(radius - (radius * radius - (radius - depth).powi(2)).max(0.0).sqrt())
            } else {
                depth
            };
            let run = start + (length - 2.0 * start) * along as f32 / rows as f32;
            let (x, y) = match side {
                0 => (left + run, top + depth),
                1 => (right - depth, top + run),
                2 => (right - run, bottom - depth),
                _ => (left + depth, bottom - run),
            };
            let rim = (1.0 - depth / bevel).max(0.0);
            let shade = 1.0 + 0.20 * facing * (rim * FRAC_PI_2).sin() - 0.10 * rim * rim;
            let color = pattern.sample(depth / self.cell, run / self.cell, step);
            ([x, y], self.lacquered(x, y, color, shade))
        });
    }
}

fn rgb(color: [u8; 3]) -> [f32; 3] {
    color.map(|value| f32::from(value) / 255.0)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    x
}

/// A pseudo-random number 0…1 for a seed and an index.
fn random(seed: u32, index: i32) -> f32 {
    (hash(seed ^ hash((index as u32).wrapping_add(0x9E37_79B9))) & 0x00FF_FFFF) as f32 / 16_777_215.0
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Smooth one-dimensional value noise, 0…1.
fn noise1(x: f32, seed: u32) -> f32 {
    let floor = x.floor();
    let t = smooth(x - floor);
    let index = floor as i32;
    let a = random(seed, index);
    let b = random(seed, index.wrapping_add(1));
    a + (b - a) * t
}

/// Smooth two-dimensional value noise, 0…1.
fn noise2(x: f32, y: f32, seed: u32) -> f32 {
    let (fx, fy) = (x.floor(), y.floor());
    let (tx, ty) = (smooth(x - fx), smooth(y - fy));
    let (ix, iy) = (fx as i32, fy as i32);
    let row = |j: i32| hash(seed ^ (j as u32).wrapping_mul(0x85EB_CA6B));
    let (upper, lower) = (row(iy), row(iy.wrapping_add(1)));
    let top = random(upper, ix) + (random(upper, ix.wrapping_add(1)) - random(upper, ix)) * tx;
    let bottom = random(lower, ix) + (random(lower, ix.wrapping_add(1)) - random(lower, ix)) * tx;
    top + (bottom - top) * ty
}

/// The average of the ring profile `phase^sharp` (it darkens through a ring and drops
/// back at its end) over `width` rings around `t`. Averaging over the distance between
/// two vertices keeps thin lines from breaking up into beads between the vertices.
fn filtered_profile(t: f32, width: f32, sharp: f32) -> f32 {
    let integral = |u: f32| {
        let whole = u.floor();
        (whole + (u - whole).powf(sharp + 1.0)) / (sharp + 1.0)
    };
    ((integral(t + width / 2.0) - integral(t - width / 2.0)) / width).clamp(0.0, 1.0)
}

/// The pattern of one piece of wood: where it was cut from the log.
struct Pattern<'a> {
    wood: &'a Wood,
    early: [f32; 3],
    late: [f32; 3],
    seed: u32,
    pith: (f32, f32),
    squash: f32,
    rings: f32,
    offset: f32,
    tone: f32,
}

impl<'a> Pattern<'a> {
    /// A piece `length` cells long along the grain.
    fn new(wood: &'a Wood, seed: u32, length: f32) -> Self {
        let r = |i| random(seed, i);
        let side = if r(1) < 0.5 { -1.0 } else { 1.0 };
        // The pith of the log: near the piece for figured wood (the rings show as
        // arches), far away for straight-grained wood (nearly parallel lines).
        let spread = 0.4 + 6.0 * (1.0 - wood.arch) * (1.0 - wood.arch);
        let pith_x = 0.5 + side * (0.25 + spread * r(2));
        let beyond = 0.4 + 1.6 * r(4);
        let pith_y = if r(3) < 0.5 { -beyond } else { length + beyond };
        Self {
            wood,
            early: rgb(wood.early),
            late: rgb(wood.late),
            seed: hash(seed ^ 0x51ED_270B),
            pith: (pith_x, pith_y),
            squash: 0.10 + 0.25 * wood.arch * r(5),
            rings: wood.rings * (0.85 + 0.3 * r(6)),
            offset: 20.0 * r(7),
            tone: 1.0 + wood.tone * (2.0 * r(8) - 1.0),
        }
    }

    /// Colour at `x` cells across and `y` cells along the grain; `step` is the distance
    /// between neighbouring vertices across and along, in cells.
    fn sample(&self, x: f32, y: f32, step: (f32, f32)) -> [f32; 3] {
        let w = self.wood;
        let dx = x - self.pith.0;
        let along = y - self.pith.1;
        let dy = along * self.squash;
        let distance = (dx * dx + dy * dy).sqrt().max(1e-4);
        let mut r = distance;
        r += w.meander * (2.0 * noise1(y * 0.8 + self.offset, self.seed ^ 0xA5) - 1.0);
        r += 0.5 * w.meander * (2.0 * noise2(x * 2.2, y * 1.3, self.seed ^ 0x5A) - 1.0);
        let mut rings = r * self.rings + self.offset;
        // Uneven years: the rings are not equally wide.
        rings += 0.35 * (2.0 * noise1(rings * 0.5, self.seed ^ 0x33) - 1.0);
        // How many rings one step between vertices spans.
        let gx = dx / distance;
        let gy = self.squash * self.squash * along / distance;
        let footprint = (self.rings * ((gx * step.0).powi(2) + (gy * step.1).powi(2)).sqrt()).max(1e-3);
        let year = rings.floor();
        let strength = 0.45 + 0.55 * random(self.seed ^ 0x77, year as i32);
        let late = w.contrast * strength * filtered_profile(rings, footprint, w.sharp);
        let fibre = 2.0 * noise2(rings * 22.0 / self.rings, y * 3.5, self.seed ^ 0x99) - 1.0;
        let figure = 2.0 * noise1(r * 2.2 + self.offset, self.seed ^ 0x2F) - 1.0;
        let ribbon = if w.ribbon == 0.0 {
            0.0
        } else {
            w.ribbon * (TAU * (x * 1.7 + 0.3 * noise1(y * 0.6, self.seed ^ 0x11))).sin()
        };
        let shade = self.tone * (1.0 + w.fibre * fibre + w.figure * figure + ribbon);
        mix(self.early, self.late, late).map(|value| value * shade)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(size: f32, pixels_per_point: f32, flipped: bool) -> BoardLayout {
        BoardLayout::fit(10.0, 20.0, size, size, flipped, pixels_per_point)
    }

    fn looks() -> impl Iterator<Item = (BoardStyle, &'static Look)> {
        BoardStyle::ALL
            .into_iter()
            .filter_map(|style| style.look().map(|look| (style, look)))
    }

    /// Relative luminance of an sRGB colour.
    fn luminance(color: [u8; 3]) -> f32 {
        let linear = |value: u8| {
            let c = f32::from(value) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color[0]) + 0.7152 * linear(color[1]) + 0.0722 * linear(color[2])
    }

    /// The total area of the triangles of a mesh.
    fn area(mesh: &ColorMesh) -> f64 {
        mesh.indices
            .chunks(3)
            .map(|triangle| {
                let [a, b, c] = [triangle[0], triangle[1], triangle[2]]
                    .map(|i| mesh.positions[i as usize].map(f64::from));
                ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
            })
            .sum()
    }

    /// CIE lightness (0…100) of a relative luminance.
    fn lightness(luminance: f32) -> f32 {
        if luminance > 0.008_856 {
            116.0 * luminance.cbrt() - 16.0
        } else {
            903.3 * luminance
        }
    }

    fn average(colors: &[[u8; 3]]) -> [u8; 3] {
        let n = colors.len().max(1) as f32;
        [0, 1, 2].map(|c| (colors.iter().map(|color| f32::from(color[c])).sum::<f32>() / n).round() as u8)
    }

    #[test]
    fn codes_round_trip_and_the_standard_board_is_the_default() {
        assert_eq!(BoardStyle::default(), BoardStyle::Standard);
        assert_eq!(BoardStyle::ALL[0], BoardStyle::Standard);
        for style in BoardStyle::ALL {
            assert_eq!(BoardStyle::from_code(style.code()), Some(style));
            assert_eq!(
                BoardStyle::from_code(&format!(" {} ", style.code().to_uppercase())),
                Some(style)
            );
        }
        for code in ["", "wood", "maple", "standart", "оак"] {
            assert_eq!(BoardStyle::from_code(code), None, "{code:?}");
        }
        let names: std::collections::HashSet<_> =
            BoardStyle::ALL.iter().map(|style| style.text_key()).collect();
        assert_eq!(names.len(), 6);
        assert!(BoardStyle::Standard.look().is_none());
        assert_eq!(looks().count(), 5);
    }

    #[test]
    fn the_cells_cover_the_field_exactly() {
        for (style, look) in looks() {
            for (size, ppp, flipped) in [(640.0, 1.0, false), (433.0, 1.5, true), (90.0, 2.0, false)] {
                let layout = layout(size, ppp, flipped);
                let mesh = look.cells_mesh(&layout, ppp);
                assert_eq!(mesh.positions.len(), mesh.colors.len());
                assert_eq!(mesh.indices.len() % 3, 0);
                assert!(mesh.indices.iter().all(|&i| (i as usize) < mesh.positions.len()));
                let (min_x, max_x, min_y, max_y) = mesh
                    .positions
                    .iter()
                    .fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |(a, b, c, d), p| {
                        (a.min(p[0]), b.max(p[0]), c.min(p[1]), d.max(p[1]))
                    });
                let field = layout.field_size();
                let eps = 1e-3;
                assert!((min_x - layout.field_x).abs() < eps, "{style:?}");
                assert!((min_y - layout.field_y).abs() < eps, "{style:?}");
                assert!((max_x - (layout.field_x + field)).abs() < eps, "{style:?}");
                assert!((max_y - (layout.field_y + field)).abs() < eps, "{style:?}");
                // The triangles add up to the area of the field: no gaps, no overlaps.
                let area = area(&mesh);
                assert!(
                    (area / f64::from(field * field) - 1.0).abs() < 1e-4,
                    "{style:?}: {area}"
                );
            }
        }
    }

    #[test]
    fn the_frame_lies_between_the_outer_edge_and_the_field() {
        for (style, look) in looks() {
            let layout = layout(700.0, 1.0, false);
            let mesh = look.frame_mesh(&layout, 1.0);
            assert!(mesh.indices.iter().all(|&i| (i as usize) < mesh.positions.len()));
            let outer = (layout.outer_x, layout.outer_y, layout.outer_size);
            let field = (layout.field_x, layout.field_y, layout.field_size());
            let eps = 1e-3;
            for p in &mesh.positions {
                assert!(
                    p[0] >= outer.0 - eps && p[0] <= outer.0 + outer.2 + eps,
                    "{style:?}"
                );
                assert!(
                    p[1] >= outer.1 - eps && p[1] <= outer.1 + outer.2 + eps,
                    "{style:?}"
                );
                let inside_field = p[0] > field.0 + eps
                    && p[0] < field.0 + field.2 - eps
                    && p[1] > field.1 + eps
                    && p[1] < field.1 + field.2 - eps;
                assert!(!inside_field, "{style:?}: {p:?} lies on the field");
            }
            // The area is the frame minus what the rounded corners cut off.
            let area = area(&mesh);
            let radius = corner_radius(layout.frame());
            let expected = outer.2 * outer.2
                - field.2 * field.2
                - 4.0 * radius * radius * (1.0 - std::f32::consts::FRAC_PI_4);
            assert!(
                (area / f64::from(expected) - 1.0).abs() < 2e-3,
                "{style:?}: {area} vs {expected}"
            );
        }
    }

    #[test]
    fn light_cells_are_clearly_lighter_than_dark_ones() {
        for (style, look) in looks() {
            let layout = layout(640.0, 1.0, false);
            let mesh = look.cells_mesh(&layout, 1.0);
            let mut light = Vec::new();
            let mut dark = Vec::new();
            for (p, &color) in mesh.positions.iter().zip(&mesh.colors) {
                let Some((file, rank)) = layout.cell_at(p[0] + 0.01, p[1] + 0.01) else {
                    continue;
                };
                if (file + rank) % 2 == 0 {
                    dark.push(color);
                } else {
                    light.push(color);
                }
            }
            let (light, dark) = (luminance(average(&light)), luminance(average(&dark)));
            // The cells stay apart without lines between them: their lightness differs by
            // more than a quarter of the scale.
            let difference = lightness(light) - lightness(dark);
            assert!(difference > 25.0, "{style:?}: lightness differs by {difference}");
            // Black pieces stand on dark cells: those must not be too dark either.
            assert!(
                lightness(dark) > 38.0,
                "{style:?}: dark cells {}",
                lightness(dark)
            );
        }
    }

    #[test]
    fn the_wood_has_a_visible_but_quiet_grain() {
        for (style, look) in looks() {
            let layout = layout(640.0, 1.0, false);
            let mesh = look.cells_mesh(&layout, 1.0);
            // The spread of brightness inside the a8 cell.
            let (x, y) = layout.cell_origin(0, 7);
            let inside: Vec<f32> = mesh
                .positions
                .iter()
                .zip(&mesh.colors)
                .filter(|(p, _)| {
                    p[0] > x + 1.0
                        && p[0] < x + layout.cell - 1.0
                        && p[1] > y + 1.0
                        && p[1] < y + layout.cell - 1.0
                })
                .map(|(_, &color)| luminance(color))
                .collect();
            let mean = inside.iter().sum::<f32>() / inside.len() as f32;
            let deviation =
                (inside.iter().map(|l| (l - mean).powi(2)).sum::<f32>() / inside.len() as f32).sqrt();
            let relative = deviation / mean;
            assert!(relative > 0.01, "{style:?}: the cell looks flat ({relative})");
            assert!(relative < 0.15, "{style:?}: the grain is too loud ({relative})");
        }
    }

    #[test]
    fn the_pattern_is_the_same_every_time_and_differs_between_cells() {
        let look = BoardStyle::MapleWalnut.look().unwrap();
        let layout = layout(500.0, 1.25, false);
        assert_eq!(look.cells_mesh(&layout, 1.25), look.cells_mesh(&layout, 1.25));
        assert_eq!(look.frame_mesh(&layout, 1.25), look.frame_mesh(&layout, 1.25));
        // Two light cells: the same wood, other pieces of it.
        let a = Pattern::new(&look.light, 1, 1.0);
        let b = Pattern::new(&look.light, 2, 1.0);
        let differs = (0..20).any(|i| {
            let x = i as f32 / 20.0;
            a.sample(x, 0.5, (0.02, 0.05)) != b.sample(x, 0.5, (0.02, 0.05))
        });
        assert!(differs);
    }

    #[test]
    fn the_board_looks_the_same_from_either_side() {
        // The pieces of wood belong to the places on the screen.
        for (style, look) in looks() {
            let straight = look.cells_mesh(&layout(400.0, 1.0, false), 1.0);
            let flipped = look.cells_mesh(&layout(400.0, 1.0, true), 1.0);
            assert_eq!(straight.positions, flipped.positions, "{style:?}");
            assert_eq!(straight.colors, flipped.colors, "{style:?}");
        }
    }

    #[test]
    fn the_number_of_vertices_does_not_grow_with_the_screen() {
        // Large and dense screens get longer steps, not more vertices.
        for (style, look) in looks() {
            let small = look.cells_mesh(&layout(640.0, 1.0, false), 1.0).positions.len()
                + look.frame_mesh(&layout(640.0, 1.0, false), 1.0).positions.len();
            let huge = look.cells_mesh(&layout(1400.0, 2.0, false), 2.0).positions.len()
                + look.frame_mesh(&layout(1400.0, 2.0, false), 2.0).positions.len();
            assert!(huge < 160_000, "{style:?}: {huge}");
            assert!(huge < small * 13 / 10, "{style:?}: {small} → {huge}");
        }
    }

    #[test]
    fn the_averaged_profile_matches_the_profile_and_its_mean() {
        for sharp in [2.0, 3.0, 5.0] {
            // A narrow window gives the profile itself.
            for t in [0.1_f32, 0.5, 0.8, 3.95] {
                let exact = (t - t.floor()).powf(sharp);
                assert!(
                    (filtered_profile(t, 1e-3, sharp) - exact).abs() < 0.01,
                    "{sharp} {t}"
                );
            }
            // A whole ring gives the mean of the profile wherever it starts.
            for t in [0.0, 0.3, 7.7] {
                let mean = 1.0 / (sharp + 1.0);
                assert!(
                    (filtered_profile(t, 1.0, sharp) - mean).abs() < 1e-3,
                    "{sharp} {t}"
                );
            }
        }
    }

    #[test]
    fn the_lacquer_shines_at_the_top_left_and_the_colours_stay_valid() {
        for (style, look) in looks() {
            let layout = layout(640.0, 1.0, false);
            let board = Board::new(&layout, 1.0, look.gloss);
            let grey = [0.5, 0.5, 0.5];
            let top_left = board.lacquered(
                layout.outer_x + 0.3 * layout.outer_size,
                layout.outer_y + 0.22 * layout.outer_size,
                grey,
                1.0,
            );
            let bottom_right = board.lacquered(
                layout.outer_x + 0.9 * layout.outer_size,
                layout.outer_y + 0.9 * layout.outer_size,
                grey,
                1.0,
            );
            assert!(top_left[0] > bottom_right[0], "{style:?}");
            // A reflection is soft: no more than a tenth of the full scale.
            assert!(f32::from(top_left[0]) - 127.5 < 0.1 * 255.0 + 6.0, "{style:?}");
        }
        // Damaged inputs give no panics and in-range colours.
        let look = BoardStyle::Oak.look().unwrap();
        for ppp in [0.0, f32::NAN, -1.0, 1.0] {
            let layout = layout(64.0, 1.0, false);
            let mesh = look.cells_mesh(&layout, ppp);
            assert_ne!(mesh.positions.len(), 0);
            assert!(mesh
                .positions
                .iter()
                .all(|p| p[0].is_finite() && p[1].is_finite()));
        }
    }
}
