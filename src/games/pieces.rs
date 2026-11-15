//! Shared colours and the visual representation of a piece.

/// A side.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Color {
    White,
    Black,
}

impl Color {
    /// The opposite side.
    #[inline]
    #[must_use]
    pub const fn opponent(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

/// Kind of a piece.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    /// A man.
    Man,
    /// A king.
    King,
}

/// A piece on the board.
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
