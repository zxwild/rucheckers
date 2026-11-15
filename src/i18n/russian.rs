//! Russian.

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::YourHome => "Ваш дом",
        Text::HomeLeft => "Слева",
        Text::HomeRight => "Справа",
        Text::GameRules => "Правила",
        Text::GameMode => "Игра",
        Text::Ugolki => "Уголки",
        Text::GameAppliesNew => "Выбор игры применяется при начале новой партии.",
        Text::UgolkiHint => "Переведите 12 шашек в противоположный дом. Шаг или цепочка прыжков — только по горизонтали и вертикали, без взятий. Выберите конечную клетку. Свой дом нужно освободить за 40 ходов; предел — 80 ходов. Чёрные имеют право на последний ответный ход. Запирать шашки соперника нельзя.",
        Text::UgolkiHomeFilled => "Все шашки переведены в дом соперника",
        Text::UgolkiHomeDeadline => "Свой дом не освобождён за 40 ходов",
        Text::UgolkiBothHomeDeadline => "После 40 ходов шашки остались в обоих своих домах",
        Text::UgolkiMoveLimit => "Предел 80 ходов: результат определён по числу шашек в доме соперника",
        Text::UgolkiSimultaneous => "Обе стороны заняли дом соперника за одинаковое число ходов",
        Text::UgolkiBlocked => "Нет допустимых ходов — ничья",
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
        Text::StartFailed => {
            "Не удалось открыть окно. Программе нужны графический сеанс и видеодрайвер с поддержкой OpenGL 2.0 или новее."
        }
        Text::PdnTooLarge => "Файл слишком велик для записи партии.",
        Text::PdnEmpty => "Файл пуст: в нём нет партии.",
        Text::PdnNotPdn => "Файл не похож на запись партии в формате PDN.",
        Text::PdnTooManyMoves => "В партии слишком много ходов.",
        Text::PdnWrongGameType => "Неизвестный вид игры (GameType «{value}»). Поддерживаются русские шашки (GameType 25) и уголки.",
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
