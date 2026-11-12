//! Компьютерный соперник: десять уровней силы.
//!
//! Слабые уровни считают неглубоко, оценивают ходы со случайной погрешностью и время
//! от времени делают случайный ход. Сильные уровни считают глубже, а десятый использует
//! почти весь отведённый на ход лимит времени.

pub mod eval;
pub mod search;
pub mod worker;

use crate::board::Position;
use crate::moves::{legal_moves, Move};
use crate::rng::Rng;
use crate::rules::GameState;
use search::{search, SearchParams};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// Самый слабый уровень.
pub const MIN_LEVEL: u8 = 1;
/// Самый сильный уровень.
pub const MAX_LEVEL: u8 = 10;
/// Лимит времени на ход компьютера на любом уровне, мс.
pub const MOVE_TIME_LIMIT_MS: u64 = 5_000;

/// Настройки уровня.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Level {
    /// Глубина поиска в тихих полуходах.
    pub depth: i32,
    /// Жёсткий лимит времени поиска, мс.
    pub hard_time_ms: u64,
    /// Мягкий лимит: после него новая итерация не начинается, мс.
    pub soft_time_ms: u64,
    /// Вероятность сделать случайный ход вместо найденного.
    pub blunder: f64,
    /// Амплитуда случайной погрешности оценки ходов в корне (в сотых простой).
    pub noise: i32,
    /// Амплитуда малой добавки к статической оценке: разнообразит игру сильных уровней.
    pub jitter: i32,
    /// Вероятность «невнимательного» хода: расчёт на один полуход без проверки
    /// ответных взятий, из-за чего уровень подставляет шашки.
    pub careless: f64,
    /// Размер таблицы перестановок (двоичный логарифм числа записей).
    pub table_bits: u32,
}

/// Таблица уровней: от новичка до максимальной силы.
#[rustfmt::skip]
const LEVELS: [Level; 10] = [
    Level { depth: 1, hard_time_ms: 1_000, soft_time_ms: 1_000, blunder: 0.25, careless: 1.00, noise: 80, jitter: 0, table_bits: 12 },
    Level { depth: 1, hard_time_ms: 1_000, soft_time_ms: 1_000, blunder: 0.12, careless: 0.65, noise: 50, jitter: 0, table_bits: 12 },
    Level { depth: 1, hard_time_ms: 1_000, soft_time_ms: 1_000, blunder: 0.06, careless: 0.35, noise: 35, jitter: 0, table_bits: 14 },
    Level { depth: 2, hard_time_ms: 1_000, soft_time_ms: 1_000, blunder: 0.03, careless: 0.18, noise: 25, jitter: 0, table_bits: 15 },
    Level { depth: 3, hard_time_ms: 1_500, soft_time_ms: 1_500, blunder: 0.01, careless: 0.08, noise: 16, jitter: 0, table_bits: 16 },
    Level { depth: 4, hard_time_ms: 2_000, soft_time_ms: 2_000, blunder: 0.0, careless: 0.03, noise: 10, jitter: 0, table_bits: 17 },
    Level { depth: 6, hard_time_ms: 2_500, soft_time_ms: 2_500, blunder: 0.0, careless: 0.0, noise: 6, jitter: 0, table_bits: 18 },
    Level { depth: 8, hard_time_ms: 3_000, soft_time_ms: 1_500, blunder: 0.0, careless: 0.0, noise: 0, jitter: 3, table_bits: 20 },
    Level { depth: 11, hard_time_ms: 4_000, soft_time_ms: 2_000, blunder: 0.0, careless: 0.0, noise: 0, jitter: 2, table_bits: 21 },
    Level { depth: 99, hard_time_ms: 4_500, soft_time_ms: 2_700, blunder: 0.0, careless: 0.0, noise: 0, jitter: 1, table_bits: 22 },
];

/// Приводит номер уровня к допустимому диапазону.
#[must_use]
pub fn clamp_level(level: u8) -> u8 {
    level.clamp(MIN_LEVEL, MAX_LEVEL)
}

/// Настройки уровня `level` (номер приводится к диапазону 1–10).
#[must_use]
pub fn level_settings(level: u8) -> Level {
    LEVELS[(clamp_level(level) - MIN_LEVEL) as usize]
}

/// Выбранный компьютером ход.
#[derive(Clone, Debug)]
pub struct Choice {
    pub mv: Move,
    /// Оценка хода с точки зрения компьютера; `None`, если ход выбран без поиска.
    pub score: Option<i32>,
    /// Глубина завершённого поиска.
    pub depth: i32,
    /// Число просмотренных узлов.
    pub nodes: u64,
}

/// Выбирает ход на заданном уровне.
///
/// `history` — позиции ветки партии перед текущей (для учёта повторений),
/// `seed` определяет все случайные решения, `stop` позволяет прервать расчёт.
/// Возвращает `None`, только если допустимых ходов нет.
pub fn choose_move(
    state: &GameState,
    history: &[Position],
    level: u8,
    seed: u64,
    stop: &AtomicBool,
) -> Option<Choice> {
    let settings = level_settings(level);
    let legal = legal_moves(&state.pos);
    let mut rng = Rng::new(seed);
    let unsearched = |mv: Move| Choice {
        mv,
        score: None,
        depth: 0,
        nodes: 0,
    };
    match legal.len() {
        0 => return None,
        1 => return Some(unsearched(legal[0])),
        _ => {}
    }
    if settings.blunder > 0.0 && rng.chance(settings.blunder) {
        return rng.pick(&legal).copied().map(unsearched);
    }

    let careless = settings.careless > 0.0 && rng.chance(settings.careless);
    let params = SearchParams {
        max_depth: if careless { 1 } else { settings.depth },
        hard_time: Some(Duration::from_millis(settings.hard_time_ms)),
        soft_time: Some(Duration::from_millis(settings.soft_time_ms)),
        quiescence: !careless,
        jitter: settings.jitter,
        seed: rng.next_u64(),
        table_bits: settings.table_bits,
        exact_root_scores: settings.noise > 0,
    };
    let result = search(state, history, &params, stop);

    let mut choice = Choice {
        mv: result.best.unwrap_or(legal[0]),
        score: result.best.map(|_| result.score),
        depth: result.depth,
        nodes: result.nodes,
    };
    if settings.noise > 0 && !result.root_scores.is_empty() {
        // Погрешность — сумма двух равномерных величин: малые ошибки чаще больших.
        let mut best_noisy = i32::MIN;
        for (mv, score) in &result.root_scores {
            let noisy = score + rng.symmetric(settings.noise) + rng.symmetric(settings.noise);
            if noisy > best_noisy {
                best_noisy = noisy;
                choice.mv = *mv;
                choice.score = Some(*score);
            }
        }
    }
    Some(choice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_grow_monotonically() {
        for pair in LEVELS.windows(2) {
            let (weak, strong) = (pair[0], pair[1]);
            assert!(strong.depth >= weak.depth);
            assert!(strong.blunder <= weak.blunder);
            assert!(strong.noise <= weak.noise);
            assert!(strong.careless <= weak.careless);
        }
        for level in MIN_LEVEL..=MAX_LEVEL {
            let s = level_settings(level);
            assert!(s.hard_time_ms < MOVE_TIME_LIMIT_MS, "уровень {level}");
            assert!(s.soft_time_ms <= s.hard_time_ms);
        }
        assert!(
            LEVELS[0].careless >= 1.0 && LEVELS[0].blunder > 0.2,
            "первый уровень заметно ошибается"
        );
        assert_eq!(level_settings(0), level_settings(1));
        assert_eq!(level_settings(200), level_settings(10));
    }
}
