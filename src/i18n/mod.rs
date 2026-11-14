//! Interface texts in twelve languages.
//!
//! Every string the player can see lives here, so the language switches without a
//! restart. Each language is a module with one exhaustive `match` over [`Text`]: a
//! missing translation is a compile error, and tests check the rest (placeholders,
//! scripts, distinct messages).
//!
//! Some texts are templates with named placeholders such as `{line}`; they are filled
//! in by the helper functions of this module, never by the caller.

mod chinese;
mod english;
mod french;
mod german;
mod indonesian;
mod italian;
mod japanese;
mod korean;
mod portuguese;
mod russian;
mod spanish;
mod turkish;

use crate::board::Color;
use crate::pdn::{MoveProblem, PdnError};
use crate::rules::{
    DrawReason, Outcome, WinReason, KINGS_ONLY_MOVES, KINGS_VS_KING_MOVES, MAIN_ROAD_MOVES, REPETITION_LIMIT,
};
use std::fmt::Display;

/// Interface language.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Lang {
    /// Indonesian.
    Id,
    /// German.
    De,
    /// English.
    En,
    /// Spanish.
    Es,
    /// French.
    Fr,
    /// Italian.
    It,
    /// Portuguese (Brazilian usage).
    Pt,
    /// Turkish.
    Tr,
    /// Russian.
    Ru,
    /// Chinese (Simplified).
    Zh,
    /// Japanese.
    Ja,
    /// Korean.
    Ko,
}

impl Lang {
    /// All languages in the order of the language list: by native name, Latin scripts
    /// first.
    pub const ALL: [Self; 12] = [
        Self::Id,
        Self::De,
        Self::En,
        Self::Es,
        Self::Fr,
        Self::It,
        Self::Pt,
        Self::Tr,
        Self::Ru,
        Self::Zh,
        Self::Ja,
        Self::Ko,
    ];

    /// The language of a fresh installation when the system locale says nothing.
    pub const DEFAULT: Self = Self::Ru;

    /// The language used for a system locale that has no translation.
    pub const FALLBACK: Self = Self::En;

    /// Language code for the settings file (ISO 639-1).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::De => "de",
            Self::En => "en",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::It => "it",
            Self::Pt => "pt",
            Self::Tr => "tr",
            Self::Ru => "ru",
            Self::Zh => "zh",
            Self::Ja => "ja",
            Self::Ko => "ko",
        }
    }

    /// The language with the given code; an unknown code gives `None`.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        let code = code.trim();
        Self::ALL
            .into_iter()
            .find(|lang| lang.code().eq_ignore_ascii_case(code))
    }

    /// The name of the language in the language itself.
    #[must_use]
    pub const fn native_name(self) -> &'static str {
        match self {
            Self::Id => "Bahasa Indonesia",
            Self::De => "Deutsch",
            Self::En => "English",
            Self::Es => "Español",
            Self::Fr => "Français",
            Self::It => "Italiano",
            Self::Pt => "Português",
            Self::Tr => "Türkçe",
            Self::Ru => "Русский",
            Self::Zh => "简体中文",
            Self::Ja => "日本語",
            Self::Ko => "한국어",
        }
    }

    /// The translation for a locale name: `de_DE.UTF-8` as the environment variables of
    /// Unix spell it, or `pt-BR` and `zh-Hans-CN` as Windows and macOS do. `None` if the
    /// interface is not translated into the language of the locale.
    #[must_use]
    pub fn of_locale(locale: &str) -> Option<Self> {
        // The language subtag ends at the script, territory, encoding or modifier.
        let language = locale.trim().split(['_', '-', '.', '@']).next()?;
        // `in` is the code Indonesian had before ISO 639 renamed it to `id`.
        if language.eq_ignore_ascii_case("in") {
            return Some(Self::Id);
        }
        Self::from_code(language)
    }

    /// Whether a locale name asks for no particular language: an empty one, `C` or
    /// `POSIX`, with or without an encoding (`C.UTF-8`).
    #[must_use]
    pub fn is_neutral_locale(locale: &str) -> bool {
        let name = locale.trim().split(['.', '@']).next().unwrap_or_default();
        name.is_empty() || name.eq_ignore_ascii_case("C") || name.eq_ignore_ascii_case("POSIX")
    }

    /// The language of the first start for the locale of the system.
    ///
    /// An unknown or neutral locale selects [`Lang::DEFAULT`]; a locale of a language
    /// without a translation selects [`Lang::FALLBACK`].
    #[must_use]
    pub fn from_locale(locale: Option<&str>) -> Self {
        match locale {
            Some(locale) if !Self::is_neutral_locale(locale) => {
                Self::of_locale(locale).unwrap_or(Self::FALLBACK)
            }
            _ => Self::DEFAULT,
        }
    }
}

macro_rules! text_keys {
    ($( $(#[$doc:meta])* $key:ident, )+) => {
        /// Key of an interface string.
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum Text {
            $( $(#[$doc])* $key, )+
        }

        impl Text {
            /// All keys (for checking the translations).
            pub const ALL: &'static [Self] = &[ $( Self::$key, )+ ];
        }
    };
}

text_keys! {
    AppTitle,
    NewGame,
    Open,
    Save,
    Settings,
    Close,
    Cancel,
    Start,
    Ok,
    YourColor,
    White,
    Black,
    Level,
    LevelBeginner,
    LevelWeak,
    LevelAmateur,
    LevelSkilled,
    LevelStrong,
    LevelMaximum,
    Language,
    Sound,
    SoundOn,
    SoundOff,
    MovesPanel,
    Effects,
    EffectsEnabled,
    EffectLightning,
    EffectSparks,
    EffectFlashes,
    ShowPanel,
    HidePanel,
    YourMove,
    MustCapture,
    Thinking,
    ComputerToMove,
    Review,
    ContinueHere,
    ToStart,
    StepBack,
    StepForward,
    ToEnd,
    Variations,
    NoMovesYet,
    YouPlayWhite,
    YouPlayBlack,
    YouWin,
    YouLose,
    Draw,
    WhiteNoPieces,
    BlackNoPieces,
    WhiteNoMoves,
    BlackNoMoves,
    /// Template: `{count}`.
    DrawRepetition,
    /// Template: `{moves}`.
    DrawKingsVsKing,
    /// Template: `{moves}`.
    DrawKingsOnly,
    /// Template: `{moves}`, `{pieces}`.
    DrawBalance,
    /// Template: `{moves}`.
    DrawMainRoad,
    OpenTitle,
    SaveTitle,
    PdnFilter,
    ErrorTitle,
    ReadFailed,
    WriteFailed,
    SettingsNotSaved,
    /// Shown when the window cannot be created; the system's own error text follows.
    StartFailed,
    PdnTooLarge,
    PdnEmpty,
    PdnNotPdn,
    PdnTooManyMoves,
    /// Template: `{value}`.
    PdnWrongGameType,
    /// Template: `{value}`.
    PdnBadFen,
    /// Template: `{line}`.
    PdnBadTag,
    /// Template: `{line}`.
    PdnUnbalancedVariation,
    /// Template: `{line}`.
    PdnVariationWithoutMove,
    /// Template: `{line}`.
    PdnUnterminatedComment,
    /// Template: `{line}`, `{token}`, `{problem}` (one of the `Move…` texts).
    PdnBadMove,
    MoveSyntax,
    MoveIllegal,
    MoveAmbiguous,
    MoveGameOver,
    HumanName,
    ComputerName,
    Keys,
    KeysHelp,
}

/// The interface string for `key` in the chosen language. Templates are returned as
/// they are, with their placeholders.
#[must_use]
pub const fn tr(lang: Lang, key: Text) -> &'static str {
    match lang {
        Lang::Id => indonesian::text(key),
        Lang::De => german::text(key),
        Lang::En => english::text(key),
        Lang::Es => spanish::text(key),
        Lang::Fr => french::text(key),
        Lang::It => italian::text(key),
        Lang::Pt => portuguese::text(key),
        Lang::Tr => turkish::text(key),
        Lang::Ru => russian::text(key),
        Lang::Zh => chinese::text(key),
        Lang::Ja => japanese::text(key),
        Lang::Ko => korean::text(key),
    }
}

/// Fills a template: every `{name}` is replaced with the value given for `name`.
///
/// The template is read once from left to right, so braces inside the inserted values
/// (which may come from a file) are never treated as placeholders. An unknown
/// placeholder and an unmatched brace are kept as they are.
fn fill(template: &str, values: &[(&str, &dyn Display)]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let known = after.find('}').and_then(|close| {
            let name = &after[..close];
            let value = values.iter().find(|(key, _)| *key == name)?;
            Some((value.1, close))
        });
        if let Some((value, close)) = known {
            out.push_str(&value.to_string());
            rest = &after[close + 1..];
        } else {
            out.push('{');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// The names of the placeholders of a template, in order of appearance.
#[must_use]
pub fn placeholders(template: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                names.push(&after[..close]);
                rest = &after[close + 1..];
            }
            None => break,
        }
    }
    names
}

/// Level caption: the number and a word describing the strength.
#[must_use]
pub fn level_label(lang: Lang, level: u8) -> String {
    let name = match level {
        0..=1 => Text::LevelBeginner,
        2..=3 => Text::LevelWeak,
        4..=5 => Text::LevelAmateur,
        6..=7 => Text::LevelSkilled,
        8..=9 => Text::LevelStrong,
        _ => Text::LevelMaximum,
    };
    format!("{level} — {}", tr(lang, name))
}

/// The headline of the result as seen by the human, who plays `human`.
#[must_use]
pub fn outcome_title(lang: Lang, outcome: &Outcome, human: Color) -> &'static str {
    match outcome.winner() {
        Some(winner) if winner == human => tr(lang, Text::YouWin),
        Some(_) => tr(lang, Text::YouLose),
        None => tr(lang, Text::Draw),
    }
}

/// Why the game ended, naming the exact draw rule.
#[must_use]
pub fn outcome_reason(lang: Lang, outcome: &Outcome) -> String {
    match *outcome {
        Outcome::Win { winner, reason } => {
            let key = match (winner.opponent(), reason) {
                (Color::White, WinReason::NoPieces) => Text::WhiteNoPieces,
                (Color::Black, WinReason::NoPieces) => Text::BlackNoPieces,
                (Color::White, WinReason::NoMoves) => Text::WhiteNoMoves,
                (Color::Black, WinReason::NoMoves) => Text::BlackNoMoves,
            };
            tr(lang, key).to_string()
        }
        Outcome::Draw(DrawReason::Repetition) => {
            fill(tr(lang, Text::DrawRepetition), &[("count", &REPETITION_LIMIT)])
        }
        Outcome::Draw(DrawReason::KingsVsKing) => fill(
            tr(lang, Text::DrawKingsVsKing),
            &[("moves", &KINGS_VS_KING_MOVES)],
        ),
        Outcome::Draw(DrawReason::KingsOnly) => {
            fill(tr(lang, Text::DrawKingsOnly), &[("moves", &KINGS_ONLY_MOVES)])
        }
        Outcome::Draw(DrawReason::MainRoad) => {
            fill(tr(lang, Text::DrawMainRoad), &[("moves", &MAIN_ROAD_MOVES)])
        }
        Outcome::Draw(DrawReason::Balance { pieces, moves }) => fill(
            tr(lang, Text::DrawBalance),
            &[("moves", &moves), ("pieces", &pieces)],
        ),
    }
}

/// A message about a PDN loading error that a player can understand.
#[must_use]
pub fn pdn_error_text(lang: Lang, error: &PdnError) -> String {
    let with_line = |key, line: &usize| fill(tr(lang, key), &[("line", line)]);
    let with_value = |key, value: &String| fill(tr(lang, key), &[("value", value)]);
    match error {
        PdnError::TooLarge => tr(lang, Text::PdnTooLarge).to_string(),
        PdnError::Empty => tr(lang, Text::PdnEmpty).to_string(),
        PdnError::NotPdn => tr(lang, Text::PdnNotPdn).to_string(),
        PdnError::TooManyMoves => tr(lang, Text::PdnTooManyMoves).to_string(),
        PdnError::WrongGameType(value) => with_value(Text::PdnWrongGameType, value),
        PdnError::BadFen(value) => with_value(Text::PdnBadFen, value),
        PdnError::BadTag { line } => with_line(Text::PdnBadTag, line),
        PdnError::UnbalancedVariation { line } => with_line(Text::PdnUnbalancedVariation, line),
        PdnError::VariationWithoutMove { line } => with_line(Text::PdnVariationWithoutMove, line),
        PdnError::UnterminatedComment { line } => with_line(Text::PdnUnterminatedComment, line),
        PdnError::BadMove { line, token, problem } => {
            let problem = tr(
                lang,
                match problem {
                    MoveProblem::Syntax => Text::MoveSyntax,
                    MoveProblem::Illegal => Text::MoveIllegal,
                    MoveProblem::Ambiguous => Text::MoveAmbiguous,
                    MoveProblem::GameOver => Text::MoveGameOver,
                },
            );
            fill(
                tr(lang, Text::PdnBadMove),
                &[("line", line), ("token", token), ("problem", &problem)],
            )
        }
    }
}

#[cfg(test)]
mod tests;
