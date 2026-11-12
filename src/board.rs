//! Доска 8×8, поля, фигуры и позиция.
//!
//! Игра идёт на 32 тёмных полях. Поля нумеруются от 0 до 31 слева направо и снизу вверх
//! с точки зрения белых: `a1 = 0`, `c1 = 1`, `e1 = 2`, `g1 = 3`, `b2 = 4`, …, `h8 = 31`.
//! Позиция хранится тремя 32-битными масками: белые, чёрные и дамки.

use std::fmt;

/// Сторона.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Color {
    White,
    Black,
}

impl Color {
    /// Противоположная сторона.
    #[inline]
    #[must_use]
    pub const fn opponent(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

/// Вид фигуры.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    /// Простая шашка.
    Man,
    /// Дамка.
    King,
}

/// Фигура на доске.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Piece {
    pub color: Color,
    pub kind: Kind,
}

impl Piece {
    #[must_use]
    pub const fn new(color: Color, kind: Kind) -> Self {
        Self { color, kind }
    }

    #[must_use]
    pub const fn is_king(self) -> bool {
        matches!(self.kind, Kind::King)
    }
}

/// Направления диагоналей (индексы в таблице соседей).
pub const NE: usize = 0;
pub const NW: usize = 1;
pub const SE: usize = 2;
pub const SW: usize = 3;

/// Признак отсутствия соседнего поля в таблице [`NEIGHBOR`].
pub(crate) const NO_SQUARE: u8 = 255;

const fn build_neighbors() -> [[u8; 4]; 32] {
    let deltas: [(i32, i32); 4] = [(1, 1), (-1, 1), (1, -1), (-1, -1)];
    let mut table = [[NO_SQUARE; 4]; 32];
    let mut i = 0;
    while i < 32 {
        let rank = (i / 4) as i32;
        let file = (2 * (i % 4) + (i / 4) % 2) as i32;
        let mut d = 0;
        while d < 4 {
            let f = file + deltas[d].0;
            let r = rank + deltas[d].1;
            if f >= 0 && f < 8 && r >= 0 && r < 8 {
                table[i][d] = (r * 4 + f / 2) as u8;
            }
            d += 1;
        }
        i += 1;
    }
    table
}

/// Соседнее поле по каждому из четырёх диагональных направлений.
pub(crate) static NEIGHBOR: [[u8; 4]; 32] = build_neighbors();

const fn build_main_road() -> u32 {
    let mut mask = 0u32;
    let mut i = 0;
    while i < 8 {
        // Поле с одинаковыми номерами вертикали и горизонтали: a1, b2, …, h8.
        mask |= 1 << (i * 4 + i / 2);
        i += 1;
    }
    mask
}

/// Большая дорога: диагональ a1–h8.
pub const MAIN_ROAD: u32 = build_main_road();
/// Восьмая горизонталь: здесь белая простая становится дамкой.
pub const RANK_8: u32 = 0xF000_0000;
/// Первая горизонталь: здесь чёрная простая становится дамкой.
pub const RANK_1: u32 = 0x0000_000F;

/// Дамочная горизонталь для простых указанного цвета.
#[inline]
#[must_use]
pub const fn promotion_mask(color: Color) -> u32 {
    match color {
        Color::White => RANK_8,
        Color::Black => RANK_1,
    }
}

/// Тёмное поле доски.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Square(pub(crate) u8);

impl Square {
    /// Число игровых полей.
    pub const COUNT: usize = 32;

    /// Поле по номеру 0..32.
    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        if index < Self::COUNT {
            Some(Self(index as u8))
        } else {
            None
        }
    }

    /// Поле по вертикали и горизонтали (обе от нуля: `a` = 0, первая горизонталь = 0).
    /// Для светлых полей и координат вне доски возвращает `None`.
    #[must_use]
    pub const fn from_file_rank(file: u8, rank: u8) -> Option<Self> {
        if file < 8 && rank < 8 && (file + rank) % 2 == 0 {
            Some(Self(rank * 4 + file / 2))
        } else {
            None
        }
    }

    #[inline]
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Вертикаль от нуля (`a` = 0).
    #[inline]
    #[must_use]
    pub const fn file(self) -> u8 {
        2 * (self.0 % 4) + (self.0 / 4) % 2
    }

    /// Горизонталь от нуля (первая = 0).
    #[inline]
    #[must_use]
    pub const fn rank(self) -> u8 {
        self.0 / 4
    }

    /// Битовая маска поля.
    #[inline]
    #[must_use]
    pub const fn bit(self) -> u32 {
        1u32 << self.0
    }

    /// Соседнее поле по диагонали в направлении `dir` ([`NE`], [`NW`], [`SE`], [`SW`]).
    #[must_use]
    pub fn step(self, dir: usize) -> Option<Self> {
        let next = *NEIGHBOR[self.index()].get(dir)?;
        if next == NO_SQUARE {
            None
        } else {
            Some(Self(next))
        }
    }

    /// Лежит ли поле на большой дороге a1–h8.
    #[inline]
    #[must_use]
    pub const fn on_main_road(self) -> bool {
        MAIN_ROAD & self.bit() != 0
    }

    /// Все 32 поля по возрастанию номера.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..Self::COUNT as u8).map(Square)
    }

    /// Имя поля в шашечной нотации: `c3`.
    #[must_use]
    pub fn name(self) -> String {
        let mut s = String::with_capacity(2);
        s.push((b'a' + self.file()) as char);
        s.push((b'1' + self.rank()) as char);
        s
    }

    /// Разбор имени поля (`c3`, `C3`). Светлые поля не принимаются.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 2 {
            return None;
        }
        let file = bytes[0].to_ascii_lowercase();
        let rank = bytes[1];
        if !(b'a'..=b'h').contains(&file) || !(b'1'..=b'8').contains(&rank) {
            return None;
        }
        Self::from_file_rank(file - b'a', rank - b'1')
    }
}

impl fmt::Debug for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Перечисление полей, отмеченных в маске, по возрастанию номера.
pub fn squares_of(mask: u32) -> impl Iterator<Item = Square> {
    let mut rest = mask;
    std::iter::from_fn(move || {
        if rest == 0 {
            None
        } else {
            let i = rest.trailing_zeros() as u8;
            rest &= rest - 1;
            Some(Square(i))
        }
    })
}

/// Ошибка разбора записи позиции.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FenError {
    /// Не указана или неверно указана очередь хода.
    BadSide,
    /// Раздел не начинается с `W` или `B`.
    BadSection(String),
    /// Неизвестное или светлое поле.
    BadSquare(String),
    /// Поле указано дважды.
    Duplicate(String),
    /// Простая стоит на дамочной горизонтали.
    ManOnPromotionRank(String),
    /// У стороны больше двенадцати шашек.
    TooManyPieces,
}

impl fmt::Display for FenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadSide => write!(f, "side to move is missing"),
            Self::BadSection(s) => write!(f, "bad section '{s}'"),
            Self::BadSquare(s) => write!(f, "bad square '{s}'"),
            Self::Duplicate(s) => write!(f, "square '{s}' is listed twice"),
            Self::ManOnPromotionRank(s) => write!(f, "man on the promotion rank: '{s}'"),
            Self::TooManyPieces => write!(f, "more than 12 pieces for one side"),
        }
    }
}

impl std::error::Error for FenError {}

/// Позиция: расстановка и очередь хода.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// Белые фигуры.
    pub white: u32,
    /// Чёрные фигуры.
    pub black: u32,
    /// Дамки обоих цветов.
    pub kings: u32,
    /// Чья очередь хода.
    pub side: Color,
}

impl Position {
    /// Пустая доска.
    #[must_use]
    pub const fn empty(side: Color) -> Self {
        Self {
            white: 0,
            black: 0,
            kings: 0,
            side,
        }
    }

    /// Начальная расстановка: по 12 простых, ход белых.
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            white: 0x0000_0FFF,
            black: 0xFFF0_0000,
            kings: 0,
            side: Color::White,
        }
    }

    /// Все занятые поля.
    #[inline]
    #[must_use]
    pub const fn occupied(&self) -> u32 {
        self.white | self.black
    }

    /// Фигуры указанного цвета.
    #[inline]
    #[must_use]
    pub const fn pieces(&self, color: Color) -> u32 {
        match color {
            Color::White => self.white,
            Color::Black => self.black,
        }
    }

    /// Простые указанного цвета.
    #[inline]
    #[must_use]
    pub const fn men(&self, color: Color) -> u32 {
        self.pieces(color) & !self.kings
    }

    /// Дамки указанного цвета.
    #[inline]
    #[must_use]
    pub const fn kings_of(&self, color: Color) -> u32 {
        self.pieces(color) & self.kings
    }

    /// Число фигур указанного цвета.
    #[inline]
    #[must_use]
    pub const fn count(&self, color: Color) -> u32 {
        self.pieces(color).count_ones()
    }

    /// Число фигур на доске.
    #[inline]
    #[must_use]
    pub const fn total(&self) -> u32 {
        self.occupied().count_ones()
    }

    /// Фигура на поле.
    #[must_use]
    pub const fn piece_at(&self, sq: Square) -> Option<Piece> {
        let bit = sq.bit();
        let color = if self.white & bit != 0 {
            Color::White
        } else if self.black & bit != 0 {
            Color::Black
        } else {
            return None;
        };
        let kind = if self.kings & bit != 0 {
            Kind::King
        } else {
            Kind::Man
        };
        Some(Piece { color, kind })
    }

    /// Ставит фигуру на поле, заменяя прежнюю.
    pub fn put(&mut self, sq: Square, piece: Piece) {
        self.remove(sq);
        let bit = sq.bit();
        match piece.color {
            Color::White => self.white |= bit,
            Color::Black => self.black |= bit,
        }
        if piece.kind == Kind::King {
            self.kings |= bit;
        }
    }

    /// Убирает фигуру с поля.
    pub const fn remove(&mut self, sq: Square) {
        let mask = !sq.bit();
        self.white &= mask;
        self.black &= mask;
        self.kings &= mask;
    }

    /// Проверка внутренней согласованности: фигуры не накладываются, дамки стоят
    /// на занятых полях, простых нет на дамочной горизонтали, у стороны не более 12 фигур.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.white & self.black == 0
            && self.kings & !self.occupied() == 0
            && self.men(Color::White) & RANK_8 == 0
            && self.men(Color::Black) & RANK_1 == 0
            && self.white.count_ones() <= 12
            && self.black.count_ones() <= 12
    }

    /// 64-битный хеш позиции (с учётом очереди хода).
    #[must_use]
    pub fn hash64(&self) -> u64 {
        #[inline]
        const fn mix(mut x: u64) -> u64 {
            x ^= x >> 33;
            x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
            x ^= x >> 33;
            x = x.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
            x ^= x >> 33;
            x
        }
        let a = u64::from(self.white) | (u64::from(self.black) << 32);
        let side = match self.side {
            Color::White => 0u64,
            Color::Black => 1u64,
        };
        let b = u64::from(self.kings) | (side << 32);
        mix(a ^ mix(b.wrapping_add(0x9E37_79B9_7F4A_7C15)))
    }

    /// Запись позиции в формате FEN из стандарта PDN: `W:Wa1,c1,Ke3:Bb8,Kd6`.
    #[must_use]
    pub fn to_fen(&self) -> String {
        let mut out = String::new();
        out.push(match self.side {
            Color::White => 'W',
            Color::Black => 'B',
        });
        for (letter, color) in [('W', Color::White), ('B', Color::Black)] {
            out.push(':');
            out.push(letter);
            let mut first = true;
            for sq in squares_of(self.pieces(color)) {
                if !first {
                    out.push(',');
                }
                first = false;
                if self.kings & sq.bit() != 0 {
                    out.push('K');
                }
                out.push_str(&sq.name());
            }
        }
        out
    }

    /// Parses a position written in the FEN format of the PDN standard.
    ///
    /// # Errors
    ///
    /// Returns a [`FenError`] if the side to move is missing, a section or a square
    /// cannot be read, a square is listed twice, a man stands on its promotion rank or
    /// a side has more than twelve pieces.
    pub fn from_fen(text: &str) -> Result<Self, FenError> {
        let text = text.trim().trim_end_matches('.');
        let mut parts = text.split(':');
        let side = match parts.next().map(str::trim) {
            Some("W" | "w") => Color::White,
            Some("B" | "b") => Color::Black,
            _ => return Err(FenError::BadSide),
        };
        let mut pos = Self::empty(side);
        for part in parts {
            let part = part.trim().trim_end_matches('.');
            if part.is_empty() {
                continue;
            }
            let mut chars = part.chars();
            let color = match chars.next() {
                Some('W' | 'w') => Color::White,
                Some('B' | 'b') => Color::Black,
                _ => return Err(FenError::BadSection(part.to_string())),
            };
            for item in chars.as_str().split(',') {
                let item = item.trim();
                if item.is_empty() {
                    continue;
                }
                let (kind, name) = match item.strip_prefix('K').or_else(|| item.strip_prefix('k')) {
                    Some(rest) => (Kind::King, rest),
                    None => (Kind::Man, item),
                };
                let sq = Square::parse(name).ok_or_else(|| FenError::BadSquare(item.to_string()))?;
                if pos.occupied() & sq.bit() != 0 {
                    return Err(FenError::Duplicate(item.to_string()));
                }
                if kind == Kind::Man && promotion_mask(color) & sq.bit() != 0 {
                    return Err(FenError::ManOnPromotionRank(item.to_string()));
                }
                pos.put(sq, Piece { color, kind });
            }
        }
        if pos.white.count_ones() > 12 || pos.black.count_ones() > 12 {
            return Err(FenError::TooManyPieces);
        }
        Ok(pos)
    }
}

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Position({})", self.to_fen())
    }
}

impl fmt::Display for Position {
    /// Текстовая диаграмма: `w`/`b` — простые, `W`/`B` — дамки, `.` — пустое тёмное поле.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8u8).rev() {
            write!(f, "{} ", rank + 1)?;
            for file in 0..8u8 {
                let ch = match Square::from_file_rank(file, rank) {
                    None => ' ',
                    Some(sq) => match self.piece_at(sq) {
                        None => '.',
                        Some(Piece {
                            color: Color::White,
                            kind: Kind::Man,
                        }) => 'w',
                        Some(Piece {
                            color: Color::White,
                            kind: Kind::King,
                        }) => 'W',
                        Some(Piece {
                            color: Color::Black,
                            kind: Kind::Man,
                        }) => 'b',
                        Some(Piece {
                            color: Color::Black,
                            kind: Kind::King,
                        }) => 'B',
                    },
                };
                write!(f, "{ch}")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "  abcdefgh")?;
        write!(
            f,
            "{} to move",
            match self.side {
                Color::White => "white",
                Color::Black => "black",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_numbering_matches_notation() {
        assert_eq!(Square::parse("a1"), Square::from_index(0));
        assert_eq!(Square::parse("g1"), Square::from_index(3));
        assert_eq!(Square::parse("b2"), Square::from_index(4));
        assert_eq!(Square::parse("h8"), Square::from_index(31));
        assert_eq!(Square::parse("b1"), None, "светлое поле");
        assert_eq!(Square::parse("i1"), None);
        assert_eq!(Square::parse("a9"), None);
        assert_eq!(Square::parse(""), None);
        assert_eq!(Square::parse("ц3"), None);
        for sq in Square::all() {
            assert_eq!(Square::parse(&sq.name()), Some(sq));
            assert_eq!(Square::from_file_rank(sq.file(), sq.rank()), Some(sq));
            assert_eq!((sq.file() + sq.rank()) % 2, 0, "все игровые поля тёмные");
        }
    }

    #[test]
    fn a1_is_a_dark_corner_and_main_road_is_a1_h8() {
        assert!(Square::from_file_rank(0, 0).is_some());
        assert!(Square::from_file_rank(7, 0).is_none());
        let road: Vec<String> = squares_of(MAIN_ROAD).map(super::Square::name).collect();
        assert_eq!(road, ["a1", "b2", "c3", "d4", "e5", "f6", "g7", "h8"]);
    }

    #[test]
    fn neighbors_are_consistent() {
        let c3 = Square::parse("c3").unwrap();
        assert_eq!(c3.step(NE), Square::parse("d4"));
        assert_eq!(c3.step(NW), Square::parse("b4"));
        assert_eq!(c3.step(SE), Square::parse("d2"));
        assert_eq!(c3.step(SW), Square::parse("b2"));
        assert_eq!(Square::parse("a1").unwrap().step(SW), None);
        assert_eq!(Square::parse("h8").unwrap().step(NE), None);
        assert_eq!(c3.step(7), None);
        for sq in Square::all() {
            for (dir, back) in [(NE, SW), (NW, SE), (SE, NW), (SW, NE)] {
                if let Some(next) = sq.step(dir) {
                    assert_eq!(next.step(back), Some(sq));
                }
            }
        }
    }

    #[test]
    fn initial_position() {
        let pos = Position::initial();
        assert!(pos.is_valid());
        assert_eq!(pos.count(Color::White), 12);
        assert_eq!(pos.count(Color::Black), 12);
        assert_eq!(pos.kings, 0);
        assert_eq!(pos.side, Color::White);
        assert_eq!(
            pos.to_fen(),
            "W:Wa1,c1,e1,g1,b2,d2,f2,h2,a3,c3,e3,g3:Bb6,d6,f6,h6,a7,c7,e7,g7,b8,d8,f8,h8"
        );
    }

    #[test]
    fn fen_round_trip_and_errors() {
        let pos = Position::from_fen("B:WKa1,c3:Bh8,Kf6").unwrap();
        assert_eq!(pos.side, Color::Black);
        assert_eq!(
            pos.piece_at(Square::parse("a1").unwrap()),
            Some(Piece::new(Color::White, Kind::King))
        );
        assert_eq!(
            pos.piece_at(Square::parse("h8").unwrap()),
            Some(Piece::new(Color::Black, Kind::Man))
        );
        assert_eq!(Position::from_fen(&pos.to_fen()), Ok(pos));
        assert_eq!(
            Position::from_fen(&Position::initial().to_fen()),
            Ok(Position::initial())
        );

        assert_eq!(Position::from_fen(""), Err(FenError::BadSide));
        assert_eq!(Position::from_fen("X:Wa1:Bh8"), Err(FenError::BadSide));
        assert!(matches!(
            Position::from_fen("W:Wb1:Bh8"),
            Err(FenError::BadSquare(_))
        ));
        assert!(matches!(
            Position::from_fen("W:Wa1,a1:Bh8"),
            Err(FenError::Duplicate(_))
        ));
        assert!(matches!(
            Position::from_fen("W:Wa1:Ba1"),
            Err(FenError::Duplicate(_))
        ));
        assert!(matches!(
            Position::from_fen("W:Wb8:Bh8"),
            Err(FenError::ManOnPromotionRank(_))
        ));
        assert!(matches!(
            Position::from_fen("W:Wa1:Bc1"),
            Err(FenError::ManOnPromotionRank(_))
        ));
        assert!(matches!(
            Position::from_fen("W:Za1"),
            Err(FenError::BadSection(_))
        ));
        assert!(Position::from_fen("W:WKb8:BKc1").is_ok());
    }

    #[test]
    fn hash_distinguishes_side_and_kings() {
        let a = Position::from_fen("W:Wc3:Bf6").unwrap();
        let mut b = a;
        b.side = Color::Black;
        let c = Position::from_fen("W:WKc3:Bf6").unwrap();
        assert_ne!(a.hash64(), b.hash64());
        assert_ne!(a.hash64(), c.hash64());
        assert_eq!(a.hash64(), Position::from_fen("W:Wc3:Bf6").unwrap().hash64());
    }
}
