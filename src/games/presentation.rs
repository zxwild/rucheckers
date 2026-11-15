//! Localised descriptions of game results.

use super::{Color, FinishReason, Outcome};

#[must_use]
pub fn outcome_title(lang: crate::i18n::Lang, outcome: &Outcome, human: Color) -> &'static str {
    use crate::i18n::{tr, Text};
    tr(
        lang,
        match outcome.winner() {
            Some(winner) if winner == human => Text::YouWin,
            Some(_) => Text::YouLose,
            None => Text::Draw,
        },
    )
}

#[must_use]
pub fn outcome_reason(lang: crate::i18n::Lang, outcome: &Outcome) -> String {
    use crate::i18n::{tr, Text};
    let reason = match *outcome {
        Outcome::Russian(native) => return crate::i18n::outcome_reason(lang, &native),
        Outcome::Win { reason, .. } | Outcome::Draw(reason) => reason,
    };
    let key = match reason {
        FinishReason::HomeFilled => Text::UgolkiHomeFilled,
        FinishReason::HomeDeadline if outcome.is_draw() => Text::UgolkiBothHomeDeadline,
        FinishReason::HomeDeadline => Text::UgolkiHomeDeadline,
        FinishReason::MoveLimit => Text::UgolkiMoveLimit,
        FinishReason::Simultaneous => Text::UgolkiSimultaneous,
        FinishReason::Blocked => Text::UgolkiBlocked,
        FinishReason::Repetition => return tr(lang, Text::DrawRepetition).replace("{count}", "3"),
    };
    tr(lang, key).into()
}
