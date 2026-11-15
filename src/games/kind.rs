//! Persisted game identifiers and names.

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum GameKind {
    #[default]
    RussianDraughts,
    Ugolki,
}

impl GameKind {
    pub const ALL: [Self; 2] = [Self::RussianDraughts, Self::Ugolki];

    #[must_use]
    pub const fn text_key(self) -> crate::i18n::Text {
        match self {
            Self::RussianDraughts => crate::i18n::Text::AppTitle,
            Self::Ugolki => crate::i18n::Text::Ugolki,
        }
    }

    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RussianDraughts => "russian",
            Self::Ugolki => "ugolki",
        }
    }

    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_ascii_lowercase().as_str() {
            "russian" => Some(Self::RussianDraughts),
            "ugolki" => Some(Self::Ugolki),
            _ => None,
        }
    }
}
