//! The computer's play in Ugolki: an alpha-beta search of the race.
//!
//! The search works on bitboards and on the two end cells of a move, which is all that
//! decides the next position. It deepens iteratively until the time of the level runs
//! out, searches every first move, and orders the rest by the stored best move, by
//! moves that refuted others and by the progress a move makes. Moves late in that order
//! are searched less deeply first and fully only when they look better. Finished games
//! and repetitions are judged by the rules of `super`, repetitions along the branch.

use super::bits::{side_index, Board, Finish, Rules};
use super::eval::Evaluator;
use super::State;
use crate::games::{Color, Move, Position};
use crate::rng::Rng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Search settings of one level.
#[derive(Clone, Copy, Debug)]
pub struct Level {
    /// Deepest iteration, in plies.
    pub depth: u8,
    /// Time for a move, in milliseconds.
    pub budget_ms: u64,
    /// Largest random change of the score of each first move, in the units of the
    /// evaluation, where a cell of the remaining work is worth 16.
    pub noise: i32,
    /// Probability of a random legal move instead of a search.
    pub blunder: f64,
}

/// Settings of a level of the shared scale; levels outside 1–10 are brought into it.
#[must_use]
pub fn level_settings(level: u8) -> Level {
    const DEPTH: [u8; 10] = [1, 1, 2, 2, 3, 4, 6, 8, 11, 64];
    const TIME: [u64; 10] = [60, 100, 180, 250, 400, 650, 900, 1300, 1800, 2800];
    const NOISE: [i32; 10] = [200, 120, 80, 48, 28, 16, 8, 3, 1, 0];
    const BLUNDER: [f64; 10] = [0.35, 0.20, 0.12, 0.06, 0.02, 0.0, 0.0, 0.0, 0.0, 0.0];
    let i = usize::from(crate::games::difficulty::clamp_level(level) - 1);
    Level {
        depth: DEPTH[i],
        budget_ms: TIME[i],
        noise: NOISE[i],
        blunder: BLUNDER[i],
    }
}

pub use super::eval::evaluate;

const INFINITY: i32 = 200_000;
const WIN: i32 = 100_000;
/// Scores beyond this are results of the game, not estimates.
const DECIDED: i32 = WIN - u16::MAX as i32;
const MAX_HEIGHT: usize = 96;
/// Buckets of the filter of repeated positions.
const BUCKETS: usize = 4096;
/// Moves searched to the full depth before the later ones are first searched less deeply.
const FULL_DEPTH_MOVES: usize = 3;
/// Moves after this many further are reduced by two plies, from a depth of five plies.
const DEEP_REDUCTION_MOVES: usize = 12;
/// At the last two plies, moves that make no progress are skipped after this many
/// moves per remaining ply.
const LATE_MOVES_PER_PLY: usize = 6;

type Step = (u8, u8);

const fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Random keys of a piece of each side on each cell.
const KEYS: [[u64; 64]; 2] = {
    let mut keys = [[0; 64]; 2];
    let mut cell = 0;
    while cell < 64 {
        keys[0][cell] = mix(cell as u64 * 2 + 1);
        keys[1][cell] = mix(cell as u64 * 2 + 2);
        cell += 1;
    }
    keys
};
const BLACK_TO_MOVE: u64 = mix(1000);

fn key_of(board: &Board) -> u64 {
    let mut key = if board.side == Color::Black {
        BLACK_TO_MOVE
    } else {
        0
    };
    for (side, mut pieces) in [(0, board.white), (1, board.black)] {
        while pieces != 0 {
            key ^= KEYS[side][pieces.trailing_zeros() as usize];
            pieces &= pieces - 1;
        }
    }
    key
}

/// The deadlines make the same arrangement differ with the move number.
const fn table_key(key: u64, ply: u16) -> u64 {
    key ^ mix(ply as u64 + 5000)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Copy)]
struct Entry {
    key: u64,
    score: i32,
    depth: i8,
    bound: Bound,
    step: Step,
}

/// An earlier position of the game or of the line being searched.
#[derive(Clone, Copy)]
struct Seen {
    key: u64,
    white: u64,
    black: u64,
}

fn result_score(finish: Finish, board: &Board) -> i32 {
    match finish {
        Finish::Drawn => 0,
        Finish::Won(winner) => {
            let score = WIN - i32::from(board.ply);
            if winner == board.side {
                score
            } else {
                -score
            }
        }
    }
}

fn with_noise(score: i32, noise: i32) -> i32 {
    // Difficulty noise must not turn a proven result into a different one.
    if score.abs() >= DECIDED {
        score
    } else {
        score + noise
    }
}

struct Search<'a> {
    rules: Rules,
    eval: Evaluator,
    stop: &'a AtomicBool,
    deadline: Instant,
    nodes: u64,
    aborted: bool,
    table: Vec<Option<Entry>>,
    path: Vec<Seen>,
    /// How many positions of `path` fall into each bucket of their keys: an empty bucket
    /// proves a position new without looking through the path.
    buckets: Vec<u16>,
    killers: [[Step; 2]; MAX_HEIGHT],
    history: Vec<i32>,
    buffers: Vec<Vec<Step>>,
    /// Moves of each height with their rank in the order and their progress.
    ordered: Vec<Vec<(i32, Step, i32)>>,
    /// Late moves are pruned and reduced; switched off to compare with minimax.
    selective: bool,
}

impl<'a> Search<'a> {
    fn new(
        layout: super::HomeSide,
        history: &[Position],
        stop: &'a AtomicBool,
        deadline: Instant,
        table_bits: u32,
    ) -> Self {
        let mut search = Self {
            rules: Rules::new(layout),
            eval: Evaluator::new(layout),
            stop,
            deadline,
            nodes: 0,
            aborted: false,
            table: vec![None; 1 << table_bits],
            path: Vec::new(),
            buckets: vec![0; BUCKETS],
            killers: [[(0, 0); 2]; MAX_HEIGHT],
            history: vec![0; 2 * 64 * 64],
            buffers: vec![Vec::new(); MAX_HEIGHT],
            ordered: vec![Vec::new(); MAX_HEIGHT],
            selective: true,
        };
        for pos in history {
            let board = Board {
                white: pos.white,
                black: pos.black,
                side: pos.side,
                ply: 0,
            };
            search.enter(Seen {
                key: key_of(&board),
                white: pos.white,
                black: pos.black,
            });
        }
        search
    }

    fn out_of_time(&self) -> bool {
        self.stop.load(Ordering::Relaxed) || Instant::now() >= self.deadline
    }

    fn enter(&mut self, seen: Seen) {
        self.buckets[seen.key as usize % BUCKETS] += 1;
        self.path.push(seen);
    }

    fn leave(&mut self) {
        if let Some(seen) = self.path.pop() {
            self.buckets[seen.key as usize % BUCKETS] -= 1;
        }
    }

    fn occurrences(&self, board: &Board, key: u64) -> u32 {
        if self.buckets[key as usize % BUCKETS] == 0 {
            return 1;
        }
        1 + self
            .path
            .iter()
            .filter(|seen| seen.key == key && seen.white == board.white && seen.black == board.black)
            .count() as u32
    }

    fn slot(&self, key: u64) -> usize {
        (key as usize) & (self.table.len() - 1)
    }

    fn store(&mut self, key: u64, depth: i32, score: i32, bound: Bound, step: Step) {
        let slot = self.slot(key);
        if let Some(old) = self.table[slot] {
            if old.key == key && i32::from(old.depth) > depth && bound != Bound::Exact {
                return;
            }
        }
        self.table[slot] = Some(Entry {
            key,
            score,
            depth: depth.clamp(0, 127) as i8,
            bound,
            step,
        });
    }

    fn history_index(side: usize, step: Step) -> usize {
        side * 4096 + usize::from(step.0) * 64 + usize::from(step.1)
    }

    /// Orders the moves of `board` into `self.ordered[height]`.
    fn order(&mut self, board: &Board, height: usize, first: Option<Step>) {
        let mut steps = std::mem::take(&mut self.buffers[height]);
        let mut ranked = std::mem::take(&mut self.ordered[height]);
        self.rules.moves(board, &mut steps);
        ranked.clear();
        let side = side_index(board.side);
        let killers = self.killers[height];
        let profile = self.eval.profile(board.side, board.own());
        for &step in &steps {
            let progress = self.eval.progress(board.side, &profile, step.0, step.1);
            let mut rank =
                progress * 256 + self.history[Self::history_index(side, step)].min(255 * 256) / 256;
            if Some(step) == first {
                rank = i32::MAX;
            } else if step == killers[0] {
                rank += 1 << 26;
            } else if step == killers[1] {
                rank += 1 << 25;
            }
            ranked.push((rank, step, progress));
        }
        ranked.sort_unstable_by_key(|&(rank, _, _)| std::cmp::Reverse(rank));
        self.buffers[height] = steps;
        self.ordered[height] = ranked;
    }

    /// The stored result of a position: a score that settles the node within the window,
    /// and the best move found before.
    fn probe(
        &self,
        key: u64,
        depth: i32,
        alpha: i32,
        beta: i32,
        occurrences: u32,
    ) -> (Option<i32>, Option<Step>) {
        let Some(entry) = self.table[self.slot(key)].filter(|entry| entry.key == key) else {
            return (None, None);
        };
        // A position met before on this line depends on that line; its stored value may
        // hide a repetition, so it only suggests the first move.
        if i32::from(entry.depth) < depth || occurrences > 1 {
            return (None, Some(entry.step));
        }
        let settled = match entry.bound {
            Bound::Exact => true,
            Bound::Lower => entry.score >= beta,
            Bound::Upper => entry.score <= alpha,
        };
        (settled.then_some(entry.score), Some(entry.step))
    }

    /// How much less deeply a move is searched first, or `None` to skip it: moves late
    /// in the order are rarely better than the ones before them.
    fn reduction(&self, index: usize, rank: i32, progress: i32, depth: i32, best: i32) -> Option<i32> {
        if index == 0 || rank >= 1 << 24 || !self.selective {
            return Some(0);
        }
        if depth <= 2 && index >= LATE_MOVES_PER_PLY * depth as usize && best > -DECIDED && progress <= 0 {
            return None;
        }
        Some(if depth >= 3 && index >= FULL_DEPTH_MOVES {
            1 + i32::from(index >= DEEP_REDUCTION_MOVES && depth >= 5)
        } else {
            0
        })
    }

    /// Remembers a move that refuted the opponent's previous move.
    fn refuted(&mut self, height: usize, side: usize, step: Step, depth: i32) {
        if self.killers[height][0] != step {
            self.killers[height][1] = self.killers[height][0];
            self.killers[height][0] = step;
        }
        let entry = &mut self.history[Self::history_index(side, step)];
        *entry = (*entry + depth * depth * 16).min(1 << 20);
    }

    fn negamax(
        &mut self,
        board: Board,
        key: u64,
        depth: i32,
        mut alpha: i32,
        beta: i32,
        height: usize,
    ) -> i32 {
        self.nodes += 1;
        if self.nodes.trailing_zeros() >= 10 && self.out_of_time() {
            self.aborted = true;
        }
        if self.aborted {
            return 0;
        }
        let occurrences = self.occurrences(&board, key);
        if let Some(finish) = self.rules.adjudicate(&board, occurrences) {
            return result_score(finish, &board);
        }
        let stored_key = table_key(key, board.ply);
        let (stored, first) = self.probe(stored_key, depth, alpha, beta, occurrences);
        if let Some(score) = stored {
            return score;
        }
        if depth <= 0 || height + 1 >= MAX_HEIGHT {
            if !self.rules.has_move(&board) {
                return result_score(self.rules.without_moves(&board), &board);
            }
            return self.eval.evaluate(&board);
        }
        self.order(&board, height, first);
        let ranked = std::mem::take(&mut self.ordered[height]);
        if ranked.is_empty() {
            self.ordered[height] = ranked;
            return result_score(self.rules.without_moves(&board), &board);
        }
        let side = side_index(board.side);
        self.enter(Seen {
            key,
            white: board.white,
            black: board.black,
        });
        let original_alpha = alpha;
        let mut best = -INFINITY;
        let mut best_step = ranked[0].1;
        for (index, &(rank, step, progress)) in ranked.iter().enumerate() {
            let child = board.after(step.0, step.1);
            let child_key =
                key ^ KEYS[side][usize::from(step.0)] ^ KEYS[side][usize::from(step.1)] ^ BLACK_TO_MOVE;
            let Some(reduction) = self.reduction(index, rank, progress, depth, best) else {
                continue;
            };
            let mut score;
            if index == 0 {
                score = -self.negamax(child, child_key, depth - 1, -beta, -alpha, height + 1);
            } else {
                score = -self.negamax(
                    child,
                    child_key,
                    depth - 1 - reduction,
                    -alpha - 1,
                    -alpha,
                    height + 1,
                );
                if score > alpha && reduction > 0 && !self.aborted {
                    score = -self.negamax(child, child_key, depth - 1, -alpha - 1, -alpha, height + 1);
                }
                if score > alpha && score < beta && !self.aborted {
                    score = -self.negamax(child, child_key, depth - 1, -beta, -alpha, height + 1);
                }
            }
            if self.aborted {
                break;
            }
            if score > best {
                best = score;
                best_step = step;
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        self.refuted(height, side, step, depth);
                        break;
                    }
                }
            }
        }
        self.leave();
        self.ordered[height] = ranked;
        if self.aborted {
            return 0;
        }
        if occurrences == 1 {
            let bound = if best <= original_alpha {
                Bound::Upper
            } else if best >= beta {
                Bound::Lower
            } else {
                Bound::Exact
            };
            self.store(stored_key, depth, best, bound, best_step);
        }
        best
    }
}

struct RootMove {
    step: Step,
    score: i32,
    noise: i32,
}

impl Search<'_> {
    /// Searches every first move to `depth`, the previous leader first. Returns the move
    /// that leads, unless the time runs out before the first move is settled.
    fn iterate(
        &mut self,
        board: &Board,
        key: u64,
        roots: &mut [RootMove],
        depth: i32,
    ) -> Option<(Step, i32)> {
        let side = side_index(board.side);
        let mut leader: Option<(Step, i32)> = None;
        for root in roots.iter_mut() {
            let child = board.after(root.step.0, root.step.1);
            let child_key = key
                ^ KEYS[side][usize::from(root.step.0)]
                ^ KEYS[side][usize::from(root.step.1)]
                ^ BLACK_TO_MOVE;
            let raw = match leader {
                None => -self.negamax(child, child_key, depth - 1, -INFINITY, INFINITY, 1),
                Some((_, leading)) => {
                    // Only a score above this bound can overtake the leader.
                    let bound = if leading.abs() >= DECIDED {
                        leading
                    } else {
                        leading - root.noise
                    };
                    let probe = -self.negamax(child, child_key, depth - 1, -bound - 1, -bound, 1);
                    if probe > bound && !self.aborted {
                        -self.negamax(child, child_key, depth - 1, -INFINITY, -bound, 1)
                    } else {
                        probe
                    }
                }
            };
            if self.aborted {
                break;
            }
            root.score = with_noise(raw, root.noise);
            if leader.is_none_or(|(_, leading)| root.score > leading) {
                leader = Some((root.step, root.score));
            }
        }
        leader
    }

    /// Deepens the search of all first moves until the time or the depth runs out.
    fn deepen(&mut self, board: &Board, key: u64, roots: &mut [RootMove], max_depth: i32) -> Option<Step> {
        let mut choice = roots.first().map(|root| root.step);
        for depth in 1..=max_depth {
            let previous = roots.first().map(|root| root.step);
            let leader = self.iterate(board, key, roots, depth);
            if self.aborted {
                // A move that overtook the previous choice during this iteration was
                // searched completely and is kept.
                if let Some((step, _)) = leader.filter(|&(step, _)| Some(step) != previous) {
                    choice = Some(step);
                }
                break;
            }
            let Some((step, score)) = leader else {
                break;
            };
            choice = Some(step);
            roots.sort_by_key(|root| std::cmp::Reverse(root.score));
            // A result of the game closer than the depth will not change.
            if score.abs() >= DECIDED && WIN - score.abs() - i32::from(board.ply) + 2 <= depth {
                break;
            }
            if self.out_of_time() {
                break;
            }
        }
        choice
    }
}

/// Searches with the given settings; `choose_move` takes them from the level.
#[must_use]
pub fn choose_move_with(
    state: State,
    history: &[Position],
    settings: Level,
    seed: u64,
    stop: &AtomicBool,
) -> Option<Move> {
    let deadline = Instant::now() + Duration::from_millis(settings.budget_ms);
    if stop.load(Ordering::Relaxed) {
        return None;
    }
    let legal = state.legal_moves();
    let mut rng = Rng::new(seed);
    if legal.len() <= 1 || rng.chance(settings.blunder) {
        return rng.pick(&legal).copied();
    }
    let board = Board {
        white: state.pos.white,
        black: state.pos.black,
        side: state.pos.side,
        ply: state.ply,
    };
    let table_bits = if settings.budget_ms >= 1000 { 20 } else { 18 };
    let mut search = Search::new(state.pos.home_side, history, stop, deadline, table_bits);
    let key = key_of(&board);
    search.order(&board, 0, None);
    let mut roots: Vec<RootMove> = search.ordered[0]
        .iter()
        .map(|&(_, step, _)| RootMove {
            step,
            score: -INFINITY,
            noise: rng.symmetric(settings.noise),
        })
        .collect();
    search.enter(Seen {
        key,
        white: board.white,
        black: board.black,
    });
    let step = search.deepen(&board, key, &mut roots, i32::from(settings.depth.max(1)));
    if stop.load(Ordering::Relaxed) {
        return None;
    }
    let step = step?;
    legal
        .iter()
        .find(|mv| mv.from().index() == usize::from(step.0) && mv.to().index() == usize::from(step.1))
        .or(legal.first())
        .copied()
}

/// Searches all first moves of `state` within the time of the level.
#[must_use]
pub fn choose_move(
    state: State,
    history: &[Position],
    level: u8,
    seed: u64,
    stop: &AtomicBool,
) -> Option<Move> {
    choose_move_with(state, history, level_settings(level), seed, stop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::{GameKind, Square};

    fn state(fen: &str, ply: u16) -> State {
        State {
            pos: Position::from_fen_for(GameKind::Ugolki, fen).unwrap(),
            ply,
        }
    }

    fn board(state: State) -> Board {
        Board {
            white: state.pos.white,
            black: state.pos.black,
            side: state.pos.side,
            ply: state.ply,
        }
    }

    fn minimax(state: State, depth: u8, path: &mut Vec<Position>) -> i32 {
        let count = 1 + path.iter().filter(|&&pos| pos == state.pos).count() as u32;
        if let Some(outcome) = state.outcome(count) {
            let finish = outcome.winner().map_or(Finish::Drawn, Finish::Won);
            return result_score(finish, &board(state));
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
    fn higher_levels_search_deeper_and_more_carefully() {
        for level in 1..10 {
            let (low, high) = (level_settings(level), level_settings(level + 1));
            assert!(high.depth >= low.depth && high.budget_ms > low.budget_ms);
            assert!(high.noise <= low.noise && high.blunder <= low.blunder);
        }
        let top = level_settings(10);
        assert!(top.noise == 0 && top.blunder == 0.0 && top.depth >= 64);
    }

    #[test]
    fn a_full_width_search_matches_minimax_with_any_history() {
        let stop = AtomicBool::new(false);
        for state in [
            state("W:Wc3,e3:Bf6,d6", 24),
            state("W:Wa3,h7:Be4", 78),
            state("B:Wh8,e4:Ba1,d4", 159),
            state("W:Wd4,d5,e4:Be5,c4,d3", 40),
        ] {
            let repeated = state.after(&state.legal_moves()[0]).pos;
            for history in [vec![], vec![repeated, repeated], vec![state.pos, state.pos]] {
                for depth in 1..=3 {
                    let deadline = Instant::now() + Duration::from_secs(30);
                    let mut search = Search::new(state.pos.home_side, &history, &stop, deadline, 12);
                    search.selective = false;
                    let expected = minimax(state, depth, &mut history.clone());
                    let key = key_of(&board(state));
                    let depth = i32::from(depth);
                    assert_eq!(
                        search.negamax(board(state), key, depth, -INFINITY, INFINITY, 0),
                        expected,
                        "{} at depth {depth}",
                        state.pos.to_fen()
                    );
                    // The stored results must respect any later window.
                    for (alpha, beta) in [(expected - 10, expected - 1), (expected + 1, expected + 10)] {
                        let result = search.negamax(board(state), key, depth, alpha, beta, 0);
                        assert!(if expected >= beta {
                            result >= beta
                        } else {
                            result <= alpha
                        });
                    }
                }
            }
        }
    }

    #[test]
    fn the_search_finds_the_shortest_win_of_a_race() {
        let stop = AtomicBool::new(false);
        // White is two steps from finishing and Black cannot finish at all.
        let bit = |name| Square::parse(name).unwrap().bit();
        let mut pos = Position::empty(Color::White, GameKind::Ugolki);
        pos.white = (super::super::home(Color::Black) & !bit("e6") & !bit("e7")) | bit("d6") | bit("d7");
        pos.black = bit("a4");
        let start = State { pos, ply: 30 };
        let mut current = start;
        let mut history = Vec::new();
        for _ in 0..2 {
            let mv = choose_move(current, &history, 10, 3, &stop).unwrap();
            history.push(current.pos);
            current = current.after(&mv);
            if current.pos.side == Color::Black {
                let reply = choose_move(current, &history, 10, 4, &stop).unwrap();
                history.push(current.pos);
                current = current.after(&reply);
            }
        }
        assert_eq!(current.arrived(Color::White), 12);
    }

    #[test]
    fn the_last_piece_leaves_the_home_before_it_can_be_shut_in() {
        let stop = AtomicBool::new(false);
        // Two White moves are left before the deadline. If d3 waits, Black closes d4
        // with c4-d4 and the way round through c3 takes two moves.
        let state = state(
            "W:Wd3,e8,f8,g8,h8,e7,f7,g7,h7,f6,g6,h6:Bc4,d5,e3,f3,a1,b1,c1,a2,b2,c2,a3,b3",
            76,
        );
        for level in [6, 10] {
            let mv = choose_move(state, &[], level, 5, &stop).unwrap();
            let home = super::super::home(Color::White);
            assert_eq!(mv.from(), Square::parse("d3").unwrap(), "level {level}");
            assert_eq!(home & mv.to().bit(), 0, "level {level}");
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
