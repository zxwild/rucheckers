//! A full move route for input, animation and game records.

use super::{russian, Square};

/// The shared presentation keeps the full route; engines retain their own rules.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Move {
    from: Square,
    steps: [Square; 64],
    len: u8,
    captured: u64,
    promotion: Option<usize>,
    pub(crate) russian: Option<russian::moves::Move>,
}
impl Move {
    /// Constructs a non-capturing move.
    ///
    /// # Panics
    /// Panics if the path is empty or contains more than 64 cells.
    #[must_use]
    pub fn route(from: Square, path: &[Square]) -> Self {
        assert!(!path.is_empty() && path.len() <= 64);
        let mut steps = [from; 64];
        steps[..path.len()].copy_from_slice(path);
        Self {
            from,
            steps,
            len: path.len() as u8,
            captured: 0,
            promotion: None,
            russian: None,
        }
    }
    #[must_use]
    pub fn from_russian(mv: russian::moves::Move) -> Self {
        let mut out = Self::route(
            mv.from().into(),
            &mv.path().iter().copied().map(Square::from).collect::<Vec<_>>(),
        );
        out.captured =
            russian::board::squares_of(mv.captured()).fold(0, |mask, sq| mask | Square::from(sq).bit());
        out.promotion = mv.promotion_step();
        out.russian = Some(mv);
        out
    }
    #[must_use]
    pub const fn from(&self) -> Square {
        self.from
    }
    #[must_use]
    pub const fn to(&self) -> Square {
        self.steps[self.len as usize - 1]
    }
    #[must_use]
    pub fn path(&self) -> &[Square] {
        &self.steps[..self.len as usize]
    }
    pub fn squares(&self) -> impl Iterator<Item = Square> + '_ {
        std::iter::once(self.from).chain(self.path().iter().copied())
    }
    #[must_use]
    pub const fn is_capture(&self) -> bool {
        self.captured != 0
    }
    #[must_use]
    pub const fn captured(&self) -> u64 {
        self.captured
    }
    #[must_use]
    pub const fn captured_count(&self) -> u32 {
        self.captured.count_ones()
    }
    #[must_use]
    pub const fn promotion_step(&self) -> Option<usize> {
        self.promotion
    }
    #[must_use]
    pub const fn promotes(&self) -> bool {
        self.promotion.is_some()
    }
    #[must_use]
    pub fn captured_at(&self, step: usize) -> Option<Square> {
        self.russian.and_then(|mv| mv.captured_at(step)).map(Square::from)
    }
}
