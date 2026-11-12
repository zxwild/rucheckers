//! Расчёт анимации хода: положение шашки во времени, момент превращения,
//! исчезновение побитых шашек. Модуль не рисует — он только считает кадр.
//!
//! Обычное взятие: шашка, через которую перепрыгнули, бледнеет и исчезает после
//! завершения хода. Если за ход побито несколько шашек и включены спецэффекты, ход
//! сопровождается электрическим разрядом ([`super::fx`]): молния тянется за летящей
//! шашкой, а побитые шашки исчезают в тот момент, когда она над ними пролетает.

use super::fx::{self, Discharge, Strike};
use super::geometry::screen_cell;
use crate::board::{Piece, Square};
use crate::moves::Move;

/// Длительность перемещения на соседнее поле, с.
pub const STEP_SECONDS: f64 = 0.17;
/// Добавка за каждое следующее поле длинного хода дамки, с.
pub const EXTRA_SQUARE_SECONDS: f64 = 0.04;
/// Время исчезновения побитой шашки после завершения хода (без разряда), с.
pub const FADE_SECONDS: f64 = 0.20;
/// Непрозрачность побитой шашки, пока ход не закончен и она ещё стоит на доске.
pub const CAPTURED_ALPHA: f32 = 0.42;

/// Длительность одного отрезка пути между полями `a` и `b`.
#[must_use]
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
    /// Маска побитых шашек, над которыми шашка уже пролетела.
    pub jumped: u32,
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
    /// Сопровождается ли ход электрическим разрядом.
    discharge: bool,
}

impl MoveAnimation {
    /// Анимация хода `mv` шашкой `piece` (какой она была до хода).
    ///
    /// `first_step` — сколько прыжков уже показано (игрок сделал их по шагам);
    /// анимируются остальные. `start` — время начала. Взятие нескольких шашек
    /// сопровождается разрядом; отключить его можно через [`Self::with_discharge`].
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

    /// Включает или отключает разряд (он возможен только при взятии нескольких шашек).
    #[must_use]
    pub const fn with_discharge(mut self, enabled: bool) -> Self {
        self.discharge = enabled && self.mv.captured_count() >= fx::MIN_CAPTURES;
        self
    }

    /// Анимируемый ход.
    #[must_use]
    pub const fn mv(&self) -> &Move {
        &self.mv
    }

    /// Шашка до хода.
    #[must_use]
    pub const fn piece(&self) -> Piece {
        self.piece
    }

    /// Время начала.
    #[must_use]
    pub const fn start(&self) -> f64 {
        self.start
    }

    /// Длительность движения шашки.
    #[must_use]
    pub fn travel_seconds(&self) -> f64 {
        self.ends.last().copied().unwrap_or(0.0)
    }

    /// Сопровождается ли ход электрическим разрядом.
    #[must_use]
    pub const fn has_discharge(&self) -> bool {
        self.discharge
    }

    /// Когда (от начала анимации) шашка находится на поле маршрута с номером `index`
    /// (0 — исходное поле). Поля уже показанных прыжков пройдены к началу.
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

    /// Когда (от начала анимации) шашка пролетает над шашкой, побитой прыжком `step`.
    /// Для уже показанных прыжков — в самом начале.
    fn pass_seconds(&self, step: usize) -> f64 {
        let (begin, end) = (self.knot_seconds(step), self.knot_seconds(step + 1));
        if end <= begin {
            return begin;
        }
        let points: Vec<Square> = self.mv.squares().collect();
        let (Some(&a), Some(&b)) = (points.get(step), points.get(step + 1)) else {
            return end;
        };
        // Побитая шашка не обязательно посередине прыжка: дамка бьёт издалека.
        let length = (i32::from(b.file()) - i32::from(a.file())).abs().max(1);
        let along = match self.mv.captured_at(step) {
            Some(victim) => {
                f64::from((i32::from(victim.file()) - i32::from(a.file())).abs()) / f64::from(length)
            }
            None => 0.5,
        };
        begin + (end - begin) * fx::ease_inverse(along)
    }

    /// Момент первого удара разряда (в абсолютном времени), если разряд есть.
    #[must_use]
    pub fn first_strike_time(&self) -> Option<f64> {
        self.discharge.then(|| self.start + self.pass_seconds(0))
    }

    /// Непрозрачность шашки на поле `sq` в момент `now`.
    ///
    /// Для шашек, которые этим ходом не бьются, — всегда 1. Побитая шашка без разряда
    /// бледнеет, когда через неё перепрыгнули, и исчезает после завершения хода.
    /// При разряде она исчезает сразу, как только над ней пролетела бьющая шашка.
    #[must_use]
    pub fn captured_alpha(&self, sq: Square, now: f64) -> f32 {
        let Some(step) = (0..self.mv.path().len()).find(|&i| self.mv.captured_at(i) == Some(sq)) else {
            return 1.0;
        };
        let t = now - self.start;
        let passed = self.pass_seconds(step);
        // Шашки уже показанных прыжков стояли на доске побледневшими.
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

    /// Полная длительность. Ход с разрядом считается законченным, когда погасла
    /// молния (искры ещё летят, но следующему ходу не мешают); обычное взятие —
    /// когда исчезли побитые шашки.
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

    /// Моменты (в абсолютном времени), когда шашка встаёт на очередное поле.
    #[must_use]
    pub fn landing_times(&self) -> Vec<f64> {
        self.ends.iter().map(|end| self.start + end).collect()
    }

    /// Разряд, сопровождающий ход: путь шашки с моментами времени и удары по побитым
    /// шашкам в экранных клетках. `flipped` — доска перевёрнута (внизу чёрные).
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

    /// Кадр в момент `now`.
    #[must_use]
    pub fn frame(&self, now: f64) -> AnimFrame {
        let t = now - self.start;
        let points: Vec<Square> = self.mv.squares().collect();
        let promotion = self.mv.promotion_step();
        let mut jumped = 0u32;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Color, Kind, Position};
    use crate::moves::legal_moves;
    use crate::notation::format_move;
    use crate::ui::fx::EffectSet;

    fn only_move(fen: &str) -> (Move, Piece) {
        let pos = Position::from_fen(fen).unwrap();
        let moves = legal_moves(&pos);
        assert_eq!(moves.len(), 1, "{moves:?}");
        (moves[0], pos.piece_at(moves[0].from()).unwrap())
    }

    fn named_move(fen: &str, name: &str) -> (Move, Piece) {
        let pos = Position::from_fen(fen).unwrap();
        let mv = legal_moves(&pos)
            .into_iter()
            .find(|m| format_move(m) == name)
            .expect(name);
        (mv, pos.piece_at(mv.from()).unwrap())
    }

    fn square(name: &str) -> Square {
        Square::parse(name).unwrap()
    }

    const HOP: f64 = STEP_SECONDS + EXTRA_SQUARE_SECONDS;

    #[test]
    fn quiet_move_goes_from_start_to_end() {
        let (mv, piece) = only_move("W:Wa1:Bh8,g7,f6");
        let anim = MoveAnimation::new(mv, piece, 0, 10.0);
        assert!((anim.travel_seconds() - STEP_SECONDS).abs() < 1e-9);
        assert!((anim.total_seconds() - STEP_SECONDS).abs() < 1e-9);
        assert!(!anim.has_discharge() && anim.discharge(false, 1).is_none());
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
    fn single_capture_dims_the_piece_and_removes_it_after_landing() {
        let (mv, piece) = only_move("W:Wc3,a1:Bd4");
        let anim = MoveAnimation::new(mv, piece, 0, 0.0);
        let d4 = square("d4");
        assert!(!anim.has_discharge(), "одна побитая шашка — без разряда");
        assert_eq!(anim.first_strike_time(), None);
        let travel = anim.travel_seconds();
        assert!((travel - HOP).abs() < 1e-9);
        assert!((anim.total_seconds() - (travel + FADE_SECONDS)).abs() < 1e-9);
        // До прыжка шашка цела, после — бледнеет и стоит до конца хода.
        assert_eq!(anim.frame(travel * 0.4).jumped, 0);
        assert_eq!(anim.captured_alpha(d4, travel * 0.4), 1.0);
        assert_eq!(anim.frame(travel * 0.6).jumped, d4.bit());
        assert_eq!(anim.captured_alpha(d4, travel * 0.6), CAPTURED_ALPHA);
        let landed = anim.frame(travel + 1e-6);
        assert!(landed.arrived && !landed.finished);
        assert!((anim.captured_alpha(d4, travel + 1e-6) - CAPTURED_ALPHA).abs() < 1e-3);
        let half = anim.captured_alpha(d4, travel + FADE_SECONDS / 2.0);
        assert!((half - CAPTURED_ALPHA / 2.0).abs() < 0.01);
        assert_eq!(anim.captured_alpha(d4, travel + FADE_SECONDS + 1e-6), 0.0);
        assert!(anim.frame(travel + FADE_SECONDS + 1e-6).finished);
        // Остальные поля от анимации не зависят.
        assert_eq!(anim.captured_alpha(square("a1"), 100.0), 1.0);
    }

    #[test]
    fn multi_capture_strikes_each_piece_as_the_piece_flies_over_it() {
        let (mv, piece) = only_move("W:Wa1:Bb2,d4,f6");
        let anim = MoveAnimation::new(mv, piece, 0, 5.0);
        let victims = [square("b2"), square("d4"), square("f6")];
        assert!(anim.has_discharge());
        let travel = anim.travel_seconds();
        assert!((travel - 3.0 * HOP).abs() < 1e-9);
        // Простая перепрыгивает шашку посередине прыжка.
        assert!((anim.first_strike_time().unwrap() - (5.0 + 0.5 * HOP)).abs() < 1e-9);
        for (index, sq) in victims.into_iter().enumerate() {
            let strike = 5.0 + (index as f64 + 0.5) * HOP;
            assert_eq!(
                anim.captured_alpha(sq, strike - 1e-6),
                1.0,
                "{sq}: до пролёта шашка цела"
            );
            assert_eq!(anim.frame(strike - 1e-6).jumped & sq.bit(), 0);
            assert_ne!(anim.frame(strike + 1e-6).jumped & sq.bit(), 0);
            let half = anim.captured_alpha(sq, strike + fx::VANISH_SECONDS / 2.0);
            assert!((half - 0.5).abs() < 0.01, "{sq}: {half}");
            assert_eq!(
                anim.captured_alpha(sq, strike + fx::VANISH_SECONDS + 1e-6),
                0.0,
                "{sq}"
            );
            // В этот момент бьющая шашка находится точно над побитой.
            let over = anim.frame(strike);
            assert!((over.file - f32::from(sq.file())).abs() < 1e-3);
            assert!((over.rank - f32::from(sq.rank())).abs() < 1e-3);
        }
        // Первая шашка исчезла задолго до конца хода.
        assert_eq!(anim.captured_alpha(victims[0], 5.0 + 1.5 * HOP), 0.0);
        // Ход закончен, когда погасла молния.
        let total = anim.total_seconds();
        assert!((total - (travel + fx::TRAIL_SECONDS)).abs() < 1e-9);
        assert!(!anim.frame(5.0 + total - 0.01).finished);
        assert!(anim.frame(5.0 + total + 1e-6).finished);
    }

    #[test]
    fn discharge_follows_the_piece_exactly() {
        for flipped in [false, true] {
            let (mv, piece) = only_move("W:Wa1:Bb2,d4,f6");
            let anim = MoveAnimation::new(mv, piece, 0, 5.0);
            let fx = anim.discharge(flipped, 7).unwrap();
            assert_eq!(fx.start(), 5.0);
            assert!((fx.arrival() - (5.0 + anim.travel_seconds())).abs() < 1e-9);
            assert!(
                (fx.bolt_end() - (5.0 + anim.total_seconds())).abs() < 1e-9,
                "ход кончается вместе с молнией"
            );
            // Голова молнии совпадает с положением шашки в каждый момент.
            for step in 0..=120 {
                let now = 4.9 + f64::from(step) * 0.007;
                let frame = anim.frame(now);
                let expected = screen_cell(frame.file, frame.rank, flipped);
                let head = fx.head(now);
                assert!(
                    (head.0 - expected.0).abs() < 1e-4 && (head.1 - expected.1).abs() < 1e-4,
                    "t = {now}"
                );
            }
            // Удары — по побитым шашкам, в порядке взятия, в моменты пролёта.
            let strikes = fx.strikes();
            assert_eq!(strikes.len(), 3);
            for (index, name) in ["b2", "d4", "f6"].into_iter().enumerate() {
                let sq = square(name);
                assert_eq!(
                    strikes[index].center,
                    screen_cell(f32::from(sq.file()), f32::from(sq.rank()), flipped)
                );
                assert!((strikes[index].time - (5.0 + (index as f64 + 0.5) * HOP)).abs() < 1e-9);
                let head = fx.head(strikes[index].time);
                assert!((head.0 - strikes[index].center.0).abs() < 1e-3);
                let direction = strikes[index].direction;
                assert!((direction.0.hypot(direction.1) - 1.0).abs() < 1e-4);
            }
            // Шашка летит вправо-вверх; на перевёрнутой доске — влево-вниз.
            let direction = strikes[0].direction;
            assert_eq!((direction.0 > 0.0, direction.1 < 0.0), (!flipped, !flipped));
            // Молния видна уже во время движения.
            assert!(!fx.frame(5.0 + 0.3 * HOP, EffectSet::ALL).bolts.is_empty());
        }
    }

    #[test]
    fn king_strikes_a_distant_piece_when_it_is_over_it() {
        // Дамка a1 бьёт c3 (встаёт на e5) и затем g7... маршрут a1:e5:h8 — оба прыжка длинные,
        // побитые шашки стоят не посередине прыжка.
        let (mv, piece) = named_move("W:WKa1:Bc3,g7", "a1:e5:h8");
        let anim = MoveAnimation::new(mv, piece, 0, 0.0);
        let fx = anim.discharge(false, 1).unwrap();
        for (strike, name) in fx.strikes().iter().zip(["c3", "g7"]) {
            let sq = square(name);
            let over = anim.frame(strike.time);
            assert!(
                (over.file - f32::from(sq.file())).abs() < 2e-3,
                "{name}: {}",
                over.file
            );
            assert!((over.rank - f32::from(sq.rank())).abs() < 2e-3, "{name}");
            assert_eq!(anim.captured_alpha(sq, strike.time - 1e-6), 1.0);
            assert!(anim.captured_alpha(sq, strike.time + 0.02) < 1.0);
        }
        // Первый прыжок: шашка c3 на половине пути (a1 → e5), второй: g7 на двух третях (e5 → h8).
        let first_hop = segment_seconds(square("a1"), square("e5"));
        assert!((fx.strikes()[0].time - first_hop * 0.5).abs() < 1e-6);
        let second_hop = segment_seconds(square("e5"), square("h8"));
        let expected = first_hop + second_hop * fx::ease_inverse(2.0 / 3.0);
        assert!((fx.strikes()[1].time - expected).abs() < 1e-6);
        assert!(fx.strikes()[1].time > first_hop + second_hop * 0.5);
    }

    #[test]
    fn hops_made_by_hand_are_struck_at_once() {
        // Игрок сделал первый прыжок сам: шашка d4 уже стоит побледневшей.
        let (mv, piece) = only_move("W:Wc3:Bd4,f6");
        let anim = MoveAnimation::new(mv, piece, 1, 0.0);
        let (d4, f6) = (square("d4"), square("f6"));
        assert!(
            anim.has_discharge(),
            "разряд не зависит от того, сколько шагов уже показано"
        );
        let start = anim.frame(0.0);
        assert_eq!((start.file, start.rank), (4.0, 4.0), "анимация начинается с e5");
        assert_eq!(start.jumped, d4.bit(), "первая шашка уже побита");
        assert_eq!(anim.landing_times().len(), 1);
        assert_eq!(anim.first_strike_time(), Some(0.0));
        assert_eq!(anim.captured_alpha(d4, -0.1), CAPTURED_ALPHA);
        assert!(anim.captured_alpha(d4, fx::VANISH_SECONDS / 2.0) < CAPTURED_ALPHA);
        assert_eq!(anim.captured_alpha(d4, fx::VANISH_SECONDS + 1e-6), 0.0);
        assert_eq!(anim.captured_alpha(f6, 0.4 * HOP), 1.0);
        assert_eq!(
            anim.captured_alpha(f6, 0.5 * HOP + fx::VANISH_SECONDS + 1e-6),
            0.0
        );
        // Молния сразу вспыхивает на пройденном отрезке c3–e5.
        let fx = anim.discharge(false, 3).unwrap();
        assert_eq!(fx.head(0.0), screen_cell(4.0, 4.0, false));
        let frame = fx.frame(0.01, EffectSet::ALL);
        let c3 = screen_cell(2.0, 2.0, false);
        assert!(
            frame.bolts.iter().any(|b| b.points[0] == c3),
            "молния начинается от исходного поля"
        );

        // Все прыжки показаны (шашку перетащили): движения нет, всё вспыхивает разом.
        let dropped = MoveAnimation::new(mv, piece, 2, 0.0);
        assert_eq!(dropped.travel_seconds(), 0.0);
        assert!(dropped.frame(0.0).arrived);
        assert!((dropped.total_seconds() - fx::TRAIL_SECONDS).abs() < 1e-9);
        assert_eq!(dropped.captured_alpha(f6, -0.01), CAPTURED_ALPHA);
        assert_eq!(dropped.captured_alpha(f6, fx::VANISH_SECONDS + 1e-6), 0.0);
        let clamped = MoveAnimation::new(mv, piece, 99, 0.0);
        assert!(clamped.frame(0.0).arrived);
        assert!(clamped.discharge(true, 1).is_some());
    }

    #[test]
    fn discharge_can_be_switched_off() {
        // Без спецэффектов взятие нескольких шашек идёт как обычное: шашки бледнеют
        // и исчезают после завершения хода.
        let (mv, piece) = only_move("W:Wa1:Bb2,d4,f6");
        let anim = MoveAnimation::new(mv, piece, 0, 0.0).with_discharge(false);
        assert!(!anim.has_discharge());
        assert!(anim.discharge(false, 1).is_none());
        assert_eq!(anim.first_strike_time(), None);
        let travel = anim.travel_seconds();
        assert!((anim.total_seconds() - (travel + FADE_SECONDS)).abs() < 1e-9);
        let b2 = square("b2");
        assert_eq!(anim.captured_alpha(b2, 0.4 * HOP), 1.0);
        assert_eq!(
            anim.captured_alpha(b2, 1.5 * HOP),
            CAPTURED_ALPHA,
            "стоит до конца хода"
        );
        assert_eq!(anim.captured_alpha(b2, travel + FADE_SECONDS + 1e-6), 0.0);
        // Включить разряд у одиночного взятия нельзя.
        let (single, man) = only_move("W:Wc3,a1:Bd4");
        assert!(!MoveAnimation::new(single, man, 0, 0.0)
            .with_discharge(true)
            .has_discharge());
        assert!(MoveAnimation::new(mv, piece, 0, 0.0)
            .with_discharge(true)
            .has_discharge());
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
}
