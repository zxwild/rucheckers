//! Значок окна: создаётся программно, без файлов ресурсов.

use eframe::egui::IconData;

const SIZE: u32 = 128;
/// Сглаживание: число отсчётов по каждой оси на пиксель.
const SAMPLES: u32 = 3;

/// Цвет точки значка (без предумножения альфа-канала).
fn sample(x: f32, y: f32) -> [f32; 4] {
    // Координаты в диапазоне −1…1 относительно центра.
    let (u, v) = (x * 2.0 - 1.0, y * 2.0 - 1.0);
    // Скруглённый квадрат тёмного поля.
    let corner = 0.22;
    let (qx, qy) = (
        (u.abs() - (0.94 - corner)).max(0.0),
        (v.abs() - (0.94 - corner)).max(0.0),
    );
    if (qx * qx + qy * qy).sqrt() > corner {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let square = [0.63, 0.44, 0.25, 1.0];
    // Белая шашка: боковая грань, верх и две проточки.
    let side = (u * u + (v - 0.07) * (v - 0.07)).sqrt();
    let top = (u * u + (v + 0.03) * (v + 0.03)).sqrt();
    if top <= 0.66 {
        if (top - 0.50).abs() < 0.022 || (top - 0.30).abs() < 0.02 {
            return [0.78, 0.71, 0.57, 1.0];
        }
        let shade = 1.0 - 0.07 * ((u + v) * 0.7 + 0.5).clamp(0.0, 1.0);
        return [0.96 * shade, 0.93 * shade, 0.86 * shade, 1.0];
    }
    if side <= 0.66 {
        return [0.72, 0.65, 0.50, 1.0];
    }
    square
}

/// Значок приложения: белая шашка на тёмном поле.
pub fn app_icon() -> IconData {
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    let step = 1.0 / (SIZE * SAMPLES) as f32;
    for py in 0..SIZE {
        for px in 0..SIZE {
            let mut sum = [0.0f32; 4];
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = ((px * SAMPLES + sx) as f32 + 0.5) * step;
                    let y = ((py * SAMPLES + sy) as f32 + 0.5) * step;
                    let c = sample(x, y);
                    sum[0] += c[0] * c[3];
                    sum[1] += c[1] * c[3];
                    sum[2] += c[2] * c[3];
                    sum[3] += c[3];
                }
            }
            let alpha = sum[3] / (SAMPLES * SAMPLES) as f32;
            for channel in &sum[..3] {
                let value = if sum[3] > 0.0 { channel / sum[3] } else { 0.0 };
                rgba.push((value * 255.0).round().clamp(0.0, 255.0) as u8);
            }
            rgba.push((alpha * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    IconData {
        rgba,
        width: SIZE,
        height: SIZE,
    }
}
