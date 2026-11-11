//! Тексты интерфейса на русском и английском языках.
//!
//! Все строки собраны здесь, чтобы язык переключался без перезапуска и чтобы
//! полноту перевода можно было проверить тестом.

use crate::board::Color;
use crate::pdn::{MoveProblem, PdnError};
use crate::rules::{DrawReason, Outcome, WinReason};

/// Язык интерфейса.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Lang {
    Ru,
    En,
}

impl Lang {
    /// Код языка для файла настроек.
    pub fn code(self) -> &'static str {
        match self {
            Lang::Ru => "ru",
            Lang::En => "en",
        }
    }

    /// Язык по коду; неизвестный код даёт `None`.
    pub fn from_code(code: &str) -> Option<Lang> {
        match code.trim().to_ascii_lowercase().as_str() {
            "ru" => Some(Lang::Ru),
            "en" => Some(Lang::En),
            _ => None,
        }
    }

    /// Название языка на нём самом.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::Ru => "Русский",
            Lang::En => "English",
        }
    }

    /// Язык по значению переменных окружения локали (`LC_ALL`, `LC_MESSAGES`, `LANG`).
    /// Если локаль не задана, выбирается русский.
    pub fn from_locale(locale: Option<&str>) -> Lang {
        match locale.map(|l| l.trim().to_ascii_lowercase()) {
            Some(l) if l.is_empty() || l == "c" || l == "posix" || l.starts_with("ru") => Lang::Ru,
            Some(_) => Lang::En,
            None => Lang::Ru,
        }
    }
}

macro_rules! texts {
    ($( $key:ident => ($ru:expr, $en:expr), )+) => {
        /// Ключ строки интерфейса.
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum Text {
            $( $key, )+
        }

        impl Text {
            /// Все ключи (для проверки полноты перевода).
            pub const ALL: &'static [Text] = &[ $( Text::$key, )+ ];
        }

        /// Строка интерфейса на выбранном языке.
        pub fn tr(lang: Lang, key: Text) -> &'static str {
            match (key, lang) {
                $(
                    (Text::$key, Lang::Ru) => $ru,
                    (Text::$key, Lang::En) => $en,
                )+
            }
        }
    };
}

texts! {
    AppTitle => ("Русские шашки", "Russian Draughts"),
    NewGame => ("Новая партия", "New game"),
    Open => ("Открыть…", "Open…"),
    Save => ("Сохранить…", "Save…"),
    Settings => ("Настройки", "Settings"),
    Close => ("Закрыть", "Close"),
    Cancel => ("Отмена", "Cancel"),
    Start => ("Начать", "Start"),
    Ok => ("Понятно", "OK"),
    YourColor => ("Ваш цвет", "Your colour"),
    White => ("Белые", "White"),
    Black => ("Чёрные", "Black"),
    Level => ("Уровень", "Level"),
    LevelBeginner => ("новичок", "beginner"),
    LevelWeak => ("слабый", "weak"),
    LevelAmateur => ("любитель", "amateur"),
    LevelSkilled => ("опытный", "skilled"),
    LevelStrong => ("сильный", "strong"),
    LevelMaximum => ("максимум", "maximum"),
    Language => ("Язык", "Language"),
    Sound => ("Звук", "Sound"),
    SoundOn => ("Звук включён", "Sound on"),
    SoundOff => ("Звук выключен", "Sound off"),
    MovesPanel => ("Панель ходов", "Move panel"),
    ShowPanel => ("Показать панель", "Show panel"),
    HidePanel => ("Скрыть панель", "Hide panel"),
    YourMove => ("Ваш ход", "Your move"),
    MustCapture => ("Ваш ход: взятие обязательно", "Your move: capture is mandatory"),
    Thinking => ("Компьютер думает…", "Computer is thinking…"),
    ComputerToMove => ("Ход компьютера", "Computer to move"),
    Review => ("Просмотр", "Review"),
    ContinueHere => ("Продолжить отсюда (Пробел)", "Continue from here (Space)"),
    ToStart => ("В начало (Home)", "To start (Home)"),
    StepBack => ("Ход назад (стрелка влево)", "Step back (Left arrow)"),
    StepForward => ("Ход вперёд (стрелка вправо)", "Step forward (Right arrow)"),
    ToEnd => ("В конец (End)", "To end (End)"),
    Variations => ("варианты:", "variations:"),
    NoMovesYet => ("Ходов пока нет", "No moves yet"),
    YouPlayWhite => ("Вы играете белыми", "You play White"),
    YouPlayBlack => ("Вы играете чёрными", "You play Black"),
    YouWin => ("Вы выиграли", "You won"),
    YouLose => ("Вы проиграли", "You lost"),
    Draw => ("Ничья", "Draw"),
    GameOver => ("Партия окончена", "Game over"),
    WhiteNoPieces => ("У белых не осталось шашек", "White has no pieces left"),
    BlackNoPieces => ("У чёрных не осталось шашек", "Black has no pieces left"),
    WhiteNoMoves => ("У белых нет допустимых ходов", "White has no legal moves"),
    BlackNoMoves => ("У чёрных нет допустимых ходов", "Black has no legal moves"),
    DrawRepetition => (
        "Позиция повторилась в третий раз",
        "The position occurred for the third time"
    ),
    DrawKingsVsKing => (
        "Три и более дамки не взяли одинокую дамку за 15 ходов",
        "Three or more kings failed to capture a lone king in 15 moves"
    ),
    DrawKingsOnly => (
        "15 ходов сделаны только дамками, без взятий и ходов простых",
        "15 moves were made by kings only, without captures or man moves"
    ),
    DrawMainRoad => (
        "Три шашки не взяли одинокую дамку на большой дороге за 5 ходов",
        "Three pieces failed to capture a lone king on the main road in 5 moves"
    ),
    OpenTitle => ("Открыть партию", "Open game"),
    SaveTitle => ("Сохранить партию", "Save game"),
    PdnFilter => ("Партии PDN", "PDN games"),
    ErrorTitle => ("Ошибка", "Error"),
    ReadFailed => ("Не удалось прочитать файл", "Could not read the file"),
    WriteFailed => ("Не удалось сохранить файл", "Could not save the file"),
    PdnTooLarge => ("Файл слишком велик для записи партии.", "The file is too large to be a game record."),
    PdnEmpty => ("Файл пуст: в нём нет партии.", "The file is empty: it contains no game."),
    PdnNotPdn => (
        "Файл не похож на запись партии в формате PDN.",
        "The file does not look like a PDN game record."
    ),
    PdnTooManyMoves => ("В партии слишком много ходов.", "The game has too many moves."),
    HumanName => ("Игрок", "Player"),
    ComputerName => ("Компьютер", "Computer"),
    Keys => ("Клавиши", "Keys"),
    KeysHelp => (
        "Стрелки влево и вправо — по ходам, вверх и вниз — варианты,\nHome и End — начало и конец партии, Пробел — продолжить отсюда,\nEsc или правая кнопка мыши — отменить выбор, F9 — панель ходов,\nCtrl+N, Ctrl+O, Ctrl+S (на macOS — Cmd) — новая партия, открыть, сохранить",
        "Left and Right arrows — step through moves, Up and Down — variations,\nHome and End — start and end of the game, Space — continue from here,\nEsc or right mouse button — cancel selection, F9 — move panel,\nCtrl+N, Ctrl+O, Ctrl+S (Cmd on macOS) — new game, open, save"
    ),
}

/// Подпись уровня: номер и словесная характеристика.
pub fn level_label(lang: Lang, level: u8) -> String {
    let name = match level {
        0..=1 => Text::LevelBeginner,
        2..=3 => Text::LevelWeak,
        4..=5 => Text::LevelAmateur,
        6..=7 => Text::LevelSkilled,
        8..=9 => Text::LevelStrong,
        _ => Text::LevelMaximum,
    };
    format!("{} — {}", level, tr(lang, name))
}

/// Заголовок результата с точки зрения человека, играющего цветом `human`.
pub fn outcome_title(lang: Lang, outcome: &Outcome, human: Color) -> &'static str {
    match outcome.winner() {
        Some(winner) if winner == human => tr(lang, Text::YouWin),
        Some(_) => tr(lang, Text::YouLose),
        None => tr(lang, Text::Draw),
    }
}

/// Причина окончания партии, включая конкретное правило ничьей.
pub fn outcome_reason(lang: Lang, outcome: &Outcome) -> String {
    match outcome {
        Outcome::Win { winner, reason } => {
            let key = match (winner.opponent(), reason) {
                (Color::White, WinReason::NoPieces) => Text::WhiteNoPieces,
                (Color::Black, WinReason::NoPieces) => Text::BlackNoPieces,
                (Color::White, WinReason::NoMoves) => Text::WhiteNoMoves,
                (Color::Black, WinReason::NoMoves) => Text::BlackNoMoves,
            };
            tr(lang, key).to_string()
        }
        Outcome::Draw(DrawReason::Repetition) => tr(lang, Text::DrawRepetition).to_string(),
        Outcome::Draw(DrawReason::KingsVsKing) => tr(lang, Text::DrawKingsVsKing).to_string(),
        Outcome::Draw(DrawReason::KingsOnly) => tr(lang, Text::DrawKingsOnly).to_string(),
        Outcome::Draw(DrawReason::MainRoad) => tr(lang, Text::DrawMainRoad).to_string(),
        Outcome::Draw(DrawReason::Balance { pieces, moves }) => match lang {
            Lang::Ru => format!(
                "Соотношение сил не менялось {moves} ходов (у обеих сторон дамки, фигур на доске: {pieces})"
            ),
            Lang::En => format!(
                "The balance of forces has not changed for {moves} moves (both sides have kings, pieces on board: {pieces})"
            ),
        },
    }
}

/// Понятное сообщение об ошибке загрузки PDN.
pub fn pdn_error_text(lang: Lang, error: &PdnError) -> String {
    match error {
        PdnError::TooLarge => tr(lang, Text::PdnTooLarge).to_string(),
        PdnError::Empty => tr(lang, Text::PdnEmpty).to_string(),
        PdnError::NotPdn => tr(lang, Text::PdnNotPdn).to_string(),
        PdnError::TooManyMoves => tr(lang, Text::PdnTooManyMoves).to_string(),
        PdnError::WrongGameType(value) => match lang {
            Lang::Ru => format!(
                "Это партия другого вида шашек (GameType «{value}»). Поддерживаются только русские шашки (GameType 25)."
            ),
            Lang::En => format!(
                "This is a game of another draughts variant (GameType \"{value}\"). Only Russian draughts (GameType 25) is supported."
            ),
        },
        PdnError::BadFen(value) => match lang {
            Lang::Ru => format!("Неверная начальная позиция в теге FEN: «{value}»."),
            Lang::En => format!("Invalid start position in the FEN tag: \"{value}\"."),
        },
        PdnError::BadTag { line } => match lang {
            Lang::Ru => format!("Строка {line}: повреждён заголовок партии (тег не закрыт)."),
            Lang::En => format!("Line {line}: the game header is damaged (unterminated tag)."),
        },
        PdnError::UnbalancedVariation { line } => match lang {
            Lang::Ru => format!("Строка {line}: скобки вариантов расставлены неверно."),
            Lang::En => format!("Line {line}: variation parentheses are unbalanced."),
        },
        PdnError::VariationWithoutMove { line } => match lang {
            Lang::Ru => format!("Строка {line}: вариант начат раньше хода, к которому он относится."),
            Lang::En => format!("Line {line}: a variation starts before the move it refers to."),
        },
        PdnError::UnterminatedComment { line } => match lang {
            Lang::Ru => format!("Строка {line}: комментарий не закрыт."),
            Lang::En => format!("Line {line}: unterminated comment."),
        },
        PdnError::BadMove { line, token, problem } => {
            let what = match (lang, problem) {
                (Lang::Ru, MoveProblem::Syntax) => "это не запись хода",
                (Lang::Ru, MoveProblem::Illegal) => "такой ход невозможен в этой позиции",
                (Lang::Ru, MoveProblem::Ambiguous) => "запись взятия неоднозначна",
                (Lang::Ru, MoveProblem::GameOver) => "партия к этому моменту уже закончена",
                (Lang::En, MoveProblem::Syntax) => "this is not a move",
                (Lang::En, MoveProblem::Illegal) => "the move is not legal in this position",
                (Lang::En, MoveProblem::Ambiguous) => "the capture notation is ambiguous",
                (Lang::En, MoveProblem::GameOver) => "the game is already over at this point",
            };
            match lang {
                Lang::Ru => format!("Строка {line}: «{token}» — {what}."),
                Lang::En => format!("Line {line}: \"{token}\" — {what}."),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_is_translated() {
        for &key in Text::ALL {
            for lang in [Lang::Ru, Lang::En] {
                assert!(!tr(lang, key).trim().is_empty(), "{key:?} / {lang:?}");
            }
        }
        // Русские строки действительно на кириллице, английские — нет.
        for &key in Text::ALL {
            let ru = tr(Lang::Ru, key);
            let en = tr(Lang::En, key);
            let is_cyrillic = |c: char| ('а'..='я').contains(&c) || ('А'..='Я').contains(&c) || c == 'ё';
            assert!(ru.chars().any(is_cyrillic), "{key:?}: {ru}");
            assert!(!en.chars().any(is_cyrillic), "{key:?}: {en}");
        }
    }

    #[test]
    fn language_codes_round_trip() {
        for lang in [Lang::Ru, Lang::En] {
            assert_eq!(Lang::from_code(lang.code()), Some(lang));
        }
        assert_eq!(Lang::from_code(" EN "), Some(Lang::En));
        assert_eq!(Lang::from_code("de"), None);
        assert_eq!(Lang::from_locale(Some("ru_RU.UTF-8")), Lang::Ru);
        assert_eq!(Lang::from_locale(Some("en_US.UTF-8")), Lang::En);
        assert_eq!(Lang::from_locale(Some("de_DE")), Lang::En);
        assert_eq!(Lang::from_locale(Some("C")), Lang::Ru);
        assert_eq!(Lang::from_locale(None), Lang::Ru);
    }

    #[test]
    fn every_outcome_has_a_title_and_a_reason() {
        let outcomes = [
            Outcome::Win {
                winner: Color::White,
                reason: WinReason::NoPieces,
            },
            Outcome::Win {
                winner: Color::White,
                reason: WinReason::NoMoves,
            },
            Outcome::Win {
                winner: Color::Black,
                reason: WinReason::NoPieces,
            },
            Outcome::Win {
                winner: Color::Black,
                reason: WinReason::NoMoves,
            },
            Outcome::Draw(DrawReason::Repetition),
            Outcome::Draw(DrawReason::KingsVsKing),
            Outcome::Draw(DrawReason::KingsOnly),
            Outcome::Draw(DrawReason::Balance { pieces: 5, moves: 30 }),
            Outcome::Draw(DrawReason::MainRoad),
        ];
        let mut reasons = std::collections::HashSet::new();
        for outcome in &outcomes {
            for lang in [Lang::Ru, Lang::En] {
                let reason = outcome_reason(lang, outcome);
                assert!(!reason.is_empty());
                assert!(reasons.insert(reason), "причины различаются между собой");
                assert!(!outcome_title(lang, outcome, Color::White).is_empty());
            }
        }
        let white_wins = Outcome::Win {
            winner: Color::White,
            reason: WinReason::NoMoves,
        };
        assert_eq!(outcome_title(Lang::Ru, &white_wins, Color::White), "Вы выиграли");
        assert_eq!(outcome_title(Lang::Ru, &white_wins, Color::Black), "Вы проиграли");
        assert_eq!(
            outcome_reason(Lang::Ru, &white_wins),
            "У чёрных нет допустимых ходов"
        );
        let balance = outcome_reason(
            Lang::Ru,
            &Outcome::Draw(DrawReason::Balance { pieces: 5, moves: 30 }),
        );
        assert!(balance.contains("30") && balance.contains('5'));
    }

    #[test]
    fn every_pdn_error_has_a_message() {
        let errors = [
            PdnError::TooLarge,
            PdnError::Empty,
            PdnError::NotPdn,
            PdnError::WrongGameType("20".into()),
            PdnError::BadFen("W:Wz9".into()),
            PdnError::BadTag { line: 3 },
            PdnError::BadMove {
                line: 7,
                token: "c3-c4".into(),
                problem: MoveProblem::Syntax,
            },
            PdnError::BadMove {
                line: 7,
                token: "c3-d4".into(),
                problem: MoveProblem::Illegal,
            },
            PdnError::BadMove {
                line: 7,
                token: "c3:g7".into(),
                problem: MoveProblem::Ambiguous,
            },
            PdnError::BadMove {
                line: 7,
                token: "c3-d4".into(),
                problem: MoveProblem::GameOver,
            },
            PdnError::UnbalancedVariation { line: 9 },
            PdnError::VariationWithoutMove { line: 9 },
            PdnError::UnterminatedComment { line: 9 },
            PdnError::TooManyMoves,
        ];
        for error in &errors {
            for lang in [Lang::Ru, Lang::En] {
                let text = pdn_error_text(lang, error);
                assert!(text.chars().count() > 10, "{error:?}: {text}");
            }
        }
        let text = pdn_error_text(Lang::Ru, &errors[6]);
        assert!(text.contains("Строка 7") && text.contains("c3-c4"), "{text}");
    }

    #[test]
    fn level_labels() {
        assert_eq!(level_label(Lang::Ru, 1), "1 — новичок");
        assert_eq!(level_label(Lang::En, 10), "10 — maximum");
        for level in 1..=10 {
            assert!(level_label(Lang::Ru, level).starts_with(&level.to_string()));
        }
    }
}
