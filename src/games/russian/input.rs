//! Choosing a move with the mouse: clicks and dragging, step-by-step multi-captures.
//!
//! The module knows nothing about the window or coordinates: it receives board squares
//! and returns events. A multi-capture can be made one jump at a time (then, with
//! several routes, the player chooses the next square) or by pointing at the final
//! square at once, if that determines the result of the capture unambiguously.

use crate::board::Square;
use crate::moves::Move;

/// Result of a player's action.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputEvent {
    /// Nothing has changed.
    None,
    /// A piece is selected.
    Selected(Square),
    /// The selection is cleared.
    Cleared,
    /// An intermediate jump of a multi-capture is made; the move is not finished yet.
    Step { from: Square, to: Square },
    /// The move is chosen completely.
    Move(Move),
}

/// State of choosing a move.
#[derive(Clone, Debug, Default)]
pub struct MoveInput {
    moves: Vec<Move>,
    origin: Option<Square>,
    steps: Vec<Square>,
}

impl MoveInput {
    /// A move chooser with no moves available (input is disabled).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the legal moves of the current position and resets the selection.
    pub fn set_moves(&mut self, moves: Vec<Move>) {
        self.moves = moves;
        self.origin = None;
        self.steps.clear();
    }

    /// Disables input.
    pub fn clear(&mut self) {
        self.set_moves(Vec::new());
    }

    /// Whether a move can be made now.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        !self.moves.is_empty()
    }

    /// The legal moves.
    #[must_use]
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// The starting square of the selected piece.
    #[must_use]
    pub const fn origin(&self) -> Option<Square> {
        self.origin
    }

    /// The jumps already made in an unfinished capture.
    #[must_use]
    pub fn steps(&self) -> &[Square] {
        &self.steps
    }

    /// Whether an unfinished multi-capture is in progress.
    #[must_use]
    pub fn in_progress(&self) -> bool {
        !self.steps.is_empty()
    }

    /// The square the selected piece stands on now.
    #[must_use]
    pub fn current_square(&self) -> Option<Square> {
        self.steps.last().copied().or(self.origin)
    }

    /// Whether capturing is mandatory (all legal moves are captures).
    pub fn must_capture(&self) -> bool {
        self.moves.first().is_some_and(Move::is_capture)
    }

    /// The pieces that can move. If capturing is mandatory, these are the pieces that must capture.
    #[must_use]
    pub fn movable(&self) -> Vec<Square> {
        let mut squares: Vec<Square> = Vec::new();
        for mv in &self.moves {
            if !squares.contains(&mv.from()) {
                squares.push(mv.from());
            }
        }
        squares
    }

    /// The moves compatible with the selected piece and the jumps already made.
    fn candidates(&self) -> impl Iterator<Item = &Move> {
        let origin = self.origin;
        let steps = &self.steps;
        self.moves
            .iter()
            .filter(move |mv| Some(mv.from()) == origin && mv.path().starts_with(steps))
    }

    /// The squares the selected piece can go to with its next step.
    #[must_use]
    pub fn targets(&self) -> Vec<Square> {
        let mut squares: Vec<Square> = Vec::new();
        let next = self.steps.len();
        for mv in self.candidates() {
            if let Some(&sq) = mv.path().get(next) {
                if !squares.contains(&sq) {
                    squares.push(sq);
                }
            }
        }
        squares
    }

    /// The capture that ends on the square `sq` when that square is pointed at directly,
    /// skipping the intermediate jumps.
    ///
    /// Returns a move only if the result is unambiguous: all matching routes capture the
    /// same pieces (the first of them is taken). If captures with different results
    /// lead to one square, the route has to be given step by step.
    fn shortcut(&self, sq: Square) -> Option<Move> {
        if self.current_square() == Some(sq) {
            // A click on the piece itself clears the selection; it is not a round trip.
            return None;
        }
        let done = self.steps.len();
        let mut found: Option<Move> = None;
        for mv in self.candidates() {
            if mv.to() != sq || mv.path().len() <= done + 1 {
                continue;
            }
            match found {
                None => found = Some(*mv),
                Some(first) => {
                    if first.captured() != mv.captured() || first.promotes() != mv.promotes() {
                        return None;
                    }
                }
            }
        }
        found
    }

    /// The final squares of a multi-capture that can be pointed at directly: a click on
    /// such a square makes all the remaining jumps. Squares of the next step are not included.
    #[must_use]
    pub fn final_targets(&self) -> Vec<Square> {
        let next = self.targets();
        let mut squares: Vec<Square> = Vec::new();
        for mv in self.candidates() {
            let sq = mv.to();
            if !next.contains(&sq) && !squares.contains(&sq) && self.shortcut(sq).is_some() {
                squares.push(sq);
            }
        }
        squares
    }

    /// Chooses a square for the selected piece: the next step, or the final square of a
    /// capture at once. The step has priority: a square one jump away always means that jump.
    fn choose(&mut self, sq: Square) -> Option<InputEvent> {
        self.origin?;
        if self.targets().contains(&sq) {
            return Some(self.advance(sq));
        }
        let mv = self.shortcut(sq)?;
        self.origin = None;
        self.steps.clear();
        Some(InputEvent::Move(mv))
    }

    /// Mask of the pieces already captured in an unfinished capture (they stay on the board
    /// until the move ends).
    #[must_use]
    pub fn captured_so_far(&self) -> u32 {
        let done = self.steps.len();
        match self.candidates().next() {
            Some(mv) => (0..done)
                .filter_map(|i| mv.captured_at(i))
                .fold(0, |mask, sq| mask | sq.bit()),
            None => 0,
        }
    }

    /// Whether the selected man has become a king at one of the jumps already made.
    pub fn promoted_so_far(&self) -> bool {
        let done = self.steps.len();
        self.candidates()
            .next()
            .and_then(Move::promotion_step)
            .is_some_and(|step| step < done)
    }

    /// A click on a square.
    pub fn click(&mut self, sq: Square) -> InputEvent {
        if let Some(event) = self.choose(sq) {
            return event;
        }
        if self.in_progress() {
            // In the middle of a capture other clicks are ignored: the capture must be finished.
            return InputEvent::None;
        }
        if self.moves.iter().any(|mv| mv.from() == sq) {
            if self.origin == Some(sq) {
                self.origin = None;
                return InputEvent::Cleared;
            }
            self.origin = Some(sq);
            return InputEvent::Selected(sq);
        }
        if self.origin.take().is_some() {
            return InputEvent::Cleared;
        }
        InputEvent::None
    }

    /// The start of dragging from the square `sq`. Returns `true` if the piece can be
    /// dragged: it is selected (or has just been selected) and stands on this square.
    pub fn grab(&mut self, sq: Square) -> bool {
        if self.in_progress() {
            return self.current_square() == Some(sq);
        }
        if self.moves.iter().any(|mv| mv.from() == sq) {
            self.origin = Some(sq);
            true
        } else {
            false
        }
    }

    /// The end of dragging over the square `sq`. If the square is not available, the
    /// piece returns to its place and the selection is kept.
    pub fn drop_on(&mut self, sq: Square) -> InputEvent {
        self.choose(sq).unwrap_or(InputEvent::None)
    }

    /// Clears the selection and cancels an unfinished capture.
    pub fn cancel(&mut self) -> InputEvent {
        let had_selection = self.origin.is_some();
        self.origin = None;
        self.steps.clear();
        if had_selection {
            InputEvent::Cleared
        } else {
            InputEvent::None
        }
    }

    fn advance(&mut self, sq: Square) -> InputEvent {
        let Some(from) = self.current_square() else {
            return InputEvent::None;
        };
        self.steps.push(sq);
        let origin = self.origin;
        let complete = self
            .moves
            .iter()
            .find(|mv| Some(mv.from()) == origin && mv.path() == self.steps.as_slice())
            .copied();
        match complete {
            Some(mv) => {
                self.origin = None;
                self.steps.clear();
                InputEvent::Move(mv)
            }
            None => InputEvent::Step { from, to: sq },
        }
    }
}
