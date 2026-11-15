//! Shared 64-cell presentation of boards and positions.

use super::{russian, ugolki, Color, GameKind, Kind, Piece};
use std::fmt;

/// Every cell is addressable; individual engines decide which cells are playable.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Square(u8);

impl Square {
    pub const COUNT: usize = 64;
    #[must_use]
    pub const fn from_file_rank(file: u8, rank: u8) -> Option<Self> {
        if file < 8 && rank < 8 {
            Some(Self(rank * 8 + file))
        } else {
            None
        }
    }
    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        if index < 64 {
            Some(Self(index as u8))
        } else {
            None
        }
    }
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
    #[must_use]
    pub const fn file(self) -> u8 {
        self.0 % 8
    }
    #[must_use]
    pub const fn rank(self) -> u8 {
        self.0 / 8
    }
    #[must_use]
    pub const fn bit(self) -> u64 {
        1u64 << self.0
    }
    #[must_use]
    pub fn name(self) -> String {
        format!(
            "{}{}",
            char::from(b'a' + self.file()),
            char::from(b'1' + self.rank())
        )
    }
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 2 {
            return None;
        }
        let file = bytes[0].to_ascii_lowercase();
        if !(b'a'..=b'h').contains(&file) || !(b'1'..=b'8').contains(&bytes[1]) {
            return None;
        }
        Self::from_file_rank(file - b'a', bytes[1] - b'1')
    }
    #[must_use]
    pub const fn draughts(self) -> Option<russian::board::Square> {
        russian::board::Square::from_file_rank(self.file(), self.rank())
    }
    pub fn all() -> impl Iterator<Item = Self> {
        (0..64).map(Self)
    }
}

impl From<russian::board::Square> for Square {
    fn from(square: russian::board::Square) -> Self {
        Self(square.rank() * 8 + square.file())
    }
}
impl fmt::Debug for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name().fmt(f)
    }
}
impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}

pub fn squares_of(mut mask: u64) -> impl Iterator<Item = Square> {
    std::iter::from_fn(move || {
        if mask == 0 {
            None
        } else {
            let square = Square(mask.trailing_zeros() as u8);
            mask &= mask - 1;
            Some(square)
        }
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Position {
    pub white: u64,
    pub black: u64,
    pub kings: u64,
    pub side: Color,
    pub game: GameKind,
}

impl Position {
    #[must_use]
    pub fn initial() -> Self {
        Self::initial_for(GameKind::RussianDraughts)
    }
    #[must_use]
    pub fn initial_for(game: GameKind) -> Self {
        match game {
            GameKind::RussianDraughts => Self::from_russian(russian::board::Position::initial()),
            GameKind::Ugolki => ugolki::initial(),
        }
    }
    #[must_use]
    pub const fn empty(side: Color, game: GameKind) -> Self {
        Self {
            white: 0,
            black: 0,
            kings: 0,
            side,
            game,
        }
    }
    #[must_use]
    pub fn from_russian(pos: russian::board::Position) -> Self {
        let expand =
            |bits| russian::board::squares_of(bits).fold(0, |mask, sq| mask | Square::from(sq).bit());
        Self {
            white: expand(pos.white),
            black: expand(pos.black),
            kings: expand(pos.kings),
            side: pos.side,
            game: GameKind::RussianDraughts,
        }
    }
    #[must_use]
    pub fn draughts(self) -> russian::board::Position {
        let mut pos = russian::board::Position::empty(self.side);
        for color in [Color::White, Color::Black] {
            for cell in squares_of(self.pieces(color)) {
                if let (Some(square), Some(piece)) = (cell.draughts(), self.piece_at(cell)) {
                    pos.put(square, piece);
                }
            }
        }
        pos
    }
    #[must_use]
    pub const fn occupied(self) -> u64 {
        self.white | self.black
    }
    #[must_use]
    pub const fn pieces(self, color: Color) -> u64 {
        match color {
            Color::White => self.white,
            Color::Black => self.black,
        }
    }
    #[must_use]
    pub const fn count(self, color: Color) -> u32 {
        self.pieces(color).count_ones()
    }
    #[must_use]
    pub const fn piece_at(self, square: Square) -> Option<Piece> {
        let color = if self.white & square.bit() != 0 {
            Color::White
        } else if self.black & square.bit() != 0 {
            Color::Black
        } else {
            return None;
        };
        Some(Piece::new(
            color,
            if self.kings & square.bit() != 0 {
                Kind::King
            } else {
                Kind::Man
            },
        ))
    }
    pub fn put(&mut self, square: Square, piece: Piece) {
        self.remove(square);
        match piece.color {
            Color::White => self.white |= square.bit(),
            Color::Black => self.black |= square.bit(),
        }
        if piece.is_king() {
            self.kings |= square.bit();
        }
    }
    pub fn remove(&mut self, square: Square) {
        self.white &= !square.bit();
        self.black &= !square.bit();
        self.kings &= !square.bit();
    }
    #[must_use]
    pub fn hash64(self) -> u64 {
        self.white.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ self.black.rotate_left(23)
            ^ self.kings.rotate_left(41)
            ^ if self.side == Color::White {
                0
            } else {
                0x517C_C1B7_2722_0A95
            }
    }
    /// Parses a Russian draughts position.
    ///
    /// # Errors
    /// Returns an error for malformed or invalid positions.
    pub fn from_fen(text: &str) -> Result<Self, String> {
        Self::from_fen_for(GameKind::RussianDraughts, text)
    }
    /// Parses a position using the selected game's playable cells and pieces.
    ///
    /// # Errors
    /// Returns an error for invalid cells, duplicate pieces or excessive piece counts.
    pub fn from_fen_for(game: GameKind, text: &str) -> Result<Self, String> {
        if game == GameKind::RussianDraughts {
            return russian::board::Position::from_fen(text)
                .map(Self::from_russian)
                .map_err(|e| e.to_string());
        }
        let mut parts = text.split(':');
        let side = match parts.next() {
            Some("W") => Color::White,
            Some("B") => Color::Black,
            _ => return Err(text.into()),
        };
        let mut pos = Self::empty(side, game);
        for part in parts {
            let color = match part.as_bytes().first() {
                Some(b'W') => Color::White,
                Some(b'B') => Color::Black,
                _ => return Err(text.into()),
            };
            for name in part[1..].split(',').filter(|s| !s.is_empty()) {
                let square = Square::parse(name).ok_or_else(|| text.to_string())?;
                if pos.occupied() & square.bit() != 0 {
                    return Err(text.into());
                }
                pos.put(square, Piece::new(color, Kind::Man));
            }
        }
        if pos.count(Color::White) > 12 || pos.count(Color::Black) > 12 {
            return Err(text.into());
        }
        Ok(pos)
    }
    #[must_use]
    pub fn to_fen(self) -> String {
        if self.game == GameKind::RussianDraughts {
            return self.draughts().to_fen();
        }
        format!(
            "{}:W{}:B{}",
            if self.side == Color::White { "W" } else { "B" },
            squares_of(self.white)
                .map(Square::name)
                .collect::<Vec<_>>()
                .join(","),
            squares_of(self.black)
                .map(Square::name)
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}
