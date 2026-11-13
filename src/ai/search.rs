//! Search for the best move: iterative deepening, principal variation alpha-beta, a
//! transposition table, move ordering and late move reductions.
//!
//! Features of draughts that the search takes into account:
//! * captures are mandatory, so they do not consume depth, and the static evaluation
//!   is taken only in positions where the side to move has nothing to capture;
//! * the draw rules (repetition and counters) are checked in every node;
//! * the score fades smoothly to zero as a counter draw approaches, so that the
//!   stronger side changes the balance of forces in time.

use super::eval::evaluate;
use crate::board::Position;
use crate::moves::{generate_moves, legal_moves, Move};
use crate::rng::Rng;
use crate::rules::GameState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Score of a win in the current node; a win in `n` plies is scored as `WIN - n`.
pub const WIN: i32 = 30_000;
/// The bound above which a score means a forced win.
pub const WIN_BOUND: i32 = WIN - 1_000;
const INF: i32 = 32_000;
/// Greatest depth of a search line in plies.
const MAX_PLY: usize = 120;
/// This many plies before a counter draw the score starts to fade to zero.
const DRAW_FADE_PLIES: u32 = 16;
/// If fewer plies are left before a counter draw, their number becomes part of the table key.
const DRAW_KEY_PLIES: u32 = 24;

/// What a stored score says about the true value of a position.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bound {
    /// The table slot is empty.
    None,
    /// The score is exact.
    Exact,
    /// The true value is at least the stored score (the search failed high).
    Lower,
    /// The true value is at most the stored score (the search failed low).
    Upper,
}

const NO_MOVE: u8 = u8::MAX;
const NO_KILLER: u16 = u16::MAX;
/// Amount of work (nodes and generated moves) between checks of the time and the stop signal.
const POLL_WORK: usize = 2_048;

/// Parameters of one search.
#[derive(Clone, Debug)]
pub struct SearchParams {
    /// Greatest depth in quiet plies (captures do not consume depth).
    pub max_depth: i32,
    /// Hard time limit: the search is interrupted when it runs out.
    pub hard_time: Option<Duration>,
    /// Soft limit: no new iteration starts after it.
    pub soft_time: Option<Duration>,
    /// Whether captures are followed to the end at the depth limit.
    pub quiescence: bool,
    /// Amplitude of the pseudo-random addition to the score (in hundredths of a man).
    pub jitter: i32,
    /// Seed: the order of equal moves and the addition to the score.
    pub seed: u64,
    /// Size of the transposition table: 2^`table_bits` entries.
    pub table_bits: u32,
    /// Compute the exact score of every root move (needed by levels that choose at random).
    pub exact_root_scores: bool,
}

/// Result of a search.
#[derive(Clone, Debug)]
pub struct SearchResult {
    /// The best move; `None` if there are no moves or the search was interrupted before the
    /// first score.
    pub best: Option<Move>,
    /// Score of the best move from the point of view of the side to move.
    pub score: i32,
    /// Depth of the last fully completed iteration.
    pub depth: i32,
    /// Number of nodes visited.
    pub nodes: u64,
    /// Scores of the root moves from the last completed iteration.
    /// They are exact for all moves only with `exact_root_scores`.
    pub root_scores: Vec<(Move, i32)>,
    /// The search was stopped by the clock or by an external signal.
    pub aborted: bool,
}

#[derive(Clone, Copy)]
struct Entry {
    key: u64,
    score: i16,
    depth: i8,
    bound: Bound,
    mv: u8,
}

const EMPTY_ENTRY: Entry = Entry {
    key: 0,
    score: 0,
    depth: 0,
    bound: Bound::None,
    mv: NO_MOVE,
};

/// Transposition table: buckets of two entries (one prefers depth, the other is always replaced).
struct Table {
    entries: Vec<Entry>,
    mask: usize,
}

impl Table {
    fn new(bits: u32) -> Self {
        let size = 1usize << bits.clamp(4, 26);
        Self {
            entries: vec![EMPTY_ENTRY; size],
            mask: size - 2,
        }
    }

    #[inline]
    fn probe(&self, key: u64) -> Option<Entry> {
        let index = (key as usize) & self.mask;
        let first = self.entries[index];
        if first.bound != Bound::None && first.key == key {
            return Some(first);
        }
        let second = self.entries[index + 1];
        if second.bound != Bound::None && second.key == key {
            return Some(second);
        }
        None
    }

    #[inline]
    fn store(&mut self, key: u64, depth: i32, score: i32, bound: Bound, mv: u8) {
        let index = (key as usize) & self.mask;
        let entry = Entry {
            key,
            score: score.clamp(-INF, INF) as i16,
            depth: depth.clamp(0, 127) as i8,
            bound,
            mv,
        };
        let first = self.entries[index];
        if first.bound == Bound::None || first.key == key || entry.depth >= first.depth {
            self.entries[index] = entry;
        } else {
            self.entries[index + 1] = entry;
        }
    }
}

#[inline]
const fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x
}

#[inline]
const fn to_table(score: i32, ply: usize) -> i32 {
    if score >= WIN_BOUND {
        score + ply as i32
    } else if score <= -WIN_BOUND {
        score - ply as i32
    } else {
        score
    }
}

#[inline]
const fn from_table(score: i32, ply: usize) -> i32 {
    if score >= WIN_BOUND {
        score - ply as i32
    } else if score <= -WIN_BOUND {
        score + ply as i32
    } else {
        score
    }
}

/// Removes captures that differ only in their route: if the first and last squares and
/// the set of captured pieces are the same, the position after the move is the same,
/// and such moves are equal for the search.
fn dedupe_captures(moves: &mut Vec<Move>) {
    if moves.len() < 2 || !moves[0].is_capture() {
        return;
    }
    let mut kept = 1;
    for i in 1..moves.len() {
        let mv = moves[i];
        let duplicate = moves[..kept].iter().any(|k| {
            k.to() == mv.to()
                && k.captured() == mv.captured()
                && k.from() == mv.from()
                && k.promotes() == mv.promotes()
        });
        if !duplicate {
            moves[kept] = mv;
            kept += 1;
        }
    }
    moves.truncate(kept);
}

/// Moves the best of the not yet searched moves (`order[from..]`) to position `from`.
/// Among equal scores the earliest one wins, so the order stays deterministic.
#[inline]
fn select_best(order: &mut [(i32, u16)], from: usize) {
    let mut pick = from;
    for candidate in from + 1..order.len() {
        if order[candidate].0 > order[pick].0 {
            pick = candidate;
        }
    }
    order.swap(from, pick);
}

#[inline]
const fn killer_key(mv: &Move) -> u16 {
    (mv.from().index() as u16) << 5 | mv.to().index() as u16
}

/// Transposition table key: the position and, close to a counter draw, the number of
/// plies left before it.
fn table_key(st: &GameState) -> u64 {
    let left = st.plies_to_counter_draw();
    let salt = if left < DRAW_KEY_PLIES {
        mix(0xD6E8_FEB8_6659_FD93 ^ u64::from(left))
    } else {
        0
    };
    st.pos.hash64() ^ salt
}

struct Searcher<'a> {
    table: Table,
    stop: &'a AtomicBool,
    started: Instant,
    hard_time: Option<Duration>,
    nodes: u64,
    /// Amount of work since the last time check (nodes and generated moves).
    work: usize,
    aborted: bool,
    /// Positions of the game branch up to the root and of the current search line.
    path: Vec<Position>,
    /// Index of the root position in `path`.
    root_index: usize,
    killers: Vec<[u16; 2]>,
    history: Vec<i32>,
    move_buffers: Vec<Vec<Move>>,
    order_buffers: Vec<Vec<(i32, u16)>>,
    quiescence: bool,
    jitter: i32,
    seed: u64,
}

impl Searcher<'_> {
    fn poll(&mut self) {
        if self.stop.load(Ordering::Relaxed) {
            self.aborted = true;
        } else if let Some(limit) = self.hard_time {
            if self.started.elapsed() >= limit {
                self.aborted = true;
            }
        }
    }

    /// Static score from the point of view of the side to move.
    fn eval(&self, st: &GameState) -> i32 {
        let mut score = evaluate(&st.pos);
        if self.jitter > 0 {
            let span = 2 * self.jitter as u64 + 1;
            score += (mix(st.pos.hash64() ^ self.seed) % span) as i32 - self.jitter;
        }
        let left = st.plies_to_counter_draw();
        if left < DRAW_FADE_PLIES {
            score = score * left as i32 / DRAW_FADE_PLIES as i32;
        }
        score.clamp(-WIN_BOUND + 1, WIN_BOUND - 1)
    }

    /// Repetition of a position: inside the search line one match is enough; among the
    /// game positions before the root two are needed (then this is the third occurrence).
    fn repeated(&self, st: &GameState) -> bool {
        let reversible = st.counters.kings_only as usize;
        if reversible < 4 {
            return false;
        }
        let len = self.path.len();
        let mut earlier = 0;
        let mut back = 4;
        while back <= reversible && back <= len {
            let index = len - back;
            if self.path[index] == st.pos {
                if index >= self.root_index {
                    return true;
                }
                earlier += 1;
                if earlier >= 2 {
                    return true;
                }
            }
            back += 2;
        }
        false
    }

    /// Fills `order` with a sorting score for every move: the table move first, then
    /// big captures, killer moves and the history heuristic.
    fn score_moves(
        &self,
        st: &GameState,
        moves: &[Move],
        table_move: u8,
        ply: usize,
        order: &mut Vec<(i32, u16)>,
    ) {
        let in_capture = moves[0].is_capture();
        let side = st.pos.side as usize;
        let killers = self.killers[ply];
        order.clear();
        for (index, mv) in moves.iter().enumerate() {
            let score = if index == table_move as usize {
                1_000_000_000
            } else if in_capture {
                let kings = (mv.captured() & st.pos.kings).count_ones() as i32;
                1_000 * mv.captured_count() as i32 + 500 * kings + 100 * i32::from(mv.promotes())
            } else {
                let k = killer_key(mv);
                if k == killers[0] {
                    900_000_000
                } else if k == killers[1] {
                    800_000_000
                } else {
                    let h = self.history[side * 1024 + k as usize];
                    h + if mv.promotes() { 500_000_000 } else { 0 }
                }
            };
            order.push((score, index as u16));
        }
    }

    /// Remembers a quiet move that refuted the line: it becomes a killer move at this
    /// ply and gains history score.
    fn record_cutoff(&mut self, mv: &Move, side: usize, depth: i32, ply: usize) {
        let k = killer_key(mv);
        if self.killers[ply][0] != k {
            self.killers[ply][1] = self.killers[ply][0];
            self.killers[ply][0] = k;
        }
        let slot = &mut self.history[side * 1024 + k as usize];
        *slot = (*slot + depth * depth).min(400_000_000);
    }

    fn negamax(&mut self, st: &GameState, depth: i32, alpha: i32, beta: i32, ply: usize, pv: bool) -> i32 {
        self.nodes += 1;
        if self.aborted {
            return 0;
        }
        let mut moves = std::mem::take(&mut self.move_buffers[ply]);
        let mut order = std::mem::take(&mut self.order_buffers[ply]);
        generate_moves(&st.pos, &mut moves);
        // The time is checked by the amount of work, not by the number of nodes: in
        // positions with long king captures a single node can hold hundreds of moves.
        self.work += 1 + moves.len();
        if self.work >= POLL_WORK {
            self.work = 0;
            self.poll();
        }
        dedupe_captures(&mut moves);
        let score = if self.aborted {
            0
        } else {
            self.node(st, depth, alpha, beta, ply, pv, &moves, &mut order)
        };
        self.move_buffers[ply] = moves;
        self.order_buffers[ply] = order;
        score
    }

    #[allow(clippy::too_many_arguments)]
    fn node(
        &mut self,
        st: &GameState,
        depth: i32,
        mut alpha: i32,
        mut beta: i32,
        ply: usize,
        pv: bool,
        moves: &[Move],
        order: &mut Vec<(i32, u16)>,
    ) -> i32 {
        if moves.is_empty() {
            return -(WIN - ply as i32);
        }
        if st.counter_draw().is_some() || self.repeated(st) {
            return 0;
        }
        let in_capture = moves[0].is_capture();
        if ply + 1 >= MAX_PLY || (depth <= 0 && !(in_capture && self.quiescence)) {
            return self.eval(st);
        }
        let depth = depth.max(0);

        // Nobody can win or lose faster than in `ply` plies any more.
        alpha = alpha.max(-(WIN - ply as i32));
        beta = beta.min(WIN - ply as i32 - 1);
        if alpha >= beta {
            return alpha;
        }

        let key = table_key(st);
        let mut table_move = NO_MOVE;
        if let Some(entry) = self.table.probe(key) {
            table_move = entry.mv;
            if !pv && i32::from(entry.depth) >= depth {
                let score = from_table(i32::from(entry.score), ply);
                match entry.bound {
                    Bound::Exact => return score,
                    Bound::Lower if score >= beta => return score,
                    Bound::Upper if score <= alpha => return score,
                    _ => {}
                }
            }
        }

        self.score_moves(st, moves, table_move, ply, order);
        let side = st.pos.side as usize;
        let count = moves.len();
        // Short lists are ordered by selection as the moves are searched (the first move
        // is often enough); long ones are sorted at once.
        let presorted = count > 24;
        if presorted {
            order.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        }
        let step = i32::from(!(in_capture || count == 1));
        let original_alpha = alpha;
        let mut best = -INF;
        let mut best_index = NO_MOVE;
        self.path.push(st.pos);
        for i in 0..count {
            if !presorted {
                select_best(order, i);
            }
            let index = order[i].1 as usize;
            let mv = &moves[index];
            let child = st.play(mv);
            let next_depth = depth - step;

            let mut score;
            if i == 0 {
                score = -self.negamax(&child, next_depth, -beta, -alpha, ply + 1, pv);
            } else {
                let mut reduction = 0;
                if !in_capture && depth >= 3 && i >= 3 && !mv.promotes() {
                    reduction = if depth >= 6 && i >= 8 { 2 } else { 1 };
                }
                score = -self.negamax(&child, next_depth - reduction, -alpha - 1, -alpha, ply + 1, false);
                if score > alpha && reduction > 0 {
                    score = -self.negamax(&child, next_depth, -alpha - 1, -alpha, ply + 1, false);
                }
                if score > alpha && score < beta {
                    score = -self.negamax(&child, next_depth, -beta, -alpha, ply + 1, pv);
                }
            }
            if self.aborted {
                break;
            }
            if score > best {
                best = score;
                best_index = index.min(NO_MOVE as usize) as u8;
            }
            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                if !in_capture {
                    self.record_cutoff(mv, side, depth, ply);
                }
                break;
            }
        }
        self.path.pop();
        if self.aborted {
            return 0;
        }

        let bound = if best >= beta {
            Bound::Lower
        } else if best > original_alpha {
            Bound::Exact
        } else {
            Bound::Upper
        };
        self.table
            .store(key, depth, to_table(best, ply), bound, best_index);
        best
    }
}

/// Searches for the best move in the state `state`.
///
/// `history` holds the positions of the game branch before the current one (needed to
/// account for repetitions). `stop` allows the search to be interrupted from outside.
pub fn search(
    state: &GameState,
    history: &[Position],
    params: &SearchParams,
    stop: &AtomicBool,
) -> SearchResult {
    let mut rng = Rng::new(params.seed);
    let mut root_moves = legal_moves(&state.pos);
    dedupe_captures(&mut root_moves);
    let mut root: Vec<(Move, i32)> = root_moves.into_iter().map(|m| (m, 0)).collect();
    let mut result = SearchResult {
        best: None,
        score: 0,
        depth: 0,
        nodes: 0,
        root_scores: Vec::new(),
        aborted: false,
    };
    if root.is_empty() {
        return result;
    }
    // A random initial order: among equal moves a random one is chosen.
    rng.shuffle(&mut root);

    let mut path = Vec::with_capacity(history.len() + MAX_PLY + 1);
    path.extend_from_slice(history);
    let mut searcher = Searcher {
        table: Table::new(params.table_bits),
        stop,
        started: Instant::now(),
        hard_time: params.hard_time,
        nodes: 0,
        work: 0,
        aborted: false,
        root_index: path.len(),
        path,
        killers: vec![[NO_KILLER; 2]; MAX_PLY + 1],
        history: vec![0; 2 * 1024],
        move_buffers: (0..=MAX_PLY).map(|_| Vec::with_capacity(32)).collect(),
        order_buffers: (0..=MAX_PLY).map(|_| Vec::with_capacity(32)).collect(),
        quiescence: params.quiescence,
        jitter: params.jitter.max(0),
        seed: rng.next_u64(),
    };

    let single = root.len() == 1;
    let max_depth = params.max_depth.clamp(1, MAX_PLY as i32 - 2);
    for depth in 1..=max_depth {
        let mut alpha = -INF;
        let mut best: Option<(usize, i32)> = None;
        searcher.path.push(state.pos);
        // An index loop: the scores are written into `root` as the moves are searched.
        #[allow(clippy::needless_range_loop)]
        for i in 0..root.len() {
            let mv = root[i].0;
            let child = state.play(&mv);
            let next_depth = if mv.is_capture() || single {
                depth
            } else {
                depth - 1
            };
            let score = if i == 0 || params.exact_root_scores {
                -searcher.negamax(&child, next_depth, -INF, -alpha, 1, true)
            } else {
                let mut s = -searcher.negamax(&child, next_depth, -alpha - 1, -alpha, 1, false);
                if s > alpha && !searcher.aborted {
                    s = -searcher.negamax(&child, next_depth, -INF, -alpha, 1, true);
                }
                s
            };
            if searcher.aborted {
                break;
            }
            root[i].1 = score;
            if best.is_none_or(|(_, b)| score > b) {
                best = Some((i, score));
            }
            if !params.exact_root_scores && score > alpha {
                alpha = score;
            }
        }
        searcher.path.pop();

        // An interrupted iteration counts only if at least its first move (the best of the
        // previous iteration) has been scored: an improvement found in it can be trusted.
        if let Some((index, score)) = best {
            result.best = Some(root[index].0);
            result.score = score;
            root[..=index].rotate_right(1);
        }
        if searcher.aborted {
            result.aborted = true;
            break;
        }
        result.depth = depth;
        result.root_scores.clone_from(&root);

        // A forced outcome is found, and the depth is enough to leave no doubt about it.
        if result.score.abs() >= WIN_BOUND && depth >= WIN - result.score.abs() + 2 {
            break;
        }
        if let Some(soft) = params.soft_time {
            if searcher.started.elapsed() >= soft {
                break;
            }
        }
    }
    result.nodes = searcher.nodes;
    if result.root_scores.is_empty() {
        result.root_scores = root;
    }
    result
}
