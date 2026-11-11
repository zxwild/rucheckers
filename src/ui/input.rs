//! Выбор хода мышью: щелчки и перетаскивание, пошаговое выполнение серийного взятия.
//!
//! Модуль не знает ни об окне, ни о координатах: он получает поля доски и возвращает
//! события. Серийное взятие можно выполнять по одному прыжку — тогда при нескольких
//! маршрутах игрок сам выбирает следующее поле — или сразу указать конечное поле,
//! если результат взятия при этом определён однозначно.

use crate::board::Square;
use crate::moves::Move;

/// Результат действия игрока.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputEvent {
    /// Ничего не изменилось.
    None,
    /// Выбрана шашка.
    Selected(Square),
    /// Выбор снят.
    Cleared,
    /// Сделан промежуточный прыжок серийного взятия; ход ещё не закончен.
    Step { from: Square, to: Square },
    /// Ход выбран полностью.
    Move(Move),
}

/// Состояние выбора хода.
#[derive(Clone, Debug, Default)]
pub struct MoveInput {
    moves: Vec<Move>,
    origin: Option<Square>,
    steps: Vec<Square>,
}

impl MoveInput {
    /// Выбор хода без доступных ходов (ввод запрещён).
    pub fn new() -> MoveInput {
        MoveInput::default()
    }

    /// Задаёт допустимые ходы текущей позиции и сбрасывает выбор.
    pub fn set_moves(&mut self, moves: Vec<Move>) {
        self.moves = moves;
        self.origin = None;
        self.steps.clear();
    }

    /// Запрещает ввод.
    pub fn clear(&mut self) {
        self.set_moves(Vec::new());
    }

    /// Можно ли сейчас делать ход.
    pub fn is_enabled(&self) -> bool {
        !self.moves.is_empty()
    }

    /// Допустимые ходы.
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// Исходное поле выбранной шашки.
    pub fn origin(&self) -> Option<Square> {
        self.origin
    }

    /// Уже сделанные прыжки незаконченного взятия.
    pub fn steps(&self) -> &[Square] {
        &self.steps
    }

    /// Идёт ли незаконченное серийное взятие.
    pub fn in_progress(&self) -> bool {
        !self.steps.is_empty()
    }

    /// Поле, на котором выбранная шашка стоит сейчас.
    pub fn current_square(&self) -> Option<Square> {
        self.steps.last().copied().or(self.origin)
    }

    /// Обязано ли взятие (все допустимые ходы — взятия).
    pub fn must_capture(&self) -> bool {
        self.moves.first().is_some_and(Move::is_capture)
    }

    /// Шашки, которыми можно ходить. Если взятие обязательно, это шашки, обязанные бить.
    pub fn movable(&self) -> Vec<Square> {
        let mut squares: Vec<Square> = Vec::new();
        for mv in &self.moves {
            if !squares.contains(&mv.from()) {
                squares.push(mv.from());
            }
        }
        squares
    }

    /// Ходы, совместимые с выбранной шашкой и уже сделанными прыжками.
    fn candidates(&self) -> impl Iterator<Item = &Move> {
        let origin = self.origin;
        let steps = &self.steps;
        self.moves
            .iter()
            .filter(move |mv| Some(mv.from()) == origin && mv.path().starts_with(steps))
    }

    /// Поля, на которые выбранная шашка может пойти следующим шагом.
    pub fn targets(&self) -> Vec<Square> {
        let mut squares: Vec<Square> = Vec::new();
        let next = self.steps.len();
        for mv in self.candidates() {
            if let Some(&sq) = mv.path().get(next) {
                if !squares.contains(&sq) {
                    squares.push(sq);
                }
            }
        }
        squares
    }

    /// Взятие, которое завершится на поле `sq`, если указать это поле сразу, минуя
    /// промежуточные прыжки.
    ///
    /// Возвращает ход, только если результат однозначен: все подходящие маршруты бьют
    /// одни и те же шашки (тогда берётся первый из них). Если на одно поле ведут взятия
    /// с разным результатом, маршрут нужно уточнить шагами.
    fn shortcut(&self, sq: Square) -> Option<Move> {
        if self.current_square() == Some(sq) {
            // Щелчок по самой шашке — это снятие выбора, а не обход по кругу.
            return None;
        }
        let done = self.steps.len();
        let mut found: Option<Move> = None;
        for mv in self.candidates() {
            if mv.to() != sq || mv.path().len() <= done + 1 {
                continue;
            }
            match found {
                None => found = Some(*mv),
                Some(first) => {
                    if first.captured() != mv.captured() || first.promotes() != mv.promotes() {
                        return None;
                    }
                }
            }
        }
        found
    }

    /// Конечные поля серийного взятия, которые можно указать сразу: щелчок по такому
    /// полю выполняет все оставшиеся прыжки. Поля следующего шага сюда не входят.
    pub fn final_targets(&self) -> Vec<Square> {
        let next = self.targets();
        let mut squares: Vec<Square> = Vec::new();
        for mv in self.candidates() {
            let sq = mv.to();
            if !next.contains(&sq) && !squares.contains(&sq) && self.shortcut(sq).is_some() {
                squares.push(sq);
            }
        }
        squares
    }

    /// Выбор поля для выбранной шашки: следующий шаг либо сразу конечное поле взятия.
    /// Шаг имеет приоритет: поле, до которого один прыжок, всегда означает этот прыжок.
    fn choose(&mut self, sq: Square) -> Option<InputEvent> {
        self.origin?;
        if self.targets().contains(&sq) {
            return Some(self.advance(sq));
        }
        let mv = self.shortcut(sq)?;
        self.origin = None;
        self.steps.clear();
        Some(InputEvent::Move(mv))
    }

    /// Маска шашек, уже побитых в незаконченном взятии (они остаются на доске до конца хода).
    pub fn captured_so_far(&self) -> u32 {
        let done = self.steps.len();
        match self.candidates().next() {
            Some(mv) => (0..done)
                .filter_map(|i| mv.captured_at(i))
                .fold(0, |mask, sq| mask | sq.bit()),
            None => 0,
        }
    }

    /// Стала ли выбранная простая дамкой на одном из уже сделанных прыжков.
    pub fn promoted_so_far(&self) -> bool {
        let done = self.steps.len();
        self.candidates()
            .next()
            .and_then(Move::promotion_step)
            .is_some_and(|step| step < done)
    }

    /// Щелчок по полю.
    pub fn click(&mut self, sq: Square) -> InputEvent {
        if let Some(event) = self.choose(sq) {
            return event;
        }
        if self.in_progress() {
            // Посреди взятия посторонние щелчки игнорируются: бой нужно довести до конца.
            return InputEvent::None;
        }
        if self.moves.iter().any(|mv| mv.from() == sq) {
            if self.origin == Some(sq) {
                self.origin = None;
                return InputEvent::Cleared;
            }
            self.origin = Some(sq);
            return InputEvent::Selected(sq);
        }
        if self.origin.take().is_some() {
            return InputEvent::Cleared;
        }
        InputEvent::None
    }

    /// Начало перетаскивания с поля `sq`. Возвращает `true`, если шашку можно тащить:
    /// она выбрана (или только что выбрана) и стоит на этом поле.
    pub fn grab(&mut self, sq: Square) -> bool {
        if self.in_progress() {
            return self.current_square() == Some(sq);
        }
        if self.moves.iter().any(|mv| mv.from() == sq) {
            self.origin = Some(sq);
            true
        } else {
            false
        }
    }

    /// Конец перетаскивания над полем `sq`. Если поле недоступно, шашка возвращается
    /// на место, а выбор сохраняется.
    pub fn drop_on(&mut self, sq: Square) -> InputEvent {
        self.choose(sq).unwrap_or(InputEvent::None)
    }

    /// Снимает выбор и отменяет незаконченное взятие.
    pub fn cancel(&mut self) -> InputEvent {
        let had_selection = self.origin.is_some();
        self.origin = None;
        self.steps.clear();
        if had_selection {
            InputEvent::Cleared
        } else {
            InputEvent::None
        }
    }

    fn advance(&mut self, sq: Square) -> InputEvent {
        let Some(from) = self.current_square() else {
            return InputEvent::None;
        };
        self.steps.push(sq);
        let origin = self.origin;
        let complete = self
            .moves
            .iter()
            .find(|mv| Some(mv.from()) == origin && mv.path() == self.steps.as_slice())
            .copied();
        match complete {
            Some(mv) => {
                self.origin = None;
                self.steps.clear();
                InputEvent::Move(mv)
            }
            None => InputEvent::Step { from, to: sq },
        }
    }
}
