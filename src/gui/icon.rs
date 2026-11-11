//! Значок окна: создаётся программно, без файлов ресурсов.

use eframe::egui::IconData;
use rucheckers::board::{Color, Kind, Piece};
use rucheckers::ui::sprite::render_piece;

const SIZE: usize = 128;
/// Сглаживание края фона: число отсчётов по каждой оси на пиксель.
const SAMPLES: usize = 3;
/// Цвет тёмного поля под шашкой.
const SQUARE: [f32; 3] = [0.63, 0.44, 0.25];

/// Принадлежит ли точка (в долях стороны значка) скруглённому квадрату фона.
fn inside_square(x: f32, y: f32) -> bool {
    let (u, v) = (x * 2.0 - 1.0, y * 2.0 - 1.0);
    let corner = 0.22;
    let (qx, qy) = (
        (u.abs() - (0.94 - corner)).max(0.0),
        (v.abs() - (0.94 - corner)).max(0.0),
    );
    qx.hypot(qy) <= corner
}

/// Значок приложения: белая шашка на тёмном поле — та же, что на доске.
pub fn app_icon() -> IconData {
    let piece = render_piece(Piece::new(Color::White, Kind::Man), SIZE, true);
    let mut rgba = Vec::with_capacity(SIZE * SIZE * 4);
    let step = 1.0 / (SIZE * SAMPLES) as f32;
    for py in 0..SIZE {
        for px in 0..SIZE {
            let mut inside = 0;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = ((px * SAMPLES + sx) as f32 + 0.5) * step;
                    let y = ((py * SAMPLES + sy) as f32 + 0.5) * step;
                    inside += usize::from(inside_square(x, y));
                }
            }
            let background = inside as f32 / (SAMPLES * SAMPLES) as f32;
            // Шашка (с предумноженной прозрачностью) накладывается поверх фона.
            let top = piece.pixel(px, py);
            let top_alpha = f32::from(top[3]) / 255.0;
            let alpha = top_alpha + background * (1.0 - top_alpha);
            for channel in 0..3 {
                let premultiplied =
                    f32::from(top[channel]) / 255.0 + SQUARE[channel] * background * (1.0 - top_alpha);
                // Значок хранится без предумножения.
                let value = if alpha > 0.0 { premultiplied / alpha } else { 0.0 };
                rgba.push((value * 255.0).round().clamp(0.0, 255.0) as u8);
            }
            rgba.push((alpha * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    IconData {
        rgba,
        width: SIZE as u32,
        height: SIZE as u32,
    }
}
