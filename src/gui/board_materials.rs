//! Procedural lacquered wood studies, drawn by the same egui painter as the game.
//!
//! Only coloured triangles and lines: no board images, pixel synthesis, filters or
//! custom shaders. The geometry is cached until the material, layout or scale changes.
//! `examples/board_designs.rs` uses this module directly while the designs are chosen.

use eframe::egui::{pos2, vec2, Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke};
use rucheckers::ui::geometry::BoardLayout;
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Construction {
    Slab,
    Blocks,
    Parquet,
    EndGrain,
    Figured,
}

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub name: &'static str,
    pub description: &'static str,
    pub construction: Construction,
    light: u32,
    dark: u32,
    frame: u32,
    grain: f32,
    bend: f32,
    gloss: f32,
    horizontal: bool,
    inlay: bool,
    grid: bool,
    seed: u32,
}

pub const MATERIALS: [Material; 12] = [
    Material {
        name: "Клён и орех",
        description: "Светлый клён, тёплый орех и широкая ореховая рамка.",
        construction: Construction::Blocks,
        light: 0xf1_df_a9,
        dark: 0xae_80_58,
        frame: 0x79_53_36,
        grain: 0.94,
        bend: 0.9,
        gloss: 0.60,
        horizontal: false,
        inlay: false,
        grid: false,
        seed: 31,
    },
    Material {
        name: "Бук и красное дерево",
        description: "Сливочный бук и красно-коричневые наборные клетки.",
        construction: Construction::Blocks,
        light: 0xf3_dc_b0,
        dark: 0xb7_6e_4b,
        frame: 0x87_3f_2a,
        grain: 0.82,
        bend: 0.55,
        gloss: 0.70,
        horizontal: false,
        inlay: false,
        grid: false,
        seed: 73,
    },
    Material {
        name: "Дуб и морёный дуб",
        description: "Цельная поверхность под светлым и дымчатым лаком.",
        construction: Construction::Slab,
        light: 0xe4_d2_a2,
        dark: 0x97_83_6b,
        frame: 0x60_50_3a,
        grain: 1.25,
        bend: 1.45,
        gloss: 0.55,
        horizontal: false,
        inlay: false,
        grid: false,
        seed: 127,
    },
    Material {
        name: "Ясень и вишня · паркет",
        description: "Волокна соседних клеток развёрнуты поперёк друг друга.",
        construction: Construction::Parquet,
        light: 0xf0_df_b0,
        dark: 0xb5_7b_52,
        frame: 0x87_4b_30,
        grain: 1.00,
        bend: 1.20,
        gloss: 0.60,
        horizontal: false,
        inlay: true,
        grid: false,
        seed: 58,
    },
    Material {
        name: "Берёза · янтарный лак",
        description: "Единые мягкие волокна под медовым и коньячным лаком.",
        construction: Construction::Slab,
        light: 0xf4_d7_94,
        dark: 0xbc_80_45,
        frame: 0x92_53_2c,
        grain: 0.85,
        bend: 1.00,
        gloss: 0.90,
        horizontal: false,
        inlay: false,
        grid: false,
        seed: 219,
    },
    Material {
        name: "Цельный золотистый дуб",
        description: "Непрерывный рисунок массива и спокойный золотой блеск.",
        construction: Construction::Slab,
        light: 0xe6_c3_8a,
        dark: 0x9c_69_3d,
        frame: 0x76_50_2f,
        grain: 1.05,
        bend: 0.80,
        gloss: 0.65,
        horizontal: false,
        inlay: false,
        grid: false,
        seed: 42,
    },
    Material {
        name: "Цельный дымчатый орех",
        description: "Длинные горизонтальные волокна и глубокий коричневый тон.",
        construction: Construction::Slab,
        light: 0xc6_a2_79,
        dark: 0x80_60_44,
        frame: 0x59_41_2e,
        grain: 1.10,
        bend: 1.55,
        gloss: 0.55,
        horizontal: true,
        inlay: false,
        grid: false,
        seed: 83,
    },
    Material {
        name: "Торцевой массив",
        description: "Набор дубовых и ореховых брусков с дугами годовых колец.",
        construction: Construction::EndGrain,
        light: 0xdc_bc_86,
        dark: 0x96_71_4c,
        frame: 0x6e_4b_30,
        grain: 0.90,
        bend: 1.00,
        gloss: 0.55,
        horizontal: false,
        inlay: false,
        grid: false,
        seed: 104,
    },
    Material {
        name: "Фигурный орех",
        description: "Выразительный волнистый шпон под гладким прозрачным лаком.",
        construction: Construction::Figured,
        light: 0xdd_be_88,
        dark: 0x98_70_4c,
        frame: 0x65_43_2c,
        grain: 1.15,
        bend: 2.20,
        gloss: 0.85,
        horizontal: false,
        inlay: true,
        grid: false,
        seed: 59,
    },
    Material {
        name: "Клён и венге",
        description: "Контрастная наборная доска с тонкими тёмными волокнами.",
        construction: Construction::Blocks,
        light: 0xe5_d1_a1,
        dark: 0x65_51_3b,
        frame: 0x49_37_28,
        grain: 1.20,
        bend: 0.45,
        gloss: 0.75,
        horizontal: false,
        inlay: true,
        grid: false,
        seed: 164,
    },
    Material {
        name: "Светлый ясень · сатин",
        description: "Светлая цельная доска и едва заметные границы клеток.",
        construction: Construction::Slab,
        light: 0xe5_de_bd,
        dark: 0xb3_a4_84,
        frame: 0x8c_7b_59,
        grain: 0.85,
        bend: 0.80,
        gloss: 0.40,
        horizontal: false,
        inlay: false,
        grid: true,
        seed: 91,
    },
    Material {
        name: "Оливковый массив",
        description: "Живые извилистые волокна проходят через светлые и тёмные поля.",
        construction: Construction::Slab,
        light: 0xe0_c3_8a,
        dark: 0x9f_78_47,
        frame: 0x78_60_3b,
        grain: 1.25,
        bend: 2.40,
        gloss: 0.75,
        horizontal: false,
        inlay: false,
        grid: false,
        seed: 308,
    },
];

fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

fn tone(color: Color32, amount: f32) -> Color32 {
    let c = |v: u8| {
        (f32::from(v)
            + if amount > 0.0 {
                255.0 - f32::from(v)
            } else {
                f32::from(v)
            } * amount)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(c(color.r()), c(color.g()), c(color.b()))
}

fn alpha(color: Color32, opacity: f32) -> Color32 {
    // Match egui_glow's gamma-space blending and board::blended. Linear-space
    // premultiplication makes pale, translucent fibres add light to dark cells.
    let opacity = opacity.clamp(0.0, 1.0);
    let channel = |value: u8| (f32::from(value) * opacity).round() as u8;
    Color32::from_rgba_premultiplied(
        channel(color.r()),
        channel(color.g()),
        channel(color.b()),
        (opacity * 255.0).round() as u8,
    )
}

fn noise(mut key: u32) -> f32 {
    key = key.wrapping_mul(0x9e37_79b9) ^ 0x85eb_ca6b;
    key ^= key >> 15;
    key = key.wrapping_mul(0x2c1b_3c6d);
    key ^= key >> 12;
    (key & 0xffff) as f32 / 65535.0
}

/// A bilinear surface. Frame rails are trapezoids with real mitred corners.
#[derive(Clone, Copy)]
struct Patch([Pos2; 4]);

impl Patch {
    fn rect(rect: Rect, horizontal: bool) -> Self {
        if horizontal {
            Self([
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
            ])
        } else {
            Self([
                rect.left_top(),
                rect.left_bottom(),
                rect.right_bottom(),
                rect.right_top(),
            ])
        }
    }

    fn at(self, u: f32, v: f32) -> Pos2 {
        let a = self.0[0].lerp(self.0[1], u);
        let b = self.0[3].lerp(self.0[2], u);
        a.lerp(b, v)
    }
}

fn quad(mesh: &mut Mesh, points: [Pos2; 4], colors: [Color32; 4]) {
    let base = mesh.vertices.len() as u32;
    for (position, color) in points.into_iter().zip(colors) {
        mesh.colored_vertex(position, color);
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base, base + 2, base + 3);
}

fn strip(mesh: &mut Mesh, patch: Patch, from: f32, to: f32, a: Color32, b: Color32) {
    quad(
        mesh,
        [
            patch.at(0.0, from),
            patch.at(1.0, from),
            patch.at(1.0, to),
            patch.at(0.0, to),
        ],
        [a, a, b, b],
    );
}

fn warp(u: f32, v: f32, seed: u32, bend: f32) -> f32 {
    let phase = noise(seed) * TAU;
    let sweep = (u * 4.6 + phase).sin() * 0.65 + (u * 8.2 + phase * 0.7).sin() * 0.20;
    let arch = (-((u - 0.53) * 4.0).powi(2)).exp() * ((v - 0.44) * 5.0).tanh();
    (sweep + arch * 0.95) * 0.033 * bend
}

/// Soft, wide bands supply most of the texture. Hairlines add a few fine pores.
/// The ends are transparent, so the wood has depth without looking corrugated.
fn grain(mesh: &mut Mesh, patch: Patch, count: u32, seed: u32, strength: f32, bend: f32) {
    let steps = ((patch.0[1] - patch.0[0]).length() / 20.0)
        .ceil()
        .clamp(12.0, 48.0) as u32;
    for i in 0..count + 2 {
        let key = seed.wrapping_add(i * 73);
        let center = (i as f32 - 0.5 + noise(key) * 0.65) / count as f32;
        let width = (0.26 + 0.34 * noise(key + 1)) / count as f32;
        let opacity = (0.10 + 0.13 * noise(key + 2)) * strength;
        for step in 0..steps {
            let u0 = step as f32 / steps as f32;
            let u1 = (step + 1) as f32 / steps as f32;
            let a = center + warp(u0, center, seed, bend);
            let b = center + warp(u1, center, seed, bend);
            // A warm highlight along one side and a soft, asymmetrical dark pore.
            let stops = [(-1.0, 0.0), (0.0, 1.0), (1.0, 0.0)];
            for pair in stops.windows(2) {
                let v0 = (a + pair[0].0 * width).clamp(0.0, 1.0);
                let v1 = (a + pair[1].0 * width).clamp(0.0, 1.0);
                let w0 = (b + pair[0].0 * width).clamp(0.0, 1.0);
                let w1 = (b + pair[1].0 * width).clamp(0.0, 1.0);
                let c0 = alpha(rgb(0x51_32_1b), opacity * pair[0].1);
                let c1 = alpha(rgb(0x51_32_1b), opacity * pair[1].1);
                quad(
                    mesh,
                    [
                        patch.at(u0, v0),
                        patch.at(u1, w0),
                        patch.at(u1, w1),
                        patch.at(u0, v1),
                    ],
                    [c0, c0, c1, c1],
                );
            }
            let h = width * 0.8;
            let c = alpha(rgb(0xff_ed_b8), opacity * 0.31);
            quad(
                mesh,
                [
                    patch.at(u0, (a - h * 1.8).clamp(0.0, 1.0)),
                    patch.at(u1, (b - h * 1.8).clamp(0.0, 1.0)),
                    patch.at(u1, (b - h * 0.6).clamp(0.0, 1.0)),
                    patch.at(u0, (a - h * 0.6).clamp(0.0, 1.0)),
                ],
                [Color32::TRANSPARENT, Color32::TRANSPARENT, c, c],
            );
        }
    }
    for i in 0..count * 3 {
        let key = seed.wrapping_add(i * 101 + 900);
        let center = noise(key);
        let half_width = (0.010 + 0.014 * noise(key + 2)) / count as f32;
        let color = alpha(rgb(0x5c_3b_22), (0.035 + 0.035 * noise(key + 1)) * strength);
        let start = noise(key + 3) * 0.08;
        for step in 0..12 {
            let u0 = start + (1.0 - start) * step as f32 / 12.0;
            let u1 = start + (1.0 - start) * (step + 1) as f32 / 12.0;
            let a = center + warp(u0, center, seed, bend);
            let b = center + warp(u1, center, seed, bend);
            quad(
                mesh,
                [
                    patch.at(u0, (a - half_width).clamp(0.0, 1.0)),
                    patch.at(u1, (b - half_width).clamp(0.0, 1.0)),
                    patch.at(u1, (b + half_width).clamp(0.0, 1.0)),
                    patch.at(u0, (a + half_width).clamp(0.0, 1.0)),
                ],
                [color; 4],
            );
        }
    }
}

fn end_grain(mesh: &mut Mesh, rect: Rect, seed: u32, strength: f32) {
    let center = rect.min + rect.size() * vec2(noise(seed) * 1.1 - 0.05, noise(seed + 1) * 1.1 - 0.05);
    let scale = rect.width();
    let squash = 0.7 + noise(seed + 2) * 0.55;
    for i in 1..12 {
        let radius = (i as f32 * 0.135 + noise(seed + i) * 0.025) * scale;
        let half = scale * (0.006 + noise(seed + i + 30) * 0.006);
        let c = alpha(rgb(0x54_35_1f), 0.17 * strength);
        for j in 0..40 {
            let at = |k: u32, offset: f32| {
                let a = k as f32 / 40.0 * TAU;
                let r = radius
                    * (1.0 + 0.035 * (a * 3.0 + noise(seed) * TAU).sin() + 0.014 * (a * 7.0).sin())
                    + offset;
                center + vec2(a.cos(), a.sin() * squash) * r
            };
            quad(
                mesh,
                [at(j, -half), at(j + 1, -half), at(j + 1, 0.0), at(j, 0.0)],
                [Color32::TRANSPARENT, Color32::TRANSPARENT, c, c],
            );
            quad(
                mesh,
                [at(j, 0.0), at(j + 1, 0.0), at(j + 1, half), at(j, half)],
                [c, c, Color32::TRANSPARENT, Color32::TRANSPARENT],
            );
        }
    }
}

#[derive(Clone)]
struct Layer {
    clip: Rect,
    shape: Shape,
}

fn layer(layers: &mut Vec<Layer>, clip: Rect, mesh: Mesh) {
    layers.push(Layer {
        clip,
        shape: Shape::mesh(mesh),
    });
}

fn frame(layers: &mut Vec<Layer>, outer: Rect, width: f32, material: Material) {
    let inner = outer.shrink(width);
    let patches = [
        Patch([
            outer.left_top(),
            outer.right_top(),
            inner.right_top(),
            inner.left_top(),
        ]),
        Patch([
            outer.left_bottom(),
            outer.left_top(),
            inner.left_top(),
            inner.left_bottom(),
        ]),
        Patch([
            outer.right_bottom(),
            outer.left_bottom(),
            inner.left_bottom(),
            inner.right_bottom(),
        ]),
        Patch([
            outer.right_top(),
            outer.right_bottom(),
            inner.right_bottom(),
            inner.right_top(),
        ]),
    ];
    let base = rgb(material.frame);
    for (side, patch) in patches.into_iter().enumerate() {
        let mut mesh = Mesh::default();
        let light = if side < 2 { 0.035 } else { -0.035 };
        let profile = [
            (0.0, -0.36),
            (0.025, -0.08),
            (0.06, 0.15),
            (0.13, 0.07),
            (0.67, -0.03),
            (0.84, -0.18),
            (0.90, -0.34),
            (0.93, 0.08),
            (0.96, -0.36),
            (1.0, -0.48),
        ];
        for pair in profile.windows(2) {
            strip(
                &mut mesh,
                patch,
                pair[0].0,
                pair[1].0,
                tone(base, pair[0].1 + light),
                tone(base, pair[1].1 + light),
            );
        }
        grain(
            &mut mesh,
            patch,
            8,
            material.seed + side as u32 * 91,
            material.grain * 0.82,
            0.7,
        );
        if material.inlay {
            strip(
                &mut mesh,
                patch,
                0.905,
                0.925,
                tone(rgb(material.light), -0.15),
                tone(rgb(material.light), -0.27),
            );
        }
        layer(layers, outer, mesh);
        layers.push(Layer {
            clip: outer,
            shape: Shape::line_segment(
                [patch.0[0], patch.0[3]],
                Stroke::new(0.75, alpha(rgb(0x2b_1a_0c), 0.27)),
            ),
        });
    }
}

fn playing_field(layers: &mut Vec<Layer>, layout: &BoardLayout, field: Rect, material: Material) {
    let mut cells = Mesh::default();
    for row in 0..8 {
        for column in 0..8 {
            let rect = Rect::from_min_size(
                field.min + vec2(column as f32, row as f32) * layout.cell,
                vec2(layout.cell, layout.cell),
            );
            let base = rgb(if (row + column) % 2 == 0 {
                material.light
            } else {
                material.dark
            });
            let tint = if material.construction == Construction::Slab {
                0.0
            } else {
                (noise(material.seed + row * 31 + column * 17) - 0.5) * 0.035
            };
            strip(
                &mut cells,
                Patch::rect(rect, true),
                0.0,
                1.0,
                tone(base, tint),
                tone(base, tint),
            );
        }
    }
    layer(layers, field, cells);
    if material.construction == Construction::Slab {
        let mut mesh = Mesh::default();
        grain(
            &mut mesh,
            Patch::rect(field, material.horizontal),
            54,
            material.seed,
            material.grain,
            material.bend * 0.75,
        );
        layer(layers, field, mesh);
    } else {
        for row in 0..8 {
            for column in 0..8 {
                let rect = Rect::from_min_size(
                    field.min + vec2(column as f32, row as f32) * layout.cell,
                    vec2(layout.cell, layout.cell),
                );
                let seed = material.seed + row * 137 + column * 47;
                let mut mesh = Mesh::default();
                if material.construction == Construction::EndGrain {
                    end_grain(&mut mesh, rect, seed, material.grain);
                } else {
                    let horizontal =
                        material.construction == Construction::Parquet && (row + column) % 2 == 0;
                    grain(
                        &mut mesh,
                        Patch::rect(rect, horizontal),
                        6,
                        seed,
                        material.grain,
                        material.bend,
                    );
                }
                layer(layers, rect, mesh);
            }
        }
    }
}

fn geometry(layout: &BoardLayout, material: Material, ppp: f32) -> Vec<Layer> {
    let outer = Rect::from_min_size(
        pos2(layout.outer_x, layout.outer_y),
        vec2(layout.outer_size, layout.outer_size),
    );
    let field = Rect::from_min_size(
        pos2(layout.field_x, layout.field_y),
        vec2(layout.field_size(), layout.field_size()),
    );
    let mut layers = Vec::new();
    let pixel = 1.0 / ppp.max(0.1);
    for i in (1..=5).rev() {
        let shadow = outer
            .translate(vec2(0.0, layout.frame() * 0.08))
            .expand(i as f32 * pixel * 0.8);
        layers.push(Layer {
            clip: shadow.expand(2.0),
            shape: Shape::rect_filled(shadow, 3.0, Color32::from_black_alpha(9)),
        });
    }
    playing_field(&mut layers, layout, field, material);
    let mut finish = Mesh::default();
    let field_patch = Patch::rect(field, true);
    strip(
        &mut finish,
        field_patch,
        0.0,
        1.0,
        alpha(rgb(0xff_e9_bc), 0.045),
        alpha(rgb(0x2e_1c_0e), 0.065),
    );
    // A single broad lacquer reflection over the whole field, not 64 plastic tiles.
    for row in 0..24 {
        for column in 0..24 {
            let a = column as f32 / 24.0;
            let b = row as f32 / 24.0;
            let coords = [
                (a, b),
                (a + 1.0 / 24.0, b),
                (a + 1.0 / 24.0, b + 1.0 / 24.0),
                (a, b + 1.0 / 24.0),
            ];
            let colors = coords.map(|(u, v)| {
                let band = (-((u * 0.48 + v * 0.88 - 0.29) / 0.17).powi(2)).exp();
                alpha(rgb(0xff_f4_d9), band * 0.17 * material.gloss)
            });
            quad(&mut finish, coords.map(|(u, v)| field_patch.at(u, v)), colors);
        }
    }
    layer(&mut layers, field, finish);
    if material.grid {
        for i in 1..8 {
            let x = field.left() + i as f32 * layout.cell;
            let y = field.top() + i as f32 * layout.cell;
            for points in [
                [pos2(x, field.top()), pos2(x, field.bottom())],
                [pos2(field.left(), y), pos2(field.right(), y)],
            ] {
                layers.push(Layer {
                    clip: field,
                    shape: Shape::line_segment(
                        points,
                        Stroke::new(pixel * 0.7, alpha(rgb(0x55_46_2e), 0.20)),
                    ),
                });
            }
        }
    }
    frame(&mut layers, outer, layout.frame(), material);
    // Contact shadow makes the field sit below the raised inner lip.
    let mut shadow = Mesh::default();
    let shadow_width = (layout.cell * 0.045).max(pixel);
    strip(
        &mut shadow,
        Patch::rect(
            Rect::from_min_max(field.min, pos2(field.right(), field.top() + shadow_width)),
            true,
        ),
        0.0,
        1.0,
        Color32::from_black_alpha(53),
        Color32::TRANSPARENT,
    );
    strip(
        &mut shadow,
        Patch::rect(
            Rect::from_min_max(field.min, pos2(field.left() + shadow_width, field.bottom())),
            false,
        ),
        0.0,
        1.0,
        Color32::from_black_alpha(42),
        Color32::TRANSPARENT,
    );
    layer(&mut layers, field, shadow);
    layers
}

/// Retains geometry between frames; selecting a material or resizing rebuilds once.
#[derive(Default)]
pub struct BoardArt {
    key: Option<(BoardLayout, usize, f32)>,
    layers: Vec<Layer>,
}

impl BoardArt {
    pub fn draw(&mut self, painter: &Painter, layout: &BoardLayout, style: usize, pixels_per_point: f32) {
        let style = style.min(MATERIALS.len() - 1);
        let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
            pixels_per_point
        } else {
            1.0
        };
        let key = (*layout, style, ppp);
        if self.key != Some(key) {
            self.layers = geometry(layout, MATERIALS[style], ppp);
            self.key = Some(key);
        }
        for layer in &self.layers {
            painter.with_clip_rect(layer.clip).add(layer.shape.clone());
        }
        let frame = layout.frame();
        if frame * 0.52 < 6.0 {
            return;
        }
        let color = rgb(0xee_dc_ba);
        for i in 0..8u8 {
            let along = (f32::from(i) + 0.5) * layout.cell;
            for y in [
                layout.outer_y + frame * 0.49,
                layout.outer_y + layout.outer_size - frame * 0.51,
            ] {
                painter.text(
                    pos2(layout.field_x + along, y),
                    Align2::CENTER_CENTER,
                    char::from(b'a' + layout.file_of_column(i)),
                    FontId::proportional(frame * 0.52),
                    color,
                );
            }
            for x in [
                layout.outer_x + frame * 0.49,
                layout.outer_x + layout.outer_size - frame * 0.51,
            ] {
                painter.text(
                    pos2(x, layout.field_y + along),
                    Align2::CENTER_CENTER,
                    char::from(b'1' + layout.rank_of_row(i)),
                    FontId::proportional(frame * 0.52),
                    color,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_material_has_finite_bounded_vector_geometry_at_small_and_large_scales() {
        for size in [96.0, 1000.0, 2400.0] {
            for material in MATERIALS {
                let layout = BoardLayout::fit(7.0, 13.0, size, size, false, 1.5);
                let layers = geometry(&layout, material, 1.5);
                let mut vertices = 0;
                for layer in layers {
                    assert!(layer.clip.is_finite());
                    if let Shape::Mesh(mesh) = layer.shape {
                        assert_eq!(
                            mesh.texture_id,
                            eframe::egui::TextureId::default(),
                            "board must not use a texture asset"
                        );
                        assert!(mesh.is_valid());
                        assert!(mesh
                            .vertices
                            .iter()
                            .all(|v| v.pos.x.is_finite() && v.pos.y.is_finite()));
                        vertices += mesh.vertices.len();
                    }
                }
                assert!(vertices < 260_000, "{}: {vertices}", material.name);
            }
        }
    }

    #[test]
    fn translucent_wood_colors_do_not_add_light_in_the_gamma_blending_pipeline() {
        for color in [rgb(0xff_ed_b8), rgb(0x51_32_1b), rgb(0x2e_1c_0e)] {
            assert_eq!(alpha(color, 0.0), Color32::TRANSPARENT);
            assert_eq!(alpha(color, 1.0), color);
            for opacity in [0.04, 0.1, 0.3] {
                let c = alpha(color, opacity);
                assert!(c.r() <= c.a() && c.g() <= c.a() && c.b() <= c.a());
            }
        }
    }
}
