//! The computer opponent: ten levels of strength.
//!
//! The weak levels search shallowly, judge moves with a random error and now and then
//! play a random move. The strong levels search deeper, and the tenth uses almost all
//! of the time allowed for a move.

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

/// The weakest level.
pub const MIN_LEVEL: u8 = 1;
/// The strongest level.
pub const MAX_LEVEL: u8 = 10;
/// Time limit for a computer move on any level, ms.
pub const MOVE_TIME_LIMIT_MS: u64 = 5_000;

/// Settings of a level.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Level {
    /// Search depth in quiet plies.
    pub depth: i32,
    /// Hard limit of the search time, ms.
    pub hard_time_ms: u64,
    /// Soft limit: no new iteration starts after it, ms.
    pub soft_time_ms: u64,
    /// Probability of playing a random move instead of the one found.
    pub blunder: f64,
    /// Amplitude of the random error in the scores of root moves (in hundredths of a man).
    pub noise: i32,
    /// Amplitude of a small addition to the static score: it varies the play of strong levels.
    pub jitter: i32,
    /// Probability of a "careless" move: a one-ply calculation that does not check the
    /// opponent's captures in reply, so the level leaves pieces en prise.
    pub careless: f64,
    /// Size of the transposition table (binary logarithm of the number of entries).
    pub table_bits: u32,
}

/// Table of levels: from a beginner to full strength.
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

/// Brings a level number into the allowed range.
#[must_use]
pub fn clamp_level(level: u8) -> u8 {
    level.clamp(MIN_LEVEL, MAX_LEVEL)
}

/// Settings of the level `level` (the number is brought into the range 1–10).
#[must_use]
pub fn level_settings(level: u8) -> Level {
    LEVELS[(clamp_level(level) - MIN_LEVEL) as usize]
}

/// The move chosen by the computer.
#[derive(Clone, Debug)]
pub struct Choice {
    pub mv: Move,
    /// Score of the move from the computer's point of view; `None` if it was chosen without
    /// a search.
    pub score: Option<i32>,
    /// Depth of the completed search.
    pub depth: i32,
    /// Number of nodes visited.
    pub nodes: u64,
}

/// Chooses a move on the given level.
///
/// `history` holds the positions of the game branch before the current one (to account
/// for repetitions), `seed` determines every random decision, `stop` allows the
/// calculation to be interrupted. Returns `None` only if there are no legal moves.
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
        // The error is a sum of two uniform values: small mistakes are more frequent than big ones.
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
            assert!(s.hard_time_ms < MOVE_TIME_LIMIT_MS, "level {level}");
            assert!(s.soft_time_ms <= s.hard_time_ms);
        }
        assert!(
            LEVELS[0].careless >= 1.0 && LEVELS[0].blunder > 0.2,
            "the first level makes noticeable mistakes"
        );
        assert_eq!(level_settings(0), level_settings(1));
        assert_eq!(level_settings(200), level_settings(10));
    }
}
