//! Синтез звуков: все сигналы создаются программно, без файлов ресурсов.
//!
//! Модуль только вычисляет отсчёты (моно, [`SAMPLE_RATE`] Гц, диапазон −1…1).
//! Воспроизведение находится в исполняемом файле.

use crate::rng::Rng;
use std::f32::consts::TAU;

/// Частота дискретизации синтезируемых звуков, Гц.
pub const SAMPLE_RATE: u32 = 44_100;

/// Звуковые сигналы игры.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Sound {
    /// Тихий ход.
    Move,
    /// Взятие.
    Capture,
    /// Превращение в дамку.
    Promotion,
    /// Партия выиграна человеком.
    Win,
    /// Партия проиграна человеком.
    Loss,
    /// Ничья.
    Draw,
}

impl Sound {
    /// Все сигналы.
    pub const ALL: [Sound; 6] = [
        Sound::Move,
        Sound::Capture,
        Sound::Promotion,
        Sound::Win,
        Sound::Loss,
        Sound::Draw,
    ];
}

fn samples(seconds: f32) -> usize {
    (seconds * SAMPLE_RATE as f32) as usize
}

/// Добавляет в буфер короткий «деревянный» стук, начиная с момента `at` секунд.
fn knock(buffer: &mut [f32], at: f32, pitch: f32, gain: f32, rng: &mut Rng) {
    let start = samples(at);
    let length = samples(0.09);
    // Собственные частоты удара по дереву и скорости их затухания.
    let partials: [(f32, f32, f32); 4] = [
        (190.0, 0.55, 38.0),
        (760.0, 0.50, 85.0),
        (1_480.0, 0.32, 140.0),
        (2_650.0, 0.16, 220.0),
    ];
    let mut noise_state = 0.0f32;
    for i in 0..length {
        let Some(slot) = buffer.get_mut(start + i) else {
            break;
        };
        let t = i as f32 / SAMPLE_RATE as f32;
        let mut value = 0.0;
        for (frequency, amplitude, decay) in partials {
            value += amplitude * (TAU * frequency * pitch * t).sin() * (-decay * t).exp();
        }
        // Шумовой щелчок в первые миллисекунды: момент касания.
        let white = rng.unit() as f32 * 2.0 - 1.0;
        noise_state += 0.35 * (white - noise_state);
        value += 0.5 * noise_state * (-420.0 * t).exp();
        let attack = (t / 0.0015).min(1.0);
        *slot += gain * attack * value;
    }
}

/// Добавляет в буфер мягкую ноту колокольчика.
fn note(buffer: &mut [f32], at: f32, frequency: f32, seconds: f32, gain: f32) {
    let start = samples(at);
    let length = samples(seconds);
    for i in 0..length {
        let Some(slot) = buffer.get_mut(start + i) else {
            break;
        };
        let t = i as f32 / SAMPLE_RATE as f32;
        let envelope = (t / 0.008).min(1.0) * (-5.0 * t / seconds).exp();
        let tone = (TAU * frequency * t).sin()
            + 0.30 * (TAU * 2.0 * frequency * t).sin()
            + 0.10 * (TAU * 3.0 * frequency * t).sin();
        *slot += gain * envelope * tone;
    }
}

/// Сглаживает края и ограничивает пик, чтобы звук не щёлкал и не перегружал выход.
fn finish(mut buffer: Vec<f32>, peak: f32) -> Vec<f32> {
    let fade = samples(0.004).min(buffer.len() / 2);
    let len = buffer.len();
    for i in 0..fade {
        let k = i as f32 / fade as f32;
        buffer[i] *= k;
        buffer[len - 1 - i] *= k;
    }
    let max = buffer.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if max > 1e-6 {
        let scale = peak / max;
        for value in &mut buffer {
            *value *= scale;
        }
    }
    buffer
}

/// Отсчёты сигнала (моно, [`SAMPLE_RATE`] Гц). Результат детерминирован.
pub fn synthesize(sound: Sound) -> Vec<f32> {
    let mut rng = Rng::new(0x5EED_0000 + sound as u64);
    match sound {
        Sound::Move => {
            let mut buffer = vec![0.0; samples(0.11)];
            knock(&mut buffer, 0.0, 1.0, 1.0, &mut rng);
            finish(buffer, 0.55)
        }
        Sound::Capture => {
            // Удар по доске и следом — более высокий стук снятой шашки.
            let mut buffer = vec![0.0; samples(0.19)];
            knock(&mut buffer, 0.0, 0.92, 1.0, &mut rng);
            knock(&mut buffer, 0.075, 1.35, 0.6, &mut rng);
            finish(buffer, 0.62)
        }
        Sound::Promotion => {
            let mut buffer = vec![0.0; samples(0.52)];
            knock(&mut buffer, 0.0, 1.0, 0.5, &mut rng);
            note(&mut buffer, 0.02, 659.25, 0.30, 0.6);
            note(&mut buffer, 0.14, 987.77, 0.38, 0.6);
            finish(buffer, 0.50)
        }
        Sound::Win => {
            let mut buffer = vec![0.0; samples(0.95)];
            note(&mut buffer, 0.00, 523.25, 0.40, 0.6);
            note(&mut buffer, 0.16, 659.25, 0.40, 0.6);
            note(&mut buffer, 0.32, 783.99, 0.62, 0.7);
            finish(buffer, 0.50)
        }
        Sound::Loss => {
            let mut buffer = vec![0.0; samples(0.95)];
            note(&mut buffer, 0.00, 392.00, 0.40, 0.6);
            note(&mut buffer, 0.18, 311.13, 0.40, 0.6);
            note(&mut buffer, 0.36, 261.63, 0.58, 0.7);
            finish(buffer, 0.45)
        }
        Sound::Draw => {
            let mut buffer = vec![0.0; samples(0.75)];
            note(&mut buffer, 0.00, 587.33, 0.36, 0.6);
            note(&mut buffer, 0.20, 440.00, 0.54, 0.6);
            finish(buffer, 0.45)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_are_short_bounded_and_click_free() {
        for sound in Sound::ALL {
            let data = synthesize(sound);
            let seconds = data.len() as f32 / SAMPLE_RATE as f32;
            assert!(seconds > 0.05 && seconds < 1.2, "{sound:?}: {seconds} с");
            assert!(data.iter().all(|v| v.is_finite()), "{sound:?}");
            let peak = data.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.2 && peak <= 0.7, "{sound:?}: пик {peak}");
            assert!(
                data[0].abs() < 0.01 && data[data.len() - 1].abs() < 0.01,
                "{sound:?}: края"
            );
            let energy: f32 = data.iter().map(|v| v * v).sum::<f32>() / data.len() as f32;
            assert!(energy > 1e-4, "{sound:?}: звук слышен");
        }
    }

    #[test]
    fn sounds_are_deterministic_and_distinct() {
        for sound in Sound::ALL {
            assert_eq!(synthesize(sound), synthesize(sound), "{sound:?}");
        }
        for (i, a) in Sound::ALL.iter().enumerate() {
            for b in &Sound::ALL[i + 1..] {
                assert_ne!(synthesize(*a), synthesize(*b), "{a:?} и {b:?}");
            }
        }
        assert!(synthesize(Sound::Move).len() < synthesize(Sound::Capture).len());
    }
}
