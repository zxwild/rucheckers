//! Race-specific search: match pieces to destination cells, clear the home in time,
//! and search useful jump routes. Difficulty uses the application's shared 1–10 scale.

use super::State;
use crate::games::{Color, Move, Outcome, Position, Square};
use crate::rng::Rng;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Level {
    pub depth: u8,
    pub width: usize,
    pub budget_ms: u64,
    pub noise: i32,
    pub blunder: f64,
}

#[must_use]
pub fn level_settings(level: u8) -> Level {
    const DEPTH: [u8; 10] = [1, 1, 2, 2, 3, 3, 4, 4, 5, 6];
    const WIDTH: [usize; 10] = [6, 8, 8, 10, 10, 12, 12, 14, 16, 18];
    const TIME: [u64; 10] = [60, 100, 180, 250, 400, 650, 900, 1300, 1800, 2800];
    const NOISE: [i32; 10] = [500, 300, 200, 120, 70, 40, 20, 8, 3, 0];
    const BLUNDER: [f64; 10] = [0.35, 0.20, 0.12, 0.06, 0.02, 0.0, 0.0, 0.0, 0.0, 0.0];
    let i = usize::from(crate::games::difficulty::clamp_level(level) - 1);
    Level {
        depth: DEPTH[i],
        width: WIDTH[i],
        budget_ms: TIME[i],
        noise: NOISE[i],
        blunder: BLUNDER[i],
    }
}

pub use super::eval::evaluate;

const INFINITY: i32 = 200_000;
const WIN: i32 = 100_000;
const CACHE_LIMIT: usize = 65_536;

type Key = (Position, u16);
type Endpoints = (Square, Square);

#[derive(Clone, Copy)]
struct Cached {
    evaluation: i32,
    preferred: Option<Endpoints>,
}

struct Search<'a> {
    stop: &'a AtomicBool,
    deadline: Instant,
    width: usize,
    path: Vec<Position>,
    // Only static evaluations and move ordering are cached. Search scores depend on
    // the repetition history, so sharing them across different paths would be unsafe.
    cache: HashMap<Key, Cached>,
}

fn result_score(state: State, outcome: Outcome, side: Color) -> i32 {
    outcome.winner().map_or(0, |winner| {
        let score = WIN - i32::from(state.ply);
        if winner == side {
            score
        } else {
            -score
        }
    })
}

fn with_noise(score: i32, noise: i32) -> i32 {
    // Difficulty noise must not turn a proven immediate win into a slower win.
    // Terminal scores stay outside the heuristic range even at the largest ply.
    if score.abs() >= WIN - i32::from(u16::MAX) {
        score
    } else {
        score + noise
    }
}

impl Search<'_> {
    fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed) || Instant::now() >= self.deadline
    }

    fn occurrences(&self, pos: Position) -> u32 {
        1 + self.path.iter().filter(|&&old| old == pos).count() as u32
    }

    fn evaluation(&mut self, state: State) -> i32 {
        let key = (state.pos, state.ply);
        if let Some(cached) = self.cache.get(&key) {
            return cached.evaluation;
        }
        let evaluation = evaluate(state, state.pos.side);
        if self.cache.len() < CACHE_LIMIT {
            self.cache.insert(
                key,
                Cached {
                    evaluation,
                    preferred: None,
                },
            );
        }
        evaluation
    }

    fn ordered(&mut self, state: State, legal: Vec<Move>) -> Vec<(Move, i32)> {
        let preferred = self
            .cache
            .get(&(state.pos, state.ply))
            .and_then(|entry| entry.preferred);
        let mut ranked = Vec::with_capacity(legal.len());
        for mv in legal {
            if self.stopped() {
                break;
            }
            let after = state.after(&mv);
            let occurrences = self.occurrences(after.pos);
            let score = if let Some(outcome) = after.adjudicate(occurrences) {
                result_score(after, outcome, state.pos.side)
            } else if after.arrived(state.pos.side) == super::PIECES as u32 {
                // White's finishing move still allows Black a reply. Always search
                // that move; its true win/draw value is established by the reply.
                WIN / 2
            } else {
                -self.evaluation(after) - (occurrences as i32 - 1) * 180
            };
            ranked.push((mv, score));
        }
        ranked.sort_by_key(|&(mv, score)| {
            (
                std::cmp::Reverse(Some((mv.from(), mv.to())) == preferred),
                std::cmp::Reverse(score),
            )
        });
        ranked
    }

    fn node(&mut self, state: State, depth: u8, mut alpha: i32, beta: i32) -> Option<i32> {
        if self.stopped() {
            return None;
        }
        if let Some(outcome) = state.adjudicate(self.occurrences(state.pos)) {
            return Some(result_score(state, outcome, state.pos.side));
        }
        let legal = state.legal_moves();
        if legal.is_empty() {
            return Some(result_score(state, state.no_moves_outcome(), state.pos.side));
        }
        if depth == 0 {
            return Some(self.evaluation(state));
        }
        self.evaluation(state);
        let mut moves = self.ordered(state, legal);
        moves.truncate(self.width);
        if self.stopped() {
            return None;
        }
        self.path.push(state.pos);
        let mut best = -INFINITY;
        let mut preferred = None;
        for (mv, _) in moves {
            let answer = self.node(state.after(&mv), depth - 1, -beta, -alpha);
            let Some(score) = answer.map(|score| -score) else {
                self.path.pop();
                return None;
            };
            if score > best {
                best = score;
                preferred = Some((mv.from(), mv.to()));
            }
            alpha = alpha.max(score);
            if alpha >= beta {
                break;
            }
        }
        self.path.pop();
        if let Some(entry) = self.cache.get_mut(&(state.pos, state.ply)) {
            entry.preferred = preferred;
        }
        Some(best)
    }
}

struct RootMove {
    mv: Move,
    score: i32,
    noise: i32,
}

/// Searches all root moves, with narrower continuations and a shared alpha bound.
/// The last fully completed iteration is kept if the unchanged time budget expires.
#[must_use]
pub fn choose_move(
    state: State,
    history: &[Position],
    level: u8,
    seed: u64,
    stop: &AtomicBool,
) -> Option<Move> {
    let settings = level_settings(level);
    let deadline = Instant::now() + Duration::from_millis(settings.budget_ms);
    if stop.load(Ordering::Relaxed) {
        return None;
    }
    let legal = state.legal_moves();
    let fallback = *legal.first()?;
    let mut rng = Rng::new(seed);
    if legal.len() == 1 || rng.chance(settings.blunder) {
        return rng.pick(&legal).copied();
    }
    let mut search = Search {
        stop,
        deadline,
        width: settings.width,
        path: history.to_vec(),
        cache: HashMap::new(),
    };
    let mut roots: Vec<_> = search
        .ordered(state, legal)
        .into_iter()
        .map(|(mv, score)| RootMove {
            mv,
            score,
            noise: rng.symmetric(settings.noise),
        })
        .collect();
    let mut best = roots.first().map_or(fallback, |root| root.mv);
    search.path.push(state.pos);
    for depth in 1..=settings.depth {
        let mut iteration = None;
        let mut best_score = -INFINITY;
        let mut complete = true;
        for root in &mut roots {
            let Some(score) = search
                .node(
                    state.after(&root.mv),
                    depth - 1,
                    -INFINITY,
                    with_noise(-best_score, root.noise),
                )
                .map(|score| with_noise(-score, root.noise))
            else {
                complete = false;
                break;
            };
            root.score = score;
            if score > best_score {
                best_score = score;
                iteration = Some(root.mv);
            }
        }
        if complete {
            if let Some(mv) = iteration {
                best = mv;
            }
        } else {
            break;
        }
        // Principal variation first in the next iteration, including at the root.
        roots.sort_by_key(|root| std::cmp::Reverse(root.score));
        if search.stopped() {
            break;
        }
    }
    if stop.load(Ordering::Relaxed) {
        None
    } else {
        Some(best)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::GameKind;

    fn state(fen: &str, ply: u16) -> State {
        State {
            pos: Position::from_fen_for(GameKind::Ugolki, fen).unwrap(),
            ply,
        }
    }

    fn minimax(state: State, depth: u8, path: &mut Vec<Position>) -> i32 {
        let count = 1 + path.iter().filter(|&&pos| pos == state.pos).count() as u32;
        if let Some(outcome) = state.outcome(count) {
            return result_score(state, outcome, state.pos.side);
        }
        if depth == 0 {
            return evaluate(state, state.pos.side);
        }
        path.push(state.pos);
        let score = state
            .legal_moves()
            .iter()
            .map(|mv| -minimax(state.after(mv), depth - 1, path))
            .max()
            .unwrap();
        path.pop();
        score
    }

    #[test]
    fn all_level_time_budgets_and_the_reply_pause_are_unchanged() {
        let expected = [60, 100, 180, 250, 400, 650, 900, 1300, 1800, 2800];
        for (index, millis) in expected.into_iter().enumerate() {
            assert_eq!(level_settings(index as u8 + 1).budget_ms, millis);
        }
        assert_eq!(crate::games::session::DEFAULT_MIN_THINK_SECONDS, 0.6);
    }

    #[test]
    fn alpha_beta_matches_full_minimax_and_cache_does_not_hide_repetition() {
        let stop = AtomicBool::new(false);
        for state in [
            state("W:Wc3,e3:Bf6,d6", 24),
            state("W:Wa3,h7:Be4", 78),
            state("B:Wh8,e4:Ba1,d4", 159),
        ] {
            let mut search = Search {
                stop: &stop,
                deadline: Instant::now() + Duration::from_secs(10),
                width: usize::MAX,
                path: Vec::new(),
                cache: HashMap::new(),
            };
            for depth in 1..=3 {
                let expected = minimax(state, depth, &mut Vec::new());
                assert_eq!(search.node(state, depth, -INFINITY, INFINITY), Some(expected));
                for (alpha, beta) in [(expected - 10, expected - 1), (expected + 1, expected + 10)] {
                    let result = search.node(state, depth, alpha, beta).unwrap();
                    assert!(if expected >= beta {
                        result >= beta
                    } else {
                        result <= alpha
                    });
                }
            }
            // Reuse the same ordering/evaluation cache with a different history.
            let repeated = state.after(&state.legal_moves()[0]).pos;
            search.path = vec![repeated, repeated];
            let expected = minimax(state, 3, &mut search.path.clone());
            assert_eq!(search.node(state, 3, -INFINITY, INFINITY), Some(expected));
            search.path = vec![state.pos, state.pos];
            assert_eq!(search.node(state, 3, -INFINITY, INFINITY), Some(0));
        }
    }

    #[test]
    fn search_can_be_cancelled_during_a_strong_move() {
        use std::sync::Arc;
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let task = std::thread::spawn(move || {
            choose_move(State::new(super::super::initial()), &[], 10, 739, &worker_stop)
        });
        std::thread::sleep(Duration::from_millis(2));
        let cancelled = Instant::now();
        stop.store(true, Ordering::Relaxed);
        assert!(task.join().unwrap().is_none());
        assert!(cancelled.elapsed() < Duration::from_secs(1));
    }
}
