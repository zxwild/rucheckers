//! Timing of a move animation: where the piece is at each moment, when it is promoted,
//! when captured pieces disappear. The module does not draw; it only computes a frame.
//!
//! An ordinary capture: the piece that was jumped over dims and disappears after the
//! move is finished. If several pieces are captured in a move and special effects are
//! on, the move comes with an electric discharge ([`crate::ui::fx`]): lightning trails the
//! flying piece, and each captured piece disappears the moment the piece flies over it.

use super::Move;
use super::{Piece, Square};
use crate::ui::fx::{self, Discharge, Strike};
use crate::ui::geometry::screen_cell;
use crate::ui::promotion::Promotion;

/// Duration of a move to a neighbouring square, s.
pub const STEP_SECONDS: f64 = 0.17;
/// Addition for every further square of a long king move, s.
pub const EXTRA_SQUARE_SECONDS: f64 = 0.04;
/// Time a captured piece takes to disappear after the move is finished (without a discharge), s.
pub const FADE_SECONDS: f64 = 0.20;
/// Opacity of a captured piece while the move is not finished and it still stands on the board.
pub const CAPTURED_ALPHA: f32 = 0.42;

/// Duration of one leg of the route, between the squares `a` and `b`.
#[must_use]
pub fn segment_seconds(a: Square, b: Square) -> f64 {
    let distance = a
        .file()
        .abs_diff(b.file())
        .max(a.rank().abs_diff(b.rank()))
        .max(1);
    STEP_SECONDS + EXTRA_SQUARE_SECONDS * f64::from(distance - 1)
}

/// A frame of the animation.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AnimFrame {
    /// Position of the centre of the piece in board coordinates (file and rank, fractional).
    pub file: f32,
    pub rank: f32,
    /// Whether the piece is drawn as a king.
    pub is_king: bool,
    /// Mask of the captured pieces the piece has already flown over.
    pub jumped: u64,
    /// The piece has reached its final square.
    pub arrived: bool,
    /// The animation is completely finished.
    pub finished: bool,
}

/// Animation of one move.
#[derive(Clone, Debug)]
pub struct MoveAnimation {
    mv: Move,
    piece: Piece,
    first_step: usize,
    start: f64,
    /// The moments each animated leg ends, relative to the start.
    ends: Vec<f64>,
    /// Whether the move comes with an electric discharge.
    discharge: bool,
}

impl MoveAnimation {
    /// Animation of the move `mv` by the piece `piece` (as it was before the move).
    ///
    /// `first_step` is how many jumps have already been shown (the player made them step
    /// by step); the rest are animated. `start` is the starting time. A capture of
    /// several pieces comes with a discharge; [`Self::with_discharge`] can switch it off.
    #[must_use]
    pub fn new(mv: Move, piece: Piece, first_step: usize, start: f64) -> Self {
        let first_step = first_step.min(mv.path().len());
        let points: Vec<Square> = mv.squares().collect();
        let mut ends = Vec::new();
        let mut elapsed = 0.0;
        for pair in points[first_step..].windows(2) {
            elapsed += segment_seconds(pair[0], pair[1]);
            ends.push(elapsed);
        }
        let discharge = mv.captured_count() >= fx::MIN_CAPTURES;
        Self {
            mv,
            piece,
            first_step,
            start,
            ends,
            discharge,
        }
    }

    /// Switches the discharge on or off (it is possible only when several pieces are captured).
    #[must_use]
    pub const fn with_discharge(mut self, enabled: bool) -> Self {
        self.discharge = enabled && self.mv.captured_count() >= fx::MIN_CAPTURES;
        self
    }

    /// The animated move.
    #[must_use]
    pub const fn mv(&self) -> &Move {
        &self.mv
    }

    /// The piece before the move.
    #[must_use]
    pub const fn piece(&self) -> Piece {
        self.piece
    }

    /// The starting time.
    #[must_use]
    pub const fn start(&self) -> f64 {
        self.start
    }

    /// Duration of the piece's travel.
    #[must_use]
    pub fn travel_seconds(&self) -> f64 {
        self.ends.last().copied().unwrap_or(0.0)
    }

    /// Whether the move comes with an electric discharge.
    #[must_use]
    pub const fn has_discharge(&self) -> bool {
        self.discharge
    }

    /// When (from the start of the animation) the piece is on the square number `index` of
    /// the route (0 is the starting square). Squares of jumps already shown are passed at
    /// the start.
    fn knot_seconds(&self, index: usize) -> f64 {
        if index <= self.first_step {
            0.0
        } else {
            self.ends
                .get(index - self.first_step - 1)
                .copied()
                .unwrap_or_else(|| self.travel_seconds())
        }
    }

    /// When (from the start of the animation) the piece flies over the piece captured by
    /// the jump `step`. For jumps already shown: at the very start.
    fn pass_seconds(&self, step: usize) -> f64 {
        let (begin, end) = (self.knot_seconds(step), self.knot_seconds(step + 1));
        if end <= begin {
            return begin;
        }
        let points: Vec<Square> = self.mv.squares().collect();
        let (Some(&a), Some(&b)) = (points.get(step), points.get(step + 1)) else {
            return end;
        };
        // The captured piece is not necessarily in the middle of the jump: a king captures
        // from afar.
        let length = (i32::from(b.file()) - i32::from(a.file())).abs().max(1);
        let along = match self.mv.captured_at(step) {
            Some(victim) => {
                f64::from((i32::from(victim.file()) - i32::from(a.file())).abs()) / f64::from(length)
            }
            None => 0.5,
        };
        begin + (end - begin) * fx::ease_inverse(along)
    }

    /// The moment of the first strike of the discharge (in absolute time), if there is a discharge.
    #[must_use]
    pub fn first_strike_time(&self) -> Option<f64> {
        self.discharge.then(|| self.start + self.pass_seconds(0))
    }

    /// Opacity of the piece on the square `sq` at the moment `now`.
    ///
    /// Always 1 for pieces this move does not capture. Without a discharge a captured
    /// piece dims when it is jumped over and disappears after the move is finished. With
    /// a discharge it disappears as soon as the capturing piece has flown over it.
    #[must_use]
    pub fn captured_alpha(&self, sq: Square, now: f64) -> f32 {
        let Some(step) = (0..self.mv.path().len()).find(|&i| self.mv.captured_at(i) == Some(sq)) else {
            return 1.0;
        };
        let t = now - self.start;
        let passed = self.pass_seconds(step);
        // The pieces of jumps already shown were standing on the board dimmed.
        let shown_before = step < self.first_step;
        let fade = |since: f64, duration: f64| (1.0 - (t - since) / duration).clamp(0.0, 1.0) as f32;
        if self.discharge {
            let base = if shown_before { CAPTURED_ALPHA } else { 1.0 };
            base * fade(passed, fx::VANISH_SECONDS)
        } else if t < passed && !shown_before {
            1.0
        } else {
            CAPTURED_ALPHA * fade(self.travel_seconds(), FADE_SECONDS)
        }
    }

    /// Total duration. A move with a discharge is finished when the lightning has gone
    /// out (sparks are still flying, but they do not get in the way of the next move);
    /// an ordinary capture, when the captured pieces have disappeared.
    #[must_use]
    pub fn total_seconds(&self) -> f64 {
        let tail = if self.discharge {
            fx::TRAIL_SECONDS
        } else if self.mv.is_capture() {
            FADE_SECONDS
        } else {
            0.0
        };
        self.travel_seconds() + tail
    }

    /// The moments (in absolute time) the piece lands on each next square.
    #[must_use]
    pub fn landing_times(&self) -> Vec<f64> {
        self.ends.iter().map(|end| self.start + end).collect()
    }

    /// The rose starts on the promotion landing, including a final square reached by
    /// dragging. An intermediate promotion already shown step by step is not replayed.
    #[must_use]
    pub fn promotion(&self) -> Option<Promotion> {
        let step = self.mv.promotion_step()?;
        if self.piece.is_king() || (step < self.first_step && step + 1 < self.mv.path().len()) {
            return None;
        }
        Some(Promotion::new(
            self.mv.path()[step].draughts()?,
            self.piece.color,
            self.start + self.knot_seconds(step + 1),
        ))
    }

    /// The discharge that comes with the move: the route of the piece with its timing
    /// and the strikes on the captured pieces, in screen cells. `flipped`: the board is
    /// turned (Black at the bottom).
    #[must_use]
    pub fn discharge(&self, flipped: bool, seed: u64) -> Option<Discharge> {
        if !self.discharge {
            return None;
        }
        let cell = |sq: Square| screen_cell(f32::from(sq.file()), f32::from(sq.rank()), flipped);
        let points: Vec<Square> = self.mv.squares().collect();
        let knots = points
            .iter()
            .enumerate()
            .map(|(index, &sq)| (cell(sq), self.start + self.knot_seconds(index)))
            .collect();
        let mut strikes = Vec::new();
        for step in 0..self.mv.path().len() {
            let Some(victim) = self.mv.captured_at(step) else {
                continue;
            };
            let (from, to) = (cell(points[step]), cell(points[step + 1]));
            let length = (to.0 - from.0).hypot(to.1 - from.1).max(1e-6);
            strikes.push(Strike {
                center: cell(victim),
                time: self.start + self.pass_seconds(step),
                direction: ((to.0 - from.0) / length, (to.1 - from.1) / length),
            });
        }
        Some(Discharge::new(knots, strikes, seed))
    }

    /// The frame at the moment `now`.
    #[must_use]
    pub fn frame(&self, now: f64) -> AnimFrame {
        let t = now - self.start;
        let points: Vec<Square> = self.mv.squares().collect();
        let promotion = self.mv.promotion_step();
        let mut jumped = 0u64;
        for step in 0..self.mv.path().len() {
            if t >= self.pass_seconds(step) {
                if let Some(sq) = self.mv.captured_at(step) {
                    jumped |= sq.bit();
                }
            }
        }

        let mut segment_start = 0.0;
        for (i, &end) in self.ends.iter().enumerate() {
            let step = self.first_step + i;
            if t < end {
                let duration = (end - segment_start).max(1e-6);
                let eased = fx::ease((t - segment_start) / duration) as f32;
                let (a, b) = (points[step], points[step + 1]);
                return AnimFrame {
                    file: lerp(f32::from(a.file()), f32::from(b.file()), eased),
                    rank: lerp(f32::from(a.rank()), f32::from(b.rank()), eased),
                    is_king: self.piece.is_king() || promotion.is_some_and(|p| p < step),
                    jumped,
                    arrived: false,
                    finished: false,
                };
            }
            segment_start = end;
        }

        let to = self.mv.to();
        AnimFrame {
            file: f32::from(to.file()),
            rank: f32::from(to.rank()),
            is_king: self.piece.is_king() || promotion.is_some(),
            jumped: self.mv.captured(),
            arrived: true,
            finished: t >= self.total_seconds(),
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
