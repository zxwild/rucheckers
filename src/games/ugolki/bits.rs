//! Bitboard geometry and a fast form of the rules for the computer's search.
//!
//! Cells are numbered as `Square::index`: a1 = 0, h1 = 7, a8 = 56, h8 = 63. The search
//! only needs the two end cells of a move: jumps remove nothing, so every route to the
//! same cell leads to the same position. The functions here follow the rules of
//! `super` exactly; the tests compare them with `State::legal_moves` and
//! `State::outcome` on many positions.

use super::{home_for, HomeSide, HOME_DEADLINE, MOVE_LIMIT, PIECES};
use crate::games::Color;

pub const FILE_A: u64 = 0x0101_0101_0101_0101;
const FILE_B: u64 = FILE_A << 1;
const FILE_G: u64 = FILE_A << 6;
const FILE_H: u64 = FILE_A << 7;
pub const RANK_1: u64 = 0xFF;
const RANK_2: u64 = RANK_1 << 8;
const RANK_7: u64 = RANK_1 << 48;
const RANK_8: u64 = RANK_1 << 56;

/// The cells next to the cells of `set` along a file or a rank.
#[must_use]
pub const fn adjacent(set: u64) -> u64 {
    (set << 8) | (set >> 8) | ((set & !FILE_H) << 1) | ((set & !FILE_A) >> 1)
}

/// The cells reachable from `start` by chains of jumps over the cells of `occupied`.
/// The start cells themselves are not included. For a single piece `occupied` must
/// not contain its own cell: the cell is empty while the piece is on its way.
#[must_use]
pub const fn jump_closure(start: u64, occupied: u64) -> u64 {
    let mut reached = start;
    loop {
        let landing = ((((reached << 8) & occupied) << 8)
            | (((reached >> 8) & occupied) >> 8)
            | ((((reached & !FILE_H) << 1) & occupied & !FILE_H) << 1)
            | ((((reached & !FILE_A) >> 1) & occupied & !FILE_A) >> 1))
            & !occupied;
        let extended = reached | landing;
        if extended == reached {
            return reached & !start;
        }
        reached = extended;
    }
}

/// The cells from which a piece could neither step nor jump if exactly the cells of
/// `occupied` were taken.
#[must_use]
pub const fn immobile(occupied: u64) -> u64 {
    // A direction is closed when there is no cell next to the piece, or the next cell
    // is occupied and the cell behind it is occupied or outside the board.
    let up = RANK_8 | ((occupied >> 8) & (RANK_7 | RANK_8 | (occupied >> 16)));
    let down = RANK_1 | ((occupied << 8) & (RANK_1 | RANK_2 | (occupied << 16)));
    let right =
        FILE_H | ((occupied >> 1) & !FILE_H & (FILE_G | FILE_H | ((occupied >> 2) & !(FILE_G | FILE_H))));
    let left =
        FILE_A | ((occupied << 1) & !FILE_A & (FILE_A | FILE_B | ((occupied << 2) & !(FILE_A | FILE_B))));
    up & down & right & left
}

/// Pieces of `enemies` walled in by the pieces of `mover` and the edges of the board:
/// they could not move even if every other enemy piece left. This is the condition
/// of the rule against locking an opponent's piece.
#[must_use]
pub const fn locked(mover: u64, enemies: u64) -> u64 {
    enemies & immobile(mover)
}

/// For every cell, the cells one or two steps away along its file and its rank: the
/// only pieces whose freedom a piece arriving there can change.
const NEAR: [u64; 64] = {
    let mut table = [0; 64];
    let mut index = 0;
    while index < 64 {
        let bit = 1u64 << index;
        table[index] = adjacent(bit)
            | (bit << 16)
            | (bit >> 16)
            | ((bit & !(FILE_G | FILE_H)) << 2)
            | ((bit & !(FILE_A | FILE_B)) >> 2);
        index += 1;
    }
    table
};

/// A position as the search sees it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Board {
    pub white: u64,
    pub black: u64,
    pub side: Color,
    pub ply: u16,
}

impl Board {
    #[must_use]
    pub const fn occupied(&self) -> u64 {
        self.white | self.black
    }

    #[must_use]
    pub const fn own(&self) -> u64 {
        match self.side {
            Color::White => self.white,
            Color::Black => self.black,
        }
    }

    #[must_use]
    pub const fn other(&self) -> u64 {
        match self.side {
            Color::White => self.black,
            Color::Black => self.white,
        }
    }

    /// The position after the side to move takes a piece from `from` to `to`.
    #[must_use]
    pub const fn after(&self, from: u8, to: u8) -> Self {
        let change = (1u64 << from) | (1u64 << to);
        let (white, black) = match self.side {
            Color::White => (self.white ^ change, self.black),
            Color::Black => (self.white, self.black ^ change),
        };
        Self {
            white,
            black,
            side: self.side.opponent(),
            ply: self.ply.saturating_add(1),
        }
    }
}

/// How a finished game ended, as far as the search needs to know.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Finish {
    Won(Color),
    Drawn,
}

/// The homes of one layout and the rules that depend on them.
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    /// The starting home of White and of Black, indexed by `side_index`.
    pub home: [u64; 2],
}

#[must_use]
pub const fn side_index(color: Color) -> usize {
    match color {
        Color::White => 0,
        Color::Black => 1,
    }
}

impl Rules {
    #[must_use]
    pub fn new(layout: HomeSide) -> Self {
        Self {
            home: [home_for(Color::White, layout), home_for(Color::Black, layout)],
        }
    }

    /// The cells `color` has to fill: the opponent's home.
    #[must_use]
    pub const fn goal(&self, color: Color) -> u64 {
        self.home[1 - side_index(color)]
    }

    #[must_use]
    pub const fn arrived(&self, board: &Board, color: Color) -> u32 {
        let pieces = match color {
            Color::White => board.white,
            Color::Black => board.black,
        };
        (pieces & self.goal(color)).count_ones()
    }

    /// The result decided before any move is generated: filled homes, the deadlines
    /// and repetitions, in the precedence of `State::adjudicate`.
    #[must_use]
    pub const fn adjudicate(&self, board: &Board, occurrences: u32) -> Option<Finish> {
        let white = self.arrived(board, Color::White);
        let black = self.arrived(board, Color::Black);
        let white_done = white == PIECES as u32;
        let black_done = black == PIECES as u32;
        if white_done && black_done {
            return Some(Finish::Drawn);
        }
        if black_done {
            return Some(Finish::Won(Color::Black));
        }
        let white_moves = matches!(board.side, Color::White);
        if white_done && white_moves {
            return Some(Finish::Won(Color::White));
        }
        if white_moves && board.ply >= HOME_DEADLINE * 2 {
            let left_white = board.white & self.home[0] != 0;
            let left_black = board.black & self.home[1] != 0;
            match (left_white, left_black) {
                (true, true) => return Some(Finish::Drawn),
                (true, false) => return Some(Finish::Won(Color::Black)),
                (false, true) => return Some(Finish::Won(Color::White)),
                (false, false) => {}
            }
        }
        if white_moves && board.ply >= MOVE_LIMIT * 2 {
            return Some(if white > black {
                Finish::Won(Color::White)
            } else if black > white {
                Finish::Won(Color::Black)
            } else {
                Finish::Drawn
            });
        }
        if occurrences >= 3 {
            return Some(Finish::Drawn);
        }
        None
    }

    /// The result when the side to move has no legal move.
    #[must_use]
    pub const fn without_moves(&self, board: &Board) -> Finish {
        if self.arrived(board, Color::White) == PIECES as u32 {
            Finish::Won(Color::White)
        } else {
            Finish::Drawn
        }
    }

    /// Cells a piece of the side to move may end its move on, ignoring the lock rule.
    const fn allowed_targets(&self, board: &Board) -> u64 {
        if board.ply >= HOME_DEADLINE * 2 {
            !self.home[side_index(board.side)]
        } else {
            !0
        }
    }

    /// Whether moving a piece of the side to move from `from` to `to` locks a piece of
    /// the opponent that was free. `before` is `locked` for the position itself.
    const fn locks(board: &Board, from: u8, to: u8, before: u64) -> bool {
        let enemies = board.other();
        if enemies & NEAR[to as usize] == 0 {
            return false;
        }
        let change = (1u64 << from) | (1u64 << to);
        locked(board.own() ^ change, enemies) & !before != 0
    }

    /// All legal moves of the side to move as pairs of end cells, one for each
    /// reachable cell, in the order of the pieces.
    pub fn moves(&self, board: &Board, out: &mut Vec<(u8, u8)>) {
        out.clear();
        let own = board.own();
        let occupied = board.occupied();
        let allowed = self.allowed_targets(board);
        let before = locked(own, board.other());
        let mut pieces = own;
        while pieces != 0 {
            let from = pieces.trailing_zeros() as u8;
            let bit = 1u64 << from;
            pieces &= pieces - 1;
            let rest = occupied & !bit;
            let mut targets = ((adjacent(bit) & !occupied) | jump_closure(bit, rest)) & allowed;
            while targets != 0 {
                let to = targets.trailing_zeros() as u8;
                targets &= targets - 1;
                if !Self::locks(board, from, to, before) {
                    out.push((from, to));
                }
            }
        }
    }

    /// Whether the side to move has a legal move. Usually decided by the first piece
    /// that can step.
    #[must_use]
    pub fn has_move(&self, board: &Board) -> bool {
        let own = board.own();
        let occupied = board.occupied();
        let allowed = self.allowed_targets(board);
        let before = locked(own, board.other());
        let mut pieces = own;
        while pieces != 0 {
            let from = pieces.trailing_zeros() as u8;
            let bit = 1u64 << from;
            pieces &= pieces - 1;
            let mut targets = adjacent(bit) & !occupied & allowed;
            if targets == 0 {
                targets = jump_closure(bit, occupied & !bit) & allowed;
            }
            while targets != 0 {
                let to = targets.trailing_zeros() as u8;
                targets &= targets - 1;
                if !Self::locks(board, from, to, before) {
                    return true;
                }
            }
        }
        // Steps were tried first; a piece whose steps all lock may still jump.
        let mut moves = Vec::new();
        self.moves(board, &mut moves);
        !moves.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::ugolki::State;
    use crate::games::{squares_of, GameKind, Position, Square};
    use crate::rng::Rng;
    use std::collections::BTreeSet;

    fn board(state: State) -> Board {
        Board {
            white: state.pos.white,
            black: state.pos.black,
            side: state.pos.side,
            ply: state.ply,
        }
    }

    fn random_state(rng: &mut Rng, layout: HomeSide) -> State {
        let mut cells: Vec<_> = Square::all().collect();
        rng.shuffle(&mut cells);
        let white = 1 + rng.below(12);
        let black = 1 + rng.below(12);
        let mut pos = Position::empty(
            if rng.chance(0.5) {
                Color::White
            } else {
                Color::Black
            },
            GameKind::Ugolki,
        );
        pos.home_side = layout;
        pos.white = cells[..white].iter().fold(0, |mask, sq| mask | sq.bit());
        pos.black = cells[white..white + black]
            .iter()
            .fold(0, |mask, sq| mask | sq.bit());
        let ply = [0, 20, 79, 80, 81, 100, 159, 160, 161][rng.below(9)];
        State { pos, ply }
    }

    /// Positions crowded around homes, where locks and deadlines matter most.
    fn crowded_state(rng: &mut Rng, layout: HomeSide) -> State {
        let mut state = random_state(rng, layout);
        let homes = home_for(Color::White, layout) | home_for(Color::Black, layout);
        for square in squares_of(homes) {
            if state.pos.occupied() & square.bit() == 0 && rng.chance(0.6) {
                if rng.chance(0.5) && state.pos.white.count_ones() < 12 {
                    state.pos.white |= square.bit();
                } else if state.pos.black.count_ones() < 12 {
                    state.pos.black |= square.bit();
                }
            }
        }
        state
    }

    #[test]
    fn the_fast_rules_match_the_rules_of_the_game() {
        let mut rng = Rng::new(0x5EED_F00D);
        let mut moves = Vec::new();
        for iteration in 0..6000 {
            let layout = HomeSide::ALL[iteration % 2];
            let state = if iteration % 3 == 0 {
                crowded_state(&mut rng, layout)
            } else {
                random_state(&mut rng, layout)
            };
            let rules = Rules::new(layout);
            let board = board(state);
            rules.moves(&board, &mut moves);
            let fast: BTreeSet<_> = moves.iter().copied().collect();
            assert_eq!(fast.len(), moves.len(), "one move for each pair of cells");
            let expected: BTreeSet<_> = state
                .legal_moves()
                .iter()
                .map(|mv| (mv.from().index() as u8, mv.to().index() as u8))
                .collect();
            assert_eq!(fast, expected, "{}, ply {}", state.pos.to_fen(), state.ply);
            assert_eq!(rules.has_move(&board), !expected.is_empty());
            for occurrences in 1..=3 {
                let expected = state.outcome(occurrences).map(|outcome| match outcome.winner() {
                    Some(color) => Finish::Won(color),
                    None => Finish::Drawn,
                });
                let fast = rules
                    .adjudicate(&board, occurrences)
                    .or_else(|| (!rules.has_move(&board)).then(|| rules.without_moves(&board)));
                assert_eq!(fast, expected, "{}, ply {}", state.pos.to_fen(), state.ply);
            }
            for &(from, to) in &moves {
                let mv = state
                    .legal_moves()
                    .into_iter()
                    .find(|mv| mv.from().index() == usize::from(from) && mv.to().index() == usize::from(to))
                    .unwrap();
                let next = state.after(&mv);
                let fast = board.after(from, to);
                assert_eq!(
                    (fast.white, fast.black, fast.side, fast.ply),
                    (next.pos.white, next.pos.black, next.pos.side, next.ply)
                );
            }
        }
    }
}
