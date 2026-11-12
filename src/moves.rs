//! Ходы и генератор ходов русских шашек.
//!
//! Правила, реализованные здесь:
//! * простая ходит на одно поле вперёд, бьёт вперёд и назад;
//! * дамка ходит и бьёт по диагонали на любое расстояние;
//! * взятие обязательно и доводится до конца, выбор между вариантами свободный;
//! * побитые шашки снимаются после завершения хода, до этого они остаются препятствием,
//!   и дважды перескочить одну шашку нельзя (турецкий удар);
//! * после взятия дамка обязана встать на поле, с которого возможно продолжение боя,
//!   если такие поля есть;
//! * простая, достигшая дамочной горизонтали во время взятия, продолжает бить как дамка.

// Направление диагонали — индекс в таблице соседей сразу для нескольких полей,
// поэтому циклы по направлениям записаны через индекс.
#![allow(clippy::needless_range_loop)]

use crate::board::{promotion_mask, Color, Position, Square, NE, NEIGHBOR, NO_SQUARE, NW, SE, SW};
use std::fmt;

/// Наибольшее число приземлений в одном ходе: за ход нельзя побить больше 12 шашек.
pub const MAX_STEPS: usize = 12;

/// Ход: исходное поле и последовательность полей, на которые встаёт шашка.
///
/// У тихого хода одно приземление. У взятия — по одному на каждую побитую шашку,
/// поэтому взятия с разным маршрутом — разные ходы.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    from: Square,
    steps: [Square; MAX_STEPS],
    len: u8,
    captured: u32,
    /// Номер приземления, на котором простая стала дамкой, плюс один; 0 — превращения нет.
    promo: u8,
}

impl Move {
    /// Исходное поле.
    #[inline]
    #[must_use]
    pub const fn from(&self) -> Square {
        self.from
    }

    /// Конечное поле.
    #[inline]
    #[must_use]
    pub const fn to(&self) -> Square {
        self.steps[self.len as usize - 1]
    }

    /// Поля приземления по порядку; последнее — конечное.
    #[inline]
    #[must_use]
    pub fn path(&self) -> &[Square] {
        &self.steps[..self.len as usize]
    }

    /// Все поля маршрута: исходное и приземления.
    pub fn squares(&self) -> impl Iterator<Item = Square> + '_ {
        std::iter::once(self.from).chain(self.path().iter().copied())
    }

    /// Является ли ход взятием.
    #[inline]
    #[must_use]
    pub const fn is_capture(&self) -> bool {
        self.captured != 0
    }

    /// Маска побитых шашек.
    #[inline]
    #[must_use]
    pub const fn captured(&self) -> u32 {
        self.captured
    }

    /// Число побитых шашек.
    #[inline]
    #[must_use]
    pub const fn captured_count(&self) -> u32 {
        self.captured.count_ones()
    }

    /// Становится ли простая дамкой в результате хода.
    #[inline]
    #[must_use]
    pub const fn promotes(&self) -> bool {
        self.promo != 0
    }

    /// Номер приземления (от нуля), на котором простая стала дамкой.
    #[must_use]
    pub const fn promotion_step(&self) -> Option<usize> {
        if self.promo == 0 {
            None
        } else {
            Some(self.promo as usize - 1)
        }
    }

    /// Шашка, побитая на шаге `step` (от нуля). Для тихого хода — `None`.
    #[must_use]
    pub fn captured_at(&self, step: usize) -> Option<Square> {
        if !self.is_capture() || step >= self.len as usize {
            return None;
        }
        let start = if step == 0 {
            self.from
        } else {
            self.steps[step - 1]
        };
        let end = self.steps[step];
        let dir = direction(start, end)?;
        let mut cur = NEIGHBOR[start.index()][dir];
        while cur != NO_SQUARE && cur != end.0 {
            if self.captured & (1 << cur) != 0 {
                return Some(Square(cur));
            }
            cur = NEIGHBOR[cur as usize][dir];
        }
        None
    }

    /// Побитые шашки в порядке взятия.
    #[must_use]
    pub fn captured_in_order(&self) -> Vec<Square> {
        (0..self.len as usize)
            .filter_map(|i| self.captured_at(i))
            .collect()
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sep = if self.is_capture() { ':' } else { '-' };
        write!(f, "{}", self.from)?;
        for sq in self.path() {
            write!(f, "{sep}{sq}")?;
        }
        Ok(())
    }
}

/// Направление диагонали от `a` к `b`, если поля лежат на одной диагонали.
fn direction(a: Square, b: Square) -> Option<usize> {
    let df = i32::from(b.file()) - i32::from(a.file());
    let dr = i32::from(b.rank()) - i32::from(a.rank());
    if df == 0 || df.abs() != dr.abs() {
        return None;
    }
    Some(match (df > 0, dr > 0) {
        (true, true) => NE,
        (false, true) => NW,
        (true, false) => SE,
        (false, false) => SW,
    })
}

#[inline]
const fn bit(index: u8) -> u32 {
    1u32 << index
}

/// Направления тихого хода простой.
#[inline]
const fn forward_dirs(color: Color) -> [usize; 2] {
    match color {
        Color::White => [NE, NW],
        Color::Black => [SE, SW],
    }
}

/// Обход вариантов взятия одной шашкой.
struct CaptureGen<'a> {
    enemy: u32,
    /// Все шашки на доске, кроме бьющей: её исходное поле во время боя свободно,
    /// а уже побитые шашки остаются на месте до конца хода.
    occ: u32,
    promo_mask: u32,
    from: Square,
    steps: [Square; MAX_STEPS],
    len: usize,
    promo: u8,
    out: &'a mut Vec<Move>,
}

impl CaptureGen<'_> {
    fn emit(&mut self, captured: u32) {
        self.out.push(Move {
            from: self.from,
            steps: self.steps,
            len: self.len as u8,
            captured,
            promo: self.promo,
        });
    }

    /// Взятия простой с поля `sq`. Возвращает `true`, если нашлось хотя бы одно.
    fn man(&mut self, sq: u8, captured: u32) -> bool {
        if self.len >= MAX_STEPS {
            return false;
        }
        let mut found = false;
        for dir in 0..4 {
            let over = NEIGHBOR[sq as usize][dir];
            if over == NO_SQUARE {
                continue;
            }
            let over_bit = bit(over);
            if self.enemy & over_bit == 0 || captured & over_bit != 0 {
                continue;
            }
            let land = NEIGHBOR[over as usize][dir];
            if land == NO_SQUARE || self.occ & bit(land) != 0 {
                continue;
            }
            found = true;
            let taken = captured | over_bit;
            self.steps[self.len] = Square(land);
            self.len += 1;
            let continues = if self.promo_mask & bit(land) != 0 {
                // Простая стала дамкой и сразу продолжает бой как дамка.
                self.promo = self.len as u8;
                let more = self.king(land, taken);
                if !more {
                    self.emit(taken);
                }
                self.promo = 0;
                true
            } else {
                self.man(land, taken)
            };
            if !continues {
                self.emit(taken);
            }
            self.len -= 1;
            self.steps[self.len] = Square(0);
        }
        found
    }

    /// Взятия дамки с поля `sq`. Возвращает `true`, если нашлось хотя бы одно.
    fn king(&mut self, sq: u8, captured: u32) -> bool {
        if self.len >= MAX_STEPS {
            return false;
        }
        let mut found = false;
        for dir in 0..4 {
            // Идём по свободным полям до первой шашки.
            let mut cur = NEIGHBOR[sq as usize][dir];
            while cur != NO_SQUARE && self.occ & bit(cur) == 0 {
                cur = NEIGHBOR[cur as usize][dir];
            }
            if cur == NO_SQUARE {
                continue;
            }
            let over_bit = bit(cur);
            // Своя шашка или уже побитая (турецкий удар) — препятствие.
            if self.enemy & over_bit == 0 || captured & over_bit != 0 {
                continue;
            }
            let mut land = NEIGHBOR[cur as usize][dir];
            if land == NO_SQUARE || self.occ & bit(land) != 0 {
                continue;
            }
            found = true;
            let taken = captured | over_bit;
            let mut landings = [0u8; 8];
            let mut count = 0;
            while land != NO_SQUARE && self.occ & bit(land) == 0 {
                landings[count] = land;
                count += 1;
                land = NEIGHBOR[land as usize][dir];
            }
            // Сначала поля, с которых бой продолжается: они обязательны.
            let mut any_continuation = false;
            for &target in &landings[..count] {
                self.steps[self.len] = Square(target);
                self.len += 1;
                if self.king(target, taken) {
                    any_continuation = true;
                }
                self.len -= 1;
                self.steps[self.len] = Square(0);
            }
            // Если продолжения нет ни с одного поля, дамка встаёт на любое из них.
            if !any_continuation {
                for &target in &landings[..count] {
                    self.steps[self.len] = Square(target);
                    self.len += 1;
                    self.emit(taken);
                    self.len -= 1;
                    self.steps[self.len] = Square(0);
                }
            }
        }
        found
    }
}

/// Все взятия стороны, чья очередь хода. Список `out` дополняется.
pub fn generate_captures(pos: &Position, out: &mut Vec<Move>) {
    let own = pos.pieces(pos.side);
    let enemy = pos.pieces(pos.side.opponent());
    let all = own | enemy;
    let mut rest = own;
    while rest != 0 {
        let sq = rest.trailing_zeros() as u8;
        rest &= rest - 1;
        let mut gen = CaptureGen {
            enemy,
            occ: all & !bit(sq),
            promo_mask: promotion_mask(pos.side),
            from: Square(sq),
            steps: [Square(0); MAX_STEPS],
            len: 0,
            promo: 0,
            out,
        };
        if pos.kings & bit(sq) != 0 {
            gen.king(sq, 0);
        } else {
            gen.man(sq, 0);
        }
    }
}

/// Все тихие ходы стороны, чья очередь хода (без учёта обязательности взятия).
/// Список `out` дополняется.
pub fn generate_quiet(pos: &Position, out: &mut Vec<Move>) {
    let own = pos.pieces(pos.side);
    let occ = pos.occupied();
    let promo_mask = promotion_mask(pos.side);
    let forward = forward_dirs(pos.side);
    let mut rest = own;
    while rest != 0 {
        let sq = rest.trailing_zeros() as u8;
        rest &= rest - 1;
        let mut steps = [Square(0); MAX_STEPS];
        if pos.kings & bit(sq) != 0 {
            for dir in 0..4 {
                let mut cur = NEIGHBOR[sq as usize][dir];
                while cur != NO_SQUARE && occ & bit(cur) == 0 {
                    steps[0] = Square(cur);
                    out.push(Move {
                        from: Square(sq),
                        steps,
                        len: 1,
                        captured: 0,
                        promo: 0,
                    });
                    cur = NEIGHBOR[cur as usize][dir];
                }
            }
        } else {
            for dir in forward {
                let target = NEIGHBOR[sq as usize][dir];
                if target == NO_SQUARE || occ & bit(target) != 0 {
                    continue;
                }
                steps[0] = Square(target);
                let promo = u8::from(promo_mask & bit(target) != 0);
                out.push(Move {
                    from: Square(sq),
                    steps,
                    len: 1,
                    captured: 0,
                    promo,
                });
            }
        }
    }
}

/// Все допустимые ходы: если есть взятия, то только они. Список `out` перезаписывается.
pub fn generate_moves(pos: &Position, out: &mut Vec<Move>) {
    out.clear();
    generate_captures(pos, out);
    if out.is_empty() {
        generate_quiet(pos, out);
    }
}

/// Все допустимые ходы в позиции.
#[must_use]
pub fn legal_moves(pos: &Position) -> Vec<Move> {
    let mut out = Vec::with_capacity(16);
    generate_moves(pos, &mut out);
    out
}

/// Может ли сторона `color` что-нибудь побить, если ход её.
#[must_use]
pub fn has_capture(pos: &Position, color: Color) -> bool {
    let own = pos.pieces(color);
    let enemy = pos.pieces(color.opponent());
    let occ = own | enemy;
    let mut rest = own;
    while rest != 0 {
        let sq = rest.trailing_zeros() as u8;
        rest &= rest - 1;
        let is_king = pos.kings & bit(sq) != 0;
        for dir in 0..4 {
            let mut cur = NEIGHBOR[sq as usize][dir];
            if is_king {
                while cur != NO_SQUARE && occ & bit(cur) == 0 {
                    cur = NEIGHBOR[cur as usize][dir];
                }
            }
            if cur == NO_SQUARE || enemy & bit(cur) == 0 {
                continue;
            }
            let land = NEIGHBOR[cur as usize][dir];
            if land != NO_SQUARE && occ & bit(land) == 0 {
                return true;
            }
        }
    }
    false
}

/// Данные для отмены хода.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Undo {
    kings: u32,
}

impl Position {
    /// Выполняет ход на месте. Ход должен быть получен генератором для этой позиции.
    pub const fn make(&mut self, mv: &Move) -> Undo {
        let undo = Undo { kings: self.kings };
        let from_bit = mv.from().bit();
        let to_bit = mv.to().bit();
        let becomes_king = self.kings & from_bit != 0 || mv.promotes();
        self.kings &= !(mv.captured | from_bit);
        match self.side {
            Color::White => {
                self.white = (self.white & !from_bit) | to_bit;
                self.black &= !mv.captured;
            }
            Color::Black => {
                self.black = (self.black & !from_bit) | to_bit;
                self.white &= !mv.captured;
            }
        }
        if becomes_king {
            self.kings |= to_bit;
        }
        self.side = self.side.opponent();
        undo
    }

    /// Отменяет ход, выполненный [`Position::make`].
    pub const fn unmake(&mut self, mv: &Move, undo: Undo) {
        let from_bit = mv.from().bit();
        let to_bit = mv.to().bit();
        self.side = self.side.opponent();
        match self.side {
            Color::White => {
                self.white = (self.white & !to_bit) | from_bit;
                self.black |= mv.captured;
            }
            Color::Black => {
                self.black = (self.black & !to_bit) | from_bit;
                self.white |= mv.captured;
            }
        }
        self.kings = undo.kings;
    }

    /// Позиция после хода.
    #[inline]
    #[must_use]
    pub fn after(&self, mv: &Move) -> Self {
        let mut next = *self;
        next.make(mv);
        next
    }
}

/// Число последовательностей ходов длиной `depth` полуходов из позиции.
#[must_use]
pub fn perft(pos: &Position, depth: u32) -> u64 {
    fn go(pos: &Position, depth: u32, buffers: &mut [Vec<Move>]) -> u64 {
        let Some((head, tail)) = buffers.split_first_mut() else {
            return 1;
        };
        generate_moves(pos, head);
        if depth <= 1 {
            return head.len() as u64;
        }
        let mut total = 0;
        for mv in head.iter() {
            total += go(&pos.after(mv), depth - 1, tail);
        }
        total
    }
    if depth == 0 {
        return 1;
    }
    let mut buffers: Vec<Vec<Move>> = (0..depth).map(|_| Vec::with_capacity(32)).collect();
    go(pos, depth, &mut buffers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(fen: &str) -> Position {
        Position::from_fen(fen).unwrap()
    }

    fn names(pos: &Position) -> Vec<String> {
        let mut v: Vec<String> = legal_moves(pos).iter().map(|m| format!("{m:?}")).collect();
        v.sort();
        v
    }

    #[test]
    fn initial_position_has_seven_moves() {
        assert_eq!(
            names(&Position::initial()),
            ["a3-b4", "c3-b4", "c3-d4", "e3-d4", "e3-f4", "g3-f4", "g3-h4"]
        );
    }

    #[test]
    fn captured_squares_are_reported_in_order() {
        let p = pos("W:Wc3:Bd4,f6");
        let moves = legal_moves(&p);
        assert_eq!(moves.len(), 1);
        let mv = moves[0];
        assert_eq!(format!("{mv:?}"), "c3:e5:g7");
        assert_eq!(mv.captured_at(0), Square::parse("d4"));
        assert_eq!(mv.captured_at(1), Square::parse("f6"));
        assert_eq!(mv.captured_at(2), None);
        assert_eq!(mv.captured_in_order().len(), 2);
        assert_eq!(mv.captured_count(), 2);
    }

    #[test]
    fn make_and_unmake_restore_position() {
        let p = pos("W:Wc3,Ka5:Bd4,f6,Kh8");
        for mv in legal_moves(&p) {
            let mut q = p;
            let undo = q.make(&mv);
            assert_eq!(q, p.after(&mv));
            assert!(q.is_valid());
            q.unmake(&mv, undo);
            assert_eq!(q, p);
        }
    }

    #[test]
    fn has_capture_agrees_with_generator() {
        for fen in [
            "W:Wc3:Bd4",
            "W:Wc3:Bd4,e5",
            "B:Wc3:Bd4",
            "W:WKa1:Bf6",
            "W:WKa1:Bg7,h8",
            "W:WKa1,b2:Bf6",
        ] {
            let p = pos(fen);
            let mut caps = Vec::new();
            generate_captures(&p, &mut caps);
            assert_eq!(has_capture(&p, p.side), !caps.is_empty(), "{fen}");
        }
    }

    #[test]
    fn perft_shallow() {
        let p = Position::initial();
        assert_eq!(perft(&p, 0), 1);
        assert_eq!(perft(&p, 1), 7);
        assert_eq!(perft(&p, 2), 49);
        assert_eq!(perft(&p, 3), 302);
    }
}
