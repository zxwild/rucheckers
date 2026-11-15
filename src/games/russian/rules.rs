//! The end of a game: a win and the automatic draws.
//!
//! "N moves" in the rules means N moves by each player, so the counters that count the
//! moves of both sides are kept in plies and compared with twice the limit. The
//! counters that count the moves of one (the stronger) side are kept in its moves.
//!
//! The "balance of forces" is the number of men and kings of each side. It changes with
//! every capture and with every promotion of a man; the counters of rules 2, 4 and 5
//! are then reset.

use crate::board::{Color, Position, MAIN_ROAD};
use crate::moves::{generate_moves, Move};

/// Rule 1: the position with the same side to move has occurred for the third time.
pub const REPETITION_LIMIT: u32 = 3;
/// Rule 2: three or more kings against one king. The stronger side has this many moves
/// to capture it, counted from the moment this balance of forces arose.
pub const KINGS_VS_KING_MOVES: u32 = 15;
/// Rule 3: this many moves in a row both sides move kings only, without captures.
pub const KINGS_ONLY_MOVES: u32 = 15;
/// Rule 4: limit of moves without a change in the balance of forces with 2–3 pieces on the board.
pub const BALANCE_MOVES_2_3: u32 = 5;
/// Rule 4: limit of moves with 4–5 pieces.
pub const BALANCE_MOVES_4_5: u32 = 30;
/// Rule 4: limit of moves with 6–7 pieces.
pub const BALANCE_MOVES_6_7: u32 = 60;
/// Rule 5: three pieces against a lone king on the main road. The stronger side has
/// this many moves to capture it.
pub const MAIN_ROAD_MOVES: u32 = 5;

/// Reason for a draw.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DrawReason {
    /// Rule 1: threefold repetition of a position.
    Repetition,
    /// Rule 2: three or more kings did not capture a lone king in 15 moves.
    KingsVsKing,
    /// Rule 3: 15 moves by kings only, without captures.
    KingsOnly,
    /// Rule 4: the balance of forces did not change for the given number of moves.
    /// `pieces` is the number of pieces on the board, `moves` is the limit that was hit.
    Balance { pieces: u32, moves: u32 },
    /// Rule 5: three pieces did not capture a lone king on the main road in 5 moves.
    MainRoad,
}

/// Reason for a win.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WinReason {
    /// The loser has no pieces left.
    NoPieces,
    /// The loser has no legal moves.
    NoMoves,
}

/// Result of a finished game.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Outcome {
    Win { winner: Color, reason: WinReason },
    Draw(DrawReason),
}

impl Outcome {
    /// The winner, unless the game ended in a draw.
    #[must_use]
    pub const fn winner(&self) -> Option<Color> {
        match self {
            Self::Win { winner, .. } => Some(*winner),
            Self::Draw(_) => None,
        }
    }

    #[must_use]
    pub const fn is_draw(&self) -> bool {
        matches!(self, Self::Draw(_))
    }
}

/// Counters of the draw rules that depend on the course of the game.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct DrawCounters {
    /// Rule 2: moves of the stronger side since the balance "three or more kings
    /// against one king" arose.
    pub kings_vs_king: u8,
    /// Rule 3: plies in a row made by kings without captures.
    pub kings_only: u8,
    /// Rule 4: plies without a change in the balance of forces, while both sides have
    /// kings and there are at most seven pieces on the board.
    pub balance: u8,
    /// Rule 5: moves of the stronger side while its three pieces face a lone king on
    /// the main road.
    pub main_road: u8,
}

/// Rule 2: the side that has three or more kings against the opponent's lone king.
#[must_use]
pub fn kings_vs_king_side(pos: &Position) -> Option<Color> {
    for strong in [Color::White, Color::Black] {
        let weak = strong.opponent();
        // The weaker side has a single piece, and that piece is a king.
        if pos.kings_of(strong).count_ones() >= 3 && pos.count(weak) == 1 && pos.kings_of(weak) != 0 {
            return Some(strong);
        }
    }
    None
}

/// Rule 5: the side that has exactly three pieces (any mix of kings and men) against
/// the opponent's lone king standing on the main road.
#[must_use]
pub fn main_road_side(pos: &Position) -> Option<Color> {
    for strong in [Color::White, Color::Black] {
        let weak = strong.opponent();
        if pos.count(strong) == 3 && pos.count(weak) == 1 && pos.kings_of(weak) & MAIN_ROAD != 0 {
            return Some(strong);
        }
    }
    None
}

/// Rule 4: the limit of moves for the current number of pieces, if both sides have kings.
#[must_use]
pub const fn balance_limit(pos: &Position) -> Option<u32> {
    if pos.kings_of(Color::White) == 0 || pos.kings_of(Color::Black) == 0 {
        return None;
    }
    match pos.total() {
        2 | 3 => Some(BALANCE_MOVES_2_3),
        4 | 5 => Some(BALANCE_MOVES_4_5),
        6 | 7 => Some(BALANCE_MOVES_6_7),
        _ => None,
    }
}

/// State of a game: the position and the draw counters accumulated along the branch that
/// leads to it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GameState {
    pub pos: Position,
    pub counters: DrawCounters,
}

impl GameState {
    /// A state with the counters at zero.
    #[must_use]
    pub fn new(pos: Position) -> Self {
        Self {
            pos,
            counters: DrawCounters::default(),
        }
    }

    /// The start of a game.
    #[must_use]
    pub fn initial() -> Self {
        Self::new(Position::initial())
    }

    /// The state after the move `mv` (the move must be legal in this position).
    #[must_use]
    pub fn play(&self, mv: &Move) -> Self {
        let mover = self.pos.side;
        let king_move = self.pos.kings & mv.from().bit() != 0;
        let capture = mv.is_capture();
        let balance_changed = capture || mv.promotes();
        let next = self.pos.after(mv);
        let old = self.counters;

        // Rule 3: only quiet moves of kings count.
        let kings_only = if king_move && !capture {
            old.kings_only.saturating_add(1)
        } else {
            0
        };

        // Rule 4.
        let balance = if !balance_changed && balance_limit(&next).is_some() {
            old.balance.saturating_add(1)
        } else {
            0
        };

        // Rule 2: the moves of the stronger side count.
        let kings_vs_king = match kings_vs_king_side(&next) {
            Some(strong) if !balance_changed => {
                if mover == strong {
                    old.kings_vs_king.saturating_add(1)
                } else {
                    old.kings_vs_king
                }
            }
            _ => 0,
        };

        // Rule 5: the count starts when the lone king is on the main road against three
        // pieces of the opponent, and stops if the king leaves the road.
        let main_road = match main_road_side(&next) {
            Some(strong) if !balance_changed && main_road_side(&self.pos) == Some(strong) => {
                if mover == strong {
                    old.main_road.saturating_add(1)
                } else {
                    old.main_road
                }
            }
            _ => 0,
        };

        Self {
            pos: next,
            counters: DrawCounters {
                kings_vs_king,
                kings_only,
                balance,
                main_road,
            },
        }
    }

    /// A draw by the counters (rules 2–5), if one of the limits has been reached.
    #[must_use]
    pub fn counter_draw(&self) -> Option<DrawReason> {
        let c = &self.counters;
        if u32::from(c.kings_vs_king) >= KINGS_VS_KING_MOVES {
            return Some(DrawReason::KingsVsKing);
        }
        if u32::from(c.kings_only) >= 2 * KINGS_ONLY_MOVES {
            return Some(DrawReason::KingsOnly);
        }
        if let Some(limit) = balance_limit(&self.pos) {
            if u32::from(c.balance) >= 2 * limit {
                return Some(DrawReason::Balance {
                    pieces: self.pos.total(),
                    moves: limit,
                });
            }
        }
        if u32::from(c.main_road) >= MAIN_ROAD_MOVES {
            return Some(DrawReason::MainRoad);
        }
        None
    }

    /// In how many plies the earliest of rules 2–5 will declare a draw if the balance
    /// of forces does not change and no man moves. `u32::MAX` if no rule is counting
    /// at the moment.
    #[must_use]
    pub fn plies_to_counter_draw(&self) -> u32 {
        let c = &self.counters;
        let mut best = u32::MAX;
        if self.pos.kings != 0 {
            best = best.min((2 * KINGS_ONLY_MOVES).saturating_sub(u32::from(c.kings_only)));
        }
        if let Some(limit) = balance_limit(&self.pos) {
            best = best.min((2 * limit).saturating_sub(u32::from(c.balance)));
        }
        let one_sided = |limit: u32, done: u8, strong: Color| -> u32 {
            let left = limit.saturating_sub(u32::from(done));
            if left == 0 {
                0
            } else if self.pos.side == strong {
                2 * left - 1
            } else {
                2 * left
            }
        };
        if let Some(strong) = kings_vs_king_side(&self.pos) {
            best = best.min(one_sided(KINGS_VS_KING_MOVES, c.kings_vs_king, strong));
        }
        if let Some(strong) = main_road_side(&self.pos) {
            best = best.min(one_sided(MAIN_ROAD_MOVES, c.main_road, strong));
        }
        best
    }

    /// Result of the game in this state.
    ///
    /// `occurrences` is how many times this position (with the same side to move) has
    /// occurred on the branch that leads to it, the current occurrence included.
    ///
    /// The loss of a side with nothing to move is checked first: a move that leaves the
    /// opponent without moves wins even if the same move exhausts a draw limit.
    pub fn outcome(&self, occurrences: u32) -> Option<Outcome> {
        let mut moves = Vec::new();
        generate_moves(&self.pos, &mut moves);
        if moves.is_empty() {
            let loser = self.pos.side;
            let reason = if self.pos.count(loser) == 0 {
                WinReason::NoPieces
            } else {
                WinReason::NoMoves
            };
            return Some(Outcome::Win {
                winner: loser.opponent(),
                reason,
            });
        }
        if occurrences >= REPETITION_LIMIT {
            return Some(Outcome::Draw(DrawReason::Repetition));
        }
        self.counter_draw().map(Outcome::Draw)
    }
}

/// How many times the position `current` occurs among `earlier` (the earlier positions
/// of the branch), itself included.
#[must_use]
pub fn occurrences(earlier: &[Position], current: &Position) -> u32 {
    1 + earlier.iter().filter(|p| *p == current).count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_match_the_rules() {
        assert_eq!(REPETITION_LIMIT, 3);
        assert_eq!(KINGS_VS_KING_MOVES, 15);
        assert_eq!(KINGS_ONLY_MOVES, 15);
        assert_eq!(
            (BALANCE_MOVES_2_3, BALANCE_MOVES_4_5, BALANCE_MOVES_6_7),
            (5, 30, 60)
        );
        assert_eq!(MAIN_ROAD_MOVES, 5);
    }

    #[test]
    fn balance_limit_depends_on_piece_count() {
        let p = |fen: &str| Position::from_fen(fen).unwrap();
        assert_eq!(balance_limit(&p("W:WKa1:BKh8")), Some(5));
        assert_eq!(balance_limit(&p("W:WKa1,c1:BKh8")), Some(5));
        assert_eq!(balance_limit(&p("W:WKa1,c1,e1:BKh8")), Some(30));
        assert_eq!(balance_limit(&p("W:WKa1,c1,e1:BKh8,b8")), Some(30));
        assert_eq!(balance_limit(&p("W:WKa1,c1,e1:BKh8,b8,d8")), Some(60));
        assert_eq!(balance_limit(&p("W:WKa1,c1,e1,g1:BKh8,b8,d8")), Some(60));
        assert_eq!(balance_limit(&p("W:WKa1,c1,e1,g1:BKh8,b8,d8,f8")), None);
        assert_eq!(balance_limit(&p("W:WKa1,c1:Bh8")), None, "Black has no king");
        assert_eq!(balance_limit(&Position::initial()), None);
    }

    #[test]
    fn strong_side_detection() {
        let p = |fen: &str| Position::from_fen(fen).unwrap();
        assert_eq!(kings_vs_king_side(&p("W:WKa1,Kc1,Ke1:BKh8")), Some(Color::White));
        assert_eq!(
            kings_vs_king_side(&p("W:WKa1:BKh8,Kb8,Kd8,Kf8")),
            Some(Color::Black)
        );
        assert_eq!(
            kings_vs_king_side(&p("W:WKa1,Kc1,e1:BKh8")),
            None,
            "only two kings"
        );
        assert_eq!(
            kings_vs_king_side(&p("W:WKa1,Kc1,Ke1:Bh8")),
            None,
            "a man, not a king"
        );
        assert_eq!(
            kings_vs_king_side(&p("W:WKa1,Kc1,Ke1:BKh8,b8")),
            None,
            "more than one king"
        );
        assert_eq!(
            kings_vs_king_side(&p("W:WKa1,Kc1,Ke1,g1:BKh8")),
            Some(Color::White)
        );

        assert_eq!(main_road_side(&p("W:Wc1,e1,g1:BKh8")), Some(Color::White));
        assert_eq!(main_road_side(&p("W:WKc1,e1,g1:BKd4")), Some(Color::White));
        assert_eq!(
            main_road_side(&p("W:WKc1,e1,g1:BKb8")),
            None,
            "the king is not on the main road"
        );
        assert_eq!(main_road_side(&p("W:WKc1,e1,g1:Bd4")), None, "a man on the road");
        assert_eq!(main_road_side(&p("W:WKc1,e1:BKd4")), None, "two pieces");
        assert_eq!(main_road_side(&p("W:WKc1,e1,g1,a3:BKd4")), None, "four pieces");
        assert_eq!(main_road_side(&p("W:WKd4:BKc1,e3,g3")), Some(Color::Black));
    }
}
