//! Race-specific search: match pieces to destination cells, clear the home in time,
//! and search useful jump routes. Difficulty uses the application's shared 1–10 scale.

use super::{home_for, State, HOME_DEADLINE};
use crate::games::{squares_of, Color, Move, Position, Square};
use crate::rng::Rng;
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

fn distance(a: Square, b: Square) -> i32 {
    i32::from(a.file().abs_diff(b.file()) + a.rank().abs_diff(b.rank()))
}

/// Rectangular Hungarian assignment: every piece gets a different destination.
/// This prevents the computer from advancing everyone towards the same corner cell.
fn assignment(pos: Position, color: Color) -> i32 {
    let pieces: Vec<_> = squares_of(pos.pieces(color)).collect();
    let goals: Vec<_> = squares_of(home_for(color.opponent(), pos.home_side)).collect();
    let mut u = [0i32; 13];
    let mut v = [0i32; 13];
    let mut matched = [0usize; 13];
    let mut previous = [0usize; 13];
    for row in 1..=pieces.len() {
        matched[0] = row;
        let mut column = 0;
        let mut minima = [i32::MAX; 13];
        let mut used = [false; 13];
        loop {
            used[column] = true;
            let current = matched[column];
            let mut delta = i32::MAX;
            let mut next = 0;
            for j in 1..=goals.len() {
                if used[j] {
                    continue;
                }
                let cost = distance(pieces[current - 1], goals[j - 1]) - u[current] - v[j];
                if cost < minima[j] {
                    minima[j] = cost;
                    previous[j] = column;
                }
                if minima[j] < delta {
                    delta = minima[j];
                    next = j;
                }
            }
            for j in 0..=goals.len() {
                if used[j] {
                    u[matched[j]] += delta;
                    v[j] -= delta;
                } else {
                    minima[j] = minima[j].saturating_sub(delta);
                }
            }
            column = next;
            if matched[column] == 0 {
                break;
            }
        }
        loop {
            let next = previous[column];
            matched[column] = matched[next];
            column = next;
            if column == 0 {
                break;
            }
        }
    }
    -v[0]
}

#[must_use]
pub fn evaluate(state: State, color: Color) -> i32 {
    let score = |side| {
        let arrived = state.arrived(side) as i32;
        let at_home = (state.pos.pieces(side) & home_for(side, state.pos.home_side)).count_ones() as i32;
        let urgency = 20 + i32::from(state.ply.min(HOME_DEADLINE * 2)) * 3;
        arrived * 110 - assignment(state.pos, side) * 24 - at_home * urgency
    };
    score(color) - score(color.opponent())
}

struct Search<'a> {
    stop: &'a AtomicBool,
    deadline: Instant,
    width: usize,
    path: Vec<Position>,
    nodes: u64,
}
impl Search<'_> {
    fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed) || Instant::now() >= self.deadline
    }
    fn ordered(&self, state: State) -> Vec<(Move, i32)> {
        let side = state.pos.side;
        let mut ranked = Vec::new();
        for mv in state.legal_moves() {
            if self.stopped() {
                break;
            }
            let after = state.after(&mv);
            let repeated = self.path.iter().filter(|&&pos| pos == after.pos).count() as i32;
            let score = evaluate(after, side) - repeated * 180;
            ranked.push((mv, score));
        }
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        ranked.truncate(self.width);
        ranked
    }
    fn node(&mut self, state: State, depth: u8, mut alpha: i32, beta: i32) -> Option<i32> {
        if self.stopped() {
            return None;
        }
        self.nodes += 1;
        let occurrences = 1 + self.path.iter().filter(|&&pos| pos == state.pos).count() as u32;
        if let Some(outcome) = state.outcome(occurrences) {
            return Some(outcome.winner().map_or(0, |winner| {
                if winner == state.pos.side {
                    100_000 + i32::from(depth)
                } else {
                    -100_000 - i32::from(depth)
                }
            }));
        }
        if depth == 0 {
            return Some(evaluate(state, state.pos.side));
        }
        let moves = self.ordered(state);
        if self.stopped() {
            return None;
        }
        self.path.push(state.pos);
        let mut best = -200_000;
        for (mv, _) in moves {
            let answer = self.node(state.after(&mv), depth - 1, -beta, -alpha);
            let Some(score) = answer.map(|score| -score) else {
                self.path.pop();
                return None;
            };
            best = best.max(score);
            alpha = alpha.max(score);
            if alpha >= beta {
                break;
            }
        }
        self.path.pop();
        Some(if best == -200_000 { 0 } else { best })
    }
}

#[must_use]
pub fn choose_move(
    state: State,
    history: &[Position],
    level: u8,
    seed: u64,
    stop: &AtomicBool,
) -> Option<Move> {
    let legal = state.legal_moves();
    let mut rng = Rng::new(seed);
    let settings = level_settings(level);
    if legal.is_empty() || stop.load(Ordering::Relaxed) {
        return None;
    }
    if legal.len() == 1 || rng.chance(settings.blunder) {
        return rng.pick(&legal).copied();
    }
    let mut search = Search {
        stop,
        deadline: Instant::now() + Duration::from_millis(settings.budget_ms),
        width: settings.width,
        path: history.to_vec(),
        nodes: 0,
    };
    let mut best = legal[0];
    for depth in 1..=settings.depth {
        let roots = search.ordered(state);
        let mut iteration = None;
        let mut best_score = -200_001;
        let mut complete = true;
        search.path.push(state.pos);
        for (mv, _) in roots {
            let Some(score) = search
                .node(state.after(&mv), depth - 1, -200_000, 200_000)
                .map(|s| -s)
            else {
                complete = false;
                break;
            };
            let score = score + rng.symmetric(settings.noise);
            if score > best_score {
                best_score = score;
                iteration = Some(mv);
            }
        }
        search.path.pop();
        if complete {
            if let Some(mv) = iteration {
                best = mv;
            }
        } else {
            break;
        }
        if search.stopped() {
            break;
        }
    }
    Some(best)
}
