//! Russian orthogonal Ugolki, 12 pieces in rectangular 3×4 homes.
//!
//! A turn is one orthogonal step or any nonempty chain of jumps over adjacent
//! pieces of either colour. No pieces are captured. White starts and Black gets
//! the equalising final reply. Homes must be cleared by move 40; move 80 compares
//! the number of arrived pieces. A third occurrence is a draw. Enemy pieces must
//! not be locked without any step or jump by the moving player's pieces.

pub mod ai;

use super::{squares_of, Color, FinishReason, GameKind, Move, Outcome, Position, Square};
use std::collections::VecDeque;

pub const PIECES: usize = 12;
pub const HOME_DEADLINE: u16 = 40;
pub const MOVE_LIMIT: u16 = 80;
const DIRECTIONS: [(i8, i8); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];

#[must_use]
pub fn home(color: Color) -> u64 {
    Square::all()
        .filter(|sq| match color {
            Color::White => sq.file() < 4 && sq.rank() < 3,
            Color::Black => sq.file() >= 4 && sq.rank() >= 5,
        })
        .fold(0, |mask, sq| mask | sq.bit())
}

#[must_use]
pub fn initial() -> Position {
    Position {
        white: home(Color::White),
        black: home(Color::Black),
        kings: 0,
        side: Color::White,
        game: GameKind::Ugolki,
    }
}

fn neighbour(square: Square, dx: i8, dy: i8) -> Option<Square> {
    let file = square.file() as i8 + dx;
    let rank = square.rank() as i8 + dy;
    if !(0..8).contains(&file) || !(0..8).contains(&rank) {
        return None;
    }
    Square::from_file_rank(file as u8, rank as u8)
}

fn moved(mut pos: Position, mv: &Move) -> Position {
    let bit = mv.from().bit();
    match pos.side {
        Color::White => pos.white = (pos.white & !bit) | mv.to().bit(),
        Color::Black => pos.black = (pos.black & !bit) | mv.to().bit(),
    }
    pos.side = pos.side.opponent();
    pos
}

fn locked_by(pos: Position, square: Square, mover: Color) -> bool {
    let occupied = pos.occupied();
    let mut participates = false;
    for (dx, dy) in DIRECTIONS {
        let Some(adjacent) = neighbour(square, dx, dy) else {
            continue;
        };
        if occupied & adjacent.bit() == 0 {
            return false;
        }
        participates |= pos.pieces(mover) & adjacent.bit() != 0;
        if let Some(landing) = neighbour(adjacent, dx, dy) {
            if occupied & landing.bit() == 0 {
                return false;
            }
            participates |= pos.pieces(mover) & landing.bit() != 0;
        }
    }
    participates
}

fn no_enemy_lock(pos: Position, mover: Color) -> bool {
    squares_of(pos.pieces(mover.opponent())).all(|sq| !locked_by(pos, sq, mover))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct State {
    pub pos: Position,
    pub ply: u16,
}

impl State {
    #[must_use]
    pub fn new(pos: Position) -> Self {
        Self {
            pos,
            ply: u16::from(pos.side == Color::Black),
        }
    }

    #[must_use]
    pub fn legal_moves(self) -> Vec<Move> {
        let mut moves = Vec::new();
        for origin in squares_of(self.pos.pieces(self.pos.side)) {
            let occupied = self.pos.occupied() & !origin.bit();
            for (dx, dy) in DIRECTIONS {
                if let Some(to) = neighbour(origin, dx, dy) {
                    if occupied & to.bit() == 0 {
                        moves.push(Move::route(origin, &[to]));
                    }
                }
            }
            // One shortest simple route per reachable endpoint is sufficient: jumps
            // never remove pieces, so all such routes lead to the same board.
            let mut parents = [None; 64];
            let mut visited = origin.bit();
            let mut queue = VecDeque::from([origin]);
            while let Some(from) = queue.pop_front() {
                for (dx, dy) in DIRECTIONS {
                    let Some(over) = neighbour(from, dx, dy) else {
                        continue;
                    };
                    let Some(to) = neighbour(over, dx, dy) else {
                        continue;
                    };
                    if occupied & over.bit() == 0 || occupied & to.bit() != 0 || visited & to.bit() != 0 {
                        continue;
                    }
                    visited |= to.bit();
                    parents[to.index()] = Some(from);
                    let mut route = vec![to];
                    let mut cursor = from;
                    while let Some(parent) = parents[cursor.index()] {
                        route.push(cursor);
                        cursor = parent;
                    }
                    route.reverse();
                    moves.push(Move::route(origin, &route));
                    queue.push_back(to);
                }
            }
        }
        let own_home = home(self.pos.side);
        moves.retain(|mv| {
            (self.ply < HOME_DEADLINE * 2 || own_home & mv.to().bit() == 0)
                && no_enemy_lock(moved(self.pos, mv), self.pos.side)
        });
        moves
    }

    #[must_use]
    pub fn after(self, mv: &Move) -> Self {
        Self {
            pos: moved(self.pos, mv),
            ply: self.ply.saturating_add(1),
        }
    }

    #[must_use]
    pub fn arrived(self, color: Color) -> u32 {
        (self.pos.pieces(color) & home(color.opponent())).count_ones()
    }

    #[must_use]
    pub fn outcome(self, occurrences: u32) -> Option<Outcome> {
        let white = self.arrived(Color::White) == PIECES as u32;
        let black = self.arrived(Color::Black) == PIECES as u32;
        let win = |winner, reason| Some(Outcome::Win { winner, reason });
        if white && black {
            return Some(Outcome::Draw(FinishReason::Simultaneous));
        }
        if black {
            return win(Color::Black, FinishReason::HomeFilled);
        }
        if white && self.pos.side == Color::White {
            return win(Color::White, FinishReason::HomeFilled);
        }
        // Deadlines are assessed after Black's reply, so both players get equally
        // many turns. A forbidden late return to one's home is not offered by input.
        if self.ply >= HOME_DEADLINE * 2 && self.pos.side == Color::White {
            let left_white = self.pos.white & home(Color::White) != 0;
            let left_black = self.pos.black & home(Color::Black) != 0;
            match (left_white, left_black) {
                (true, true) => return Some(Outcome::Draw(FinishReason::HomeDeadline)),
                (true, false) => return win(Color::Black, FinishReason::HomeDeadline),
                (false, true) => return win(Color::White, FinishReason::HomeDeadline),
                _ => {}
            }
        }
        if self.ply >= MOVE_LIMIT * 2 && self.pos.side == Color::White {
            return match self.arrived(Color::White).cmp(&self.arrived(Color::Black)) {
                std::cmp::Ordering::Greater => win(Color::White, FinishReason::MoveLimit),
                std::cmp::Ordering::Less => win(Color::Black, FinishReason::MoveLimit),
                std::cmp::Ordering::Equal => Some(Outcome::Draw(FinishReason::MoveLimit)),
            };
        }
        if occurrences >= 3 {
            return Some(Outcome::Draw(FinishReason::Repetition));
        }
        if self.legal_moves().is_empty() {
            return if white {
                win(Color::White, FinishReason::HomeFilled)
            } else {
                Some(Outcome::Draw(FinishReason::Blocked))
            };
        }
        None
    }
}
