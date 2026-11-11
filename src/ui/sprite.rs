//! Изображение шашки: вид строго сверху, попиксельный расчёт со сглаживанием.
//!
//! Шашка — круг, центр которого совпадает с центром клетки. Объём передан светотенью
//! (скруглённая кромка, бортик, проточки), а не смещением частей, поэтому силуэт
//! симметричен относительно обеих осей. Край круга сглажен по площади покрытия пикселя,
//! рельеф и корона — усреднением нескольких отсчётов на пиксель.
//!
//! Модуль только вычисляет пиксели; текстуры из них создаёт графическая часть.

use crate::board::{Color, Piece};
use std::f32::consts::{FRAC_PI_2, PI};

/// Радиус шашки в долях стороны клетки.
pub const PIECE_RADIUS: f32 = 0.40;

/// Квадратное изображение: RGBA с предумноженной прозрачностью, построчно сверху вниз.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PieceImage {
    /// Сторона изображения в пикселях.
    pub size: usize,
    /// Пиксели: по четыре байта (R, G, B, A).
    pub rgba: Vec<u8>,
}

impl PieceImage {
    /// Пиксель в столбце `x` и строке `y`; вне изображения — прозрачный.
    pub fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        if x >= self.size || y >= self.size {
            return [0; 4];
        }
        let i = 4 * (y * self.size + x);
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }

    /// Непрозрачность пикселя (0…255).
    pub fn alpha(&self, x: usize, y: usize) -> u8 {
        self.pixel(x, y)[3]
    }
}

/// Цвета шашки одного цвета.
struct Palette {
    /// Основной тон верхней грани.
    base: [f32; 3],
    /// Тон освещённых склонов.
    light: [f32; 3],
    /// Тон затенённых склонов.
    dark: [f32; 3],
    /// Тонкая кайма по краю.
    outline: [f32; 3],
    /// Корона и кольцо дамки.
    crown: [f32; 3],
    /// Блик на короне.
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

/// Направление на источник света: слева сверху (ось Y направлена вниз).
const LIGHT: (f32, f32) = (-0.555, -0.832);

/// Доли радиуса, задающие рельеф верхней грани.
const EDGE_START: f32 = 0.875; // начало скруглённой кромки
const RIM_INNER: f32 = 0.780; // внутренний край плоского бортика
const STEP_WIDTH: f32 = 0.050; // ширина ступеньки от бортика к утопленной середине
/// Проточки простой шашки: радиус середины и полуширина.
const GROOVES: [(f32, f32); 2] = [(0.560, 0.040), (0.330, 0.036)];
/// Кольцо дамки: внутренний и внешний радиусы.
const KING_RING: (f32, f32) = (0.655, 0.715);
/// Размер короны в долях радиуса шашки.
const CROWN_SCALE: f32 = 0.98;
/// Корона несимметрична по высоте; сдвиг ставит её середину в центр шашки.
const CROWN_SHIFT: f32 = 0.0475;

/// Тень лежащей шашки: смещение центра и ширина размытия в долях клетки, плотность.
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

/// Освещённость точки верхней грани: от −1 (тень) до +1 (блик).
///
/// `r` — расстояние от центра в долях радиуса, `toward_light` — косинус угла между
/// направлением от центра к точке и направлением на свет.
fn relief(r: f32, toward_light: f32, king: bool) -> f32 {
    let r = r.clamp(0.0, 1.0);
    // Едва заметный общий перепад: сторона, обращённая к свету, чуть светлее.
    let mut shade = 0.10 * toward_light * r;
    if r > EDGE_START {
        // Скруглённая кромка: склон наружу; к самому краю поверхность уходит от зрителя.
        let e = (r - EDGE_START) / (1.0 - EDGE_START);
        shade += 0.80 * toward_light * (e * FRAC_PI_2).sin() - 0.20 * e * e;
    } else if r > RIM_INNER {
        // Плоский бортик.
        shade += 0.04;
    } else if r > RIM_INNER - STEP_WIDTH {
        // Ступенька вниз к середине: склон обращён к центру.
        let q = (r - (RIM_INNER - STEP_WIDTH)) / STEP_WIDTH;
        shade -= 0.55 * toward_light * (PI * q).sin();
    } else if !king {
        for (middle, half_width) in GROOVES {
            let q = (r - middle) / half_width;
            if q.abs() < 1.0 {
                // Проточка: внутренний склон обращён наружу, внешний — к центру.
                let depth = (FRAC_PI_2 * q).cos();
                shade -= 0.50 * toward_light * (PI * q).sin() + 0.17 * depth * depth;
            }
        }
    }
    shade.clamp(-1.0, 1.0)
}

/// Принадлежит ли точка короне. Координаты — в долях размера короны, центр в нуле.
///
/// Корона задана правой половиной и отражается относительно вертикальной оси,
/// поэтому она симметрична точно, а не с точностью до округления.
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
    // Обод и подставка.
    if x <= 0.50 && ((0.08..=0.26).contains(&y) || (0.33..=0.42).contains(&y)) {
        return true;
    }
    // Половина среднего зубца и боковой зубец.
    let p = (x, y);
    if in_triangle(p, (0.00, 0.10), (0.00, -0.44), (0.28, 0.10))
        || in_triangle(p, (0.10, 0.10), (0.50, -0.34), (0.50, 0.10))
    {
        return true;
    }
    // Шарики на вершинах зубцов.
    x.hypot(y + 0.44) <= 0.075 || (x - 0.50).hypot(y + 0.34) <= 0.075
}

/// Доля пикселя с центром в `(px, py)`, покрытая короной (16 отсчётов).
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

/// Число отсчётов рельефа на пиксель по каждой оси: мелкие шашки считаются точнее,
/// у крупных детали рельефа и так шире пикселя.
fn relief_samples(size: usize) -> usize {
    match size {
        0..=63 => 3,
        64..=159 => 2,
        _ => 1,
    }
}

/// Рисует шашку в квадрате `size × size` пикселей (сторона клетки доски).
///
/// Центр шашки — в центре изображения. `shadow` добавляет мягкую тень под шашкой;
/// сама шашка от этого не меняется.
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
            // Сглаживание края: доля пикселя, покрытая кругом.
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
                // Рельеф усредняется по нескольким отсчётам внутри пикселя.
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
                    // Кольцо цвета короны.
                    let ring = (distance - KING_RING.0 * radius + 0.5).clamp(0.0, 1.0)
                        * (KING_RING.1 * radius - distance + 0.5).clamp(0.0, 1.0);
                    color = mix(color, palette.crown, ring);
                    // Корона с лёгкой тенью под ней.
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

                // Тонкая тёмная кайма отделяет шашку от поля.
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

/// Мягкая тень под шашкой, которую держат мышью: размытый тёмный круг без самой шашки.
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
        // Шашка не «скошена»: её очертание одинаково слева и справа, сверху и снизу.
        for piece in pieces() {
            for size in SIZES {
                let image = render_piece(piece, size, false);
                for y in 0..size {
                    for x in 0..size {
                        let a = image.alpha(x, y);
                        assert_eq!(a, image.alpha(size - 1 - x, y), "{piece:?} {size}: лево-право");
                        assert_eq!(a, image.alpha(x, size - 1 - y), "{piece:?} {size}: верх-низ");
                        assert_eq!(a, image.alpha(y, x), "{piece:?} {size}: диагональ");
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
                    "{piece:?} {size}: центр по X"
                );
                assert!(
                    (sum_y / sum - middle).abs() < 1e-6,
                    "{piece:?} {size}: центр по Y"
                );
                // Поля до краёв клетки одинаковы со всех сторон.
                assert_eq!(min_x, size - 1 - max_x, "{piece:?} {size}");
                assert_eq!(min_y, size - 1 - max_y, "{piece:?} {size}");
                assert_eq!(min_x, min_y, "{piece:?} {size}");
                // Диаметр соответствует заданному радиусу.
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
                // По всей окружности идёт полоса полупрозрачных пикселей…
                assert!(
                    partial as f32 > 0.8 * 2.0 * PI * radius,
                    "{piece:?} {size}: {partial} полупрозрачных"
                );
                // …и непрозрачный пиксель не соседствует с пустым (без сглаживания таких
                // переходов было бы порядка восьми радиусов).
                assert!(hard_steps <= 8, "{piece:?} {size}: {hard_steps} резких переходов");
            }
        }
    }

    #[test]
    fn shading_has_no_hard_steps_inside_the_piece() {
        // Соседние пиксели верхней грани простой шашки близки по яркости:
        // проточки и бортик прорисованы плавно, без «лесенок».
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
                assert!(worst <= 70, "{color:?} {size}: перепад {worst}");
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
                    assert_eq!(&p[..3], &s[..3], "тень не меняет цвет шашки");
                    assert!(s[3] >= p[3]);
                    if p[3] == 255 {
                        assert_eq!(p, s);
                    }
                    if p[3] == 0 && s[3] > 0 {
                        shadow_pixels += 1;
                        assert!(s[3] < 128, "тень полупрозрачна");
                    }
                }
            }
            assert!(shadow_pixels > 100, "{piece:?}: тень видна вокруг шашки");
            // Изображение не обрезано краем клетки.
            for i in 0..size {
                for (x, y) in [(i, 0), (i, size - 1), (0, i), (size - 1, i)] {
                    assert_eq!(shaded.alpha(x, y), 0, "{piece:?}: край ({x}, {y})");
                }
            }
        }
        let lift = render_lift_shadow(96);
        assert!(lift.alpha(48, 48) > 60 && lift.alpha(48, 48) < 128);
        assert_eq!(lift.alpha(0, 48), 0);
        assert!(lift.rgba.chunks(4).all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0));
    }

    /// Число золотистых пикселей (цвет короны и кольца дамки).
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
                // Очертание одно и то же, отличается рисунок.
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
                    "{color:?} {size}: отличается {differing} пикселей"
                );
                assert_eq!(golden_pixels(&man), 0, "{color:?} {size}");
                assert!(
                    golden_pixels(&king) as f32 > 0.08 * area,
                    "{color:?} {size}: корона заметна"
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
        assert!(white > 600 && black < 300, "{white} и {black}");
    }

    #[test]
    fn crown_is_centered_on_the_piece() {
        // Корона симметрична слева направо и стоит посередине шашки по высоте.
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
        let middle = (top + bottom) / 2.0 + CROWN_SHIFT;
        assert!(
            (middle * half).abs() < 0.5,
            "середина короны смещена на {} пикс.",
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
                "расчёт детерминирован"
            );
        }
        assert_eq!(render_lift_shadow(0).rgba.len(), 0);
        assert_eq!(render_lift_shadow(1).rgba.len(), 4);
    }
}
