//! Расчёт анимации хода: положение шашки во времени, момент превращения,
//! затухание побитых шашек. Модуль не рисует — он только считает кадр.

use crate::board::{Piece, Square};
use crate::moves::Move;

/// Длительность перемещения на соседнее поле, с.
pub const STEP_SECONDS: f64 = 0.17;
/// Добавка за каждое следующее поле длинного хода дамки, с.
pub const EXTRA_SQUARE_SECONDS: f64 = 0.04;
/// Время исчезновения побитых шашек после завершения хода, с.
pub const FADE_SECONDS: f64 = 0.20;

/// Длительность одного отрезка пути между полями `a` и `b`.
pub fn segment_seconds(a: Square, b: Square) -> f64 {
    let distance = (i32::from(a.file()) - i32::from(b.file())).unsigned_abs().max(1);
    STEP_SECONDS + EXTRA_SQUARE_SECONDS * f64::from(distance - 1)
}

/// Кадр анимации.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AnimFrame {
    /// Положение центра шашки в координатах доски (вертикаль и горизонталь, дробные).
    pub file: f32,
    pub rank: f32,
    /// Рисовать ли шашку дамкой.
    pub is_king: bool,
    /// Маска побитых шашек, через которые шашка уже перепрыгнула.
    pub jumped: u32,
    /// Непрозрачность побитых шашек: 1 во время движения, затем убывает до 0.
    pub captured_alpha: f32,
    /// Шашка достигла конечного поля.
    pub arrived: bool,
    /// Анимация закончена полностью.
    pub finished: bool,
}

/// Анимация одного хода.
#[derive(Clone, Debug)]
pub struct MoveAnimation {
    mv: Move,
    piece: Piece,
    first_step: usize,
    start: f64,
    /// Моменты окончания каждого анимируемого отрезка относительно начала.
    ends: Vec<f64>,
}

impl MoveAnimation {
    /// Анимация хода `mv` шашкой `piece` (какой она была до хода).
    ///
    /// `first_step` — сколько прыжков уже показано (игрок сделал их по шагам);
    /// анимируются остальные. `start` — время начала.
    pub fn new(mv: Move, piece: Piece, first_step: usize, start: f64) -> MoveAnimation {
        let first_step = first_step.min(mv.path().len());
        let points: Vec<Square> = mv.squares().collect();
        let mut ends = Vec::new();
        let mut elapsed = 0.0;
        for pair in points[first_step..].windows(2) {
            elapsed += segment_seconds(pair[0], pair[1]);
            ends.push(elapsed);
        }
        MoveAnimation {
            mv,
            piece,
            first_step,
            start,
            ends,
        }
    }

    /// Анимируемый ход.
    pub fn mv(&self) -> &Move {
        &self.mv
    }

    /// Шашка до хода.
    pub fn piece(&self) -> Piece {
        self.piece
    }

    /// Время начала.
    pub fn start(&self) -> f64 {
        self.start
    }

    /// Длительность движения шашки.
    pub fn travel_seconds(&self) -> f64 {
        self.ends.last().copied().unwrap_or(0.0)
    }

    /// Полная длительность: движение и исчезновение побитых шашек.
    pub fn total_seconds(&self) -> f64 {
        self.travel_seconds() + if self.mv.is_capture() { FADE_SECONDS } else { 0.0 }
    }

    /// Моменты (в абсолютном времени), когда шашка встаёт на очередное поле.
    pub fn landing_times(&self) -> Vec<f64> {
        self.ends.iter().map(|end| self.start + end).collect()
    }

    /// Кадр в момент `now`.
    pub fn frame(&self, now: f64) -> AnimFrame {
        let t = now - self.start;
        let points: Vec<Square> = self.mv.squares().collect();
        let promotion = self.mv.promotion_step();
        let mut jumped = 0u32;
        for step in 0..self.first_step {
            if let Some(sq) = self.mv.captured_at(step) {
                jumped |= sq.bit();
            }
        }

        let mut segment_start = 0.0;
        for (i, &end) in self.ends.iter().enumerate() {
            let step = self.first_step + i;
            if t < end {
                let duration = (end - segment_start).max(1e-6);
                let local = ((t - segment_start) / duration).clamp(0.0, 1.0);
                let eased = local * local * (3.0 - 2.0 * local);
                let (a, b) = (points[step], points[step + 1]);
                if local > 0.5 {
                    if let Some(sq) = self.mv.captured_at(step) {
                        jumped |= sq.bit();
                    }
                }
                return AnimFrame {
                    file: lerp(f32::from(a.file()), f32::from(b.file()), eased as f32),
                    rank: lerp(f32::from(a.rank()), f32::from(b.rank()), eased as f32),
                    is_king: self.piece.is_king() || promotion.is_some_and(|p| p < step),
                    jumped,
                    captured_alpha: 1.0,
                    arrived: false,
                    finished: false,
                };
            }
            if let Some(sq) = self.mv.captured_at(step) {
                jumped |= sq.bit();
            }
            segment_start = end;
        }

        let to = self.mv.to();
        let travel = self.travel_seconds();
        let total = self.total_seconds();
        let captured_alpha = if self.mv.is_capture() {
            (1.0 - (t - travel) / FADE_SECONDS).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        AnimFrame {
            file: f32::from(to.file()),
            rank: f32::from(to.rank()),
            is_king: self.piece.is_king() || promotion.is_some(),
            jumped: self.mv.captured(),
            captured_alpha,
            arrived: true,
            finished: t >= total,
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Color, Kind, Position};
    use crate::moves::legal_moves;

    fn only_move(fen: &str) -> (Move, Piece) {
        let pos = Position::from_fen(fen).unwrap();
        let moves = legal_moves(&pos);
        assert_eq!(moves.len(), 1, "{moves:?}");
        (moves[0], pos.piece_at(moves[0].from()).unwrap())
    }

    #[test]
    fn quiet_move_goes_from_start_to_end() {
        let (mv, piece) = only_move("W:Wa1:Bh8,g7,f6");
        let anim = MoveAnimation::new(mv, piece, 0, 10.0);
        assert!((anim.travel_seconds() - STEP_SECONDS).abs() < 1e-9);
        assert!((anim.total_seconds() - STEP_SECONDS).abs() < 1e-9);
        let start = anim.frame(10.0);
        assert_eq!((start.file, start.rank), (0.0, 0.0));
        assert!(!start.arrived && !start.finished);
        let before = anim.frame(9.0);
        assert_eq!(
            (before.file, before.rank),
            (0.0, 0.0),
            "до начала шашка стоит на месте"
        );
        let middle = anim.frame(10.0 + STEP_SECONDS / 2.0);
        assert!((middle.file - 0.5).abs() < 1e-4 && (middle.rank - 0.5).abs() < 1e-4);
        let end = anim.frame(10.0 + STEP_SECONDS + 1e-6);
        assert_eq!((end.file, end.rank), (1.0, 1.0));
        assert!(end.arrived && end.finished);
        assert_eq!(anim.landing_times().len(), 1);
    }

    #[test]
    fn capture_marks_jumped_pieces_and_fades_them() {
        let (mv, piece) = only_move("W:Wc3:Bd4,f6");
        let anim = MoveAnimation::new(mv, piece, 0, 0.0);
        let d4 = Square::parse("d4").unwrap().bit();
        let f6 = Square::parse("f6").unwrap().bit();
        assert_eq!(anim.frame(0.01).jumped, 0);
        assert_eq!(anim.frame(STEP_SECONDS * 0.9).jumped, d4);
        assert_eq!(anim.frame(STEP_SECONDS * 1.1).jumped, d4);
        assert_eq!(anim.frame(STEP_SECONDS * 1.9).jumped, d4 | f6);
        let travel = anim.travel_seconds();
        assert!((travel - 2.0 * (STEP_SECONDS + EXTRA_SQUARE_SECONDS)).abs() < 1e-9);
        let landed = anim.frame(travel + 1e-6);
        assert!(landed.arrived && !landed.finished);
        assert!(landed.captured_alpha > 0.9);
        let fading = anim.frame(travel + FADE_SECONDS / 2.0);
        assert!((fading.captured_alpha - 0.5).abs() < 0.01);
        assert!(anim.frame(travel + FADE_SECONDS + 1e-6).finished);
    }

    #[test]
    fn promotion_during_capture_shows_a_king_after_landing() {
        // Белая простая d6 бьёт e7, встаёт на f8, становится дамкой и тут же бьёт g7.
        let (mv, piece) = only_move("W:Wd6:Be7,g7");
        assert_eq!(piece.kind, Kind::Man);
        assert_eq!(piece.color, Color::White);
        assert_eq!(mv.promotion_step(), Some(0));
        let anim = MoveAnimation::new(mv, piece, 0, 0.0);
        let first = segment_seconds(mv.from(), mv.path()[0]);
        assert!(!anim.frame(first * 0.5).is_king, "до дамочного поля — простая");
        assert!(anim.frame(first + 0.01).is_king, "после приземления — дамка");
        assert!(anim.frame(anim.total_seconds() + 1.0).is_king);
    }

    #[test]
    fn already_shown_steps_are_skipped() {
        let (mv, piece) = only_move("W:Wc3:Bd4,f6");
        let anim = MoveAnimation::new(mv, piece, 1, 0.0);
        let d4 = Square::parse("d4").unwrap().bit();
        let start = anim.frame(0.0);
        assert_eq!((start.file, start.rank), (4.0, 4.0), "анимация начинается с e5");
        assert_eq!(start.jumped, d4, "первая шашка уже побита");
        assert_eq!(anim.landing_times().len(), 1);
        let skipped = MoveAnimation::new(mv, piece, 2, 0.0);
        assert_eq!(skipped.travel_seconds(), 0.0);
        assert!(skipped.frame(0.0).arrived);
        let clamped = MoveAnimation::new(mv, piece, 99, 0.0);
        assert!(clamped.frame(0.0).arrived);
    }
}
