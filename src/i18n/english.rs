//! English.

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::AppTitle => "Russian Draughts",
        Text::NewGame => "New game",
        Text::Open => "Open…",
        Text::Save => "Save…",
        Text::Settings => "Settings",
        Text::Close => "Close",
        Text::Cancel => "Cancel",
        Text::Start => "Start",
        Text::Ok => "OK",
        Text::YourColor => "Your colour",
        Text::White => "White",
        Text::Black => "Black",
        Text::Level => "Level",
        Text::LevelBeginner => "beginner",
        Text::LevelWeak => "weak",
        Text::LevelAmateur => "amateur",
        Text::LevelSkilled => "skilled",
        Text::LevelStrong => "strong",
        Text::LevelMaximum => "maximum",
        Text::Language => "Language",
        Text::Sound => "Sound",
        Text::SoundOn => "Sound on",
        Text::SoundOff => "Sound off",
        Text::MovesPanel => "Move panel",
        Text::Effects => "Special effects",
        Text::EffectsEnabled => "Show special effects",
        Text::EffectLightning => "Lightning behind the piece in multi-captures",
        Text::EffectSparks => "Sparks from captured pieces",
        Text::EffectFlashes => "Flashes and glow",
        Text::ShowPanel => "Show panel",
        Text::HidePanel => "Hide panel",
        Text::YourMove => "Your move",
        Text::MustCapture => "Your move: capture is mandatory",
        Text::Thinking => "Computer is thinking…",
        Text::ComputerToMove => "Computer to move",
        Text::Review => "Review",
        Text::ContinueHere => "Continue from here (Space)",
        Text::ToStart => "To start (Home)",
        Text::StepBack => "Step back (Left arrow)",
        Text::StepForward => "Step forward (Right arrow)",
        Text::ToEnd => "To end (End)",
        Text::Variations => "variations:",
        Text::NoMovesYet => "No moves yet",
        Text::YouPlayWhite => "You play White",
        Text::YouPlayBlack => "You play Black",
        Text::YouWin => "You won",
        Text::YouLose => "You lost",
        Text::Draw => "Draw",
        Text::WhiteNoPieces => "White has no pieces left",
        Text::BlackNoPieces => "Black has no pieces left",
        Text::WhiteNoMoves => "White has no legal moves",
        Text::BlackNoMoves => "Black has no legal moves",
        Text::DrawRepetition => "The same position occurred {count} times",
        Text::DrawKingsVsKing => "Three or more kings failed to capture a lone king in {moves} moves",
        Text::DrawKingsOnly => "{moves} moves were made by kings only, without captures or man moves",
        Text::DrawBalance => {
            "The balance of forces has not changed for {moves} moves \
             (both sides have kings, pieces on board: {pieces})"
        }
        Text::DrawMainRoad => "Three pieces failed to capture a lone king on the main road in {moves} moves",
        Text::OpenTitle => "Open game",
        Text::SaveTitle => "Save game",
        Text::PdnFilter => "PDN games",
        Text::ErrorTitle => "Error",
        Text::ReadFailed => "Could not read the file",
        Text::WriteFailed => "Could not save the file",
        Text::SettingsNotSaved => "Could not save the settings",
        Text::StartFailed => {
            "The window could not be opened. The program needs a graphical session and a graphics driver with OpenGL 2.0 or newer."
        }
        Text::PdnTooLarge => "The file is too large to be a game record.",
        Text::PdnEmpty => "The file is empty: it contains no game.",
        Text::PdnNotPdn => "The file does not look like a PDN game record.",
        Text::PdnTooManyMoves => "The game has too many moves.",
        Text::PdnWrongGameType => {
            "This is a game of another draughts variant (GameType \"{value}\"). \
             Only Russian draughts (GameType 25) is supported."
        }
        Text::PdnBadFen => "Invalid start position in the FEN tag: \"{value}\".",
        Text::PdnBadTag => "Line {line}: the game header is damaged (unterminated tag).",
        Text::PdnUnbalancedVariation => "Line {line}: variation parentheses are unbalanced.",
        Text::PdnVariationWithoutMove => "Line {line}: a variation starts before the move it refers to.",
        Text::PdnUnterminatedComment => "Line {line}: unterminated comment.",
        Text::PdnBadMove => "Line {line}: \"{token}\" — {problem}.",
        Text::MoveSyntax => "this is not a move",
        Text::MoveIllegal => "the move is not legal in this position",
        Text::MoveAmbiguous => "the capture notation is ambiguous",
        Text::MoveGameOver => "the game is already over at this point",
        Text::HumanName => "Player",
        Text::ComputerName => "Computer",
        Text::Keys => "Keys",
        Text::KeysHelp => {
            "Left and Right arrows — step through moves; \
             Up and Down — variations; \
             Home and End — start and end of the game; \
             Space — continue from here; \
             Esc or right mouse button — cancel selection; \
             F9 — move panel; \
             Ctrl+N, Ctrl+O, Ctrl+S (Cmd on macOS) — new game, open, save"
        }
    }
}
