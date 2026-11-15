//! Dispatch to the independent game engines.

use super::{russian, ugolki, Color, GameKind, Move, Position};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FinishReason {
    HomeFilled,
    HomeDeadline,
    MoveLimit,
    Simultaneous,
    Repetition,
    Blocked,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Russian(russian::rules::Outcome),
    Win { winner: Color, reason: FinishReason },
    Draw(FinishReason),
}
impl Outcome {
    #[must_use]
    pub const fn winner(self) -> Option<Color> {
        match self {
            Self::Russian(outcome) => outcome.winner(),
            Self::Win { winner, .. } => Some(winner),
            Self::Draw(_) => None,
        }
    }
    #[must_use]
    pub const fn is_draw(self) -> bool {
        self.winner().is_none()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NativeState {
    Russian(russian::rules::GameState),
    Ugolki(ugolki::State),
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GameState {
    pub pos: Position,
    native: NativeState,
}
impl GameState {
    #[must_use]
    pub fn new(pos: Position) -> Self {
        let native = match pos.game {
            GameKind::RussianDraughts => NativeState::Russian(russian::rules::GameState::new(pos.draughts())),
            GameKind::Ugolki => NativeState::Ugolki(ugolki::State::new(pos)),
        };
        Self { pos, native }
    }
    #[must_use]
    pub fn legal_moves(self) -> Vec<Move> {
        match self.native {
            NativeState::Russian(state) => russian::moves::legal_moves(&state.pos)
                .into_iter()
                .map(Move::from_russian)
                .collect(),
            NativeState::Ugolki(state) => state.legal_moves(),
        }
    }
    /// Applies a move returned by this state's engine.
    ///
    /// # Panics
    /// Panics if a Russian draughts state receives a move from another engine.
    #[must_use]
    pub fn after(self, mv: &Move) -> Self {
        match self.native {
            NativeState::Russian(state) => {
                let next = state.play(&mv.russian.expect("draughts move"));
                Self {
                    pos: Position::from_russian(next.pos),
                    native: NativeState::Russian(next),
                }
            }
            NativeState::Ugolki(state) => {
                let next = state.after(mv);
                Self {
                    pos: next.pos,
                    native: NativeState::Ugolki(next),
                }
            }
        }
    }
    #[must_use]
    pub fn outcome(self, occurrences: u32) -> Option<Outcome> {
        match self.native {
            NativeState::Russian(state) => state.outcome(occurrences).map(Outcome::Russian),
            NativeState::Ugolki(state) => state.outcome(occurrences),
        }
    }
}

#[must_use]
pub fn legal_moves(pos: &Position) -> Vec<Move> {
    GameState::new(*pos).legal_moves()
}

pub fn choose_move(
    state: GameState,
    history: &[Position],
    level: u8,
    seed: u64,
    stop: &std::sync::atomic::AtomicBool,
) -> Option<Move> {
    match state.native {
        NativeState::Russian(native) => {
            let history: Vec<_> = history.iter().copied().map(Position::draughts).collect();
            russian::ai::choose_move(&native, &history, level, seed, stop)
                .map(|choice| Move::from_russian(choice.mv))
        }
        NativeState::Ugolki(native) => ugolki::ai::choose_move(native, history, level, seed, stop),
    }
}
