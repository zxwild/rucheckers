//! Russian.

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::AppTitle => "Русские шашки",
        Text::NewGame => "Новая партия",
        Text::Open => "Открыть…",
        Text::Save => "Сохранить…",
        Text::Settings => "Настройки",
        Text::Close => "Закрыть",
        Text::Cancel => "Отмена",
        Text::Start => "Начать",
        Text::Ok => "Понятно",
        Text::YourColor => "Ваш цвет",
        Text::White => "Белые",
        Text::Black => "Чёрные",
        Text::Level => "Уровень",
        Text::LevelBeginner => "новичок",
        Text::LevelWeak => "слабый",
        Text::LevelAmateur => "любитель",
        Text::LevelSkilled => "опытный",
        Text::LevelStrong => "сильный",
        Text::LevelMaximum => "максимум",
        Text::Language => "Язык",
        Text::Sound => "Звук",
        Text::SoundOn => "Звук включён",
        Text::SoundOff => "Звук выключен",
        Text::MovesPanel => "Панель ходов",
        Text::Effects => "Спецэффекты",
        Text::EffectsEnabled => "Показывать спецэффекты",
        Text::EffectLightning => "Молния за шашкой при взятии нескольких шашек",
        Text::EffectSparks => "Искры от побитых шашек",
        Text::EffectFlashes => "Вспышки и свечение",
        Text::ShowPanel => "Показать панель",
        Text::HidePanel => "Скрыть панель",
        Text::YourMove => "Ваш ход",
        Text::MustCapture => "Ваш ход: взятие обязательно",
        Text::Thinking => "Компьютер думает…",
        Text::ComputerToMove => "Ход компьютера",
        Text::Review => "Просмотр",
        Text::ContinueHere => "Продолжить отсюда (Пробел)",
        Text::ToStart => "В начало (Home)",
        Text::StepBack => "Ход назад (стрелка влево)",
        Text::StepForward => "Ход вперёд (стрелка вправо)",
        Text::ToEnd => "В конец (End)",
        Text::Variations => "варианты:",
        Text::NoMovesYet => "Ходов пока нет",
        Text::YouPlayWhite => "Вы играете белыми",
        Text::YouPlayBlack => "Вы играете чёрными",
        Text::YouWin => "Вы выиграли",
        Text::YouLose => "Вы проиграли",
        Text::Draw => "Ничья",
        Text::WhiteNoPieces => "У белых не осталось шашек",
        Text::BlackNoPieces => "У чёрных не осталось шашек",
        Text::WhiteNoMoves => "У белых нет допустимых ходов",
        Text::BlackNoMoves => "У чёрных нет допустимых ходов",
        Text::DrawRepetition => "Одна и та же позиция повторилась {count} раза",
        Text::DrawKingsVsKing => "Три и более дамки не взяли одинокую дамку за {moves} ходов",
        Text::DrawKingsOnly => "{moves} ходов сделаны только дамками, без взятий и ходов простых",
        Text::DrawBalance => {
            "Соотношение сил не менялось {moves} ходов \
             (у обеих сторон дамки, фигур на доске: {pieces})"
        }
        Text::DrawMainRoad => "Три шашки не взяли одинокую дамку на большой дороге за {moves} ходов",
        Text::OpenTitle => "Открыть партию",
        Text::SaveTitle => "Сохранить партию",
        Text::PdnFilter => "Партии PDN",
        Text::ErrorTitle => "Ошибка",
        Text::ReadFailed => "Не удалось прочитать файл",
        Text::WriteFailed => "Не удалось сохранить файл",
        Text::SettingsNotSaved => "Не удалось сохранить настройки",
        Text::PdnTooLarge => "Файл слишком велик для записи партии.",
        Text::PdnEmpty => "Файл пуст: в нём нет партии.",
        Text::PdnNotPdn => "Файл не похож на запись партии в формате PDN.",
        Text::PdnTooManyMoves => "В партии слишком много ходов.",
        Text::PdnWrongGameType => {
            "Это партия другого вида шашек (GameType «{value}»). \
             Поддерживаются только русские шашки (GameType 25)."
        }
        Text::PdnBadFen => "Неверная начальная позиция в теге FEN: «{value}».",
        Text::PdnBadTag => "Строка {line}: повреждён заголовок партии (тег не закрыт).",
        Text::PdnUnbalancedVariation => "Строка {line}: скобки вариантов расставлены неверно.",
        Text::PdnVariationWithoutMove => "Строка {line}: вариант начат раньше хода, к которому он относится.",
        Text::PdnUnterminatedComment => "Строка {line}: комментарий не закрыт.",
        Text::PdnBadMove => "Строка {line}: «{token}» — {problem}.",
        Text::MoveSyntax => "это не запись хода",
        Text::MoveIllegal => "такой ход невозможен в этой позиции",
        Text::MoveAmbiguous => "запись взятия неоднозначна",
        Text::MoveGameOver => "партия к этому моменту уже закончена",
        Text::HumanName => "Игрок",
        Text::ComputerName => "Компьютер",
        Text::Keys => "Клавиши",
        Text::KeysHelp => {
            "Стрелки влево и вправо — по ходам; \
             вверх и вниз — варианты; \
             Home и End — начало и конец партии; \
             Пробел — продолжить отсюда; \
             Esc или правая кнопка мыши — отменить выбор; \
             F9 — панель ходов; \
             Ctrl+N, Ctrl+O, Ctrl+S (на macOS — Cmd) — новая партия, открыть, сохранить"
        }
    }
}
