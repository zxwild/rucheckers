//! Игровая сессия: партия человека с компьютером поверх дерева вариантов.
//!
//! Сессия решает, чей сейчас ход, запускает и отменяет фоновый расчёт компьютера,
//! переключается между игрой и просмотром истории. Время передаётся извне, поэтому
//! логика проверяется без окна.

use super::input::{InputEvent, MoveInput};
use crate::ai::clamp_level;
use crate::ai::worker::{JobStatus, SearchJob};
use crate::board::{Color, Square};
use crate::moves::{legal_moves, Move};
use crate::rng::Rng;
use crate::rules::Outcome;
use crate::tree::{GameTree, NodeId};

/// Наименьшая пауза перед ответом компьютера, с: мгновенный ответ выглядит неестественно.
pub const DEFAULT_MIN_THINK_SECONDS: f64 = 0.6;

/// Переход по истории партии.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nav {
    Start,
    Back,
    Forward,
    End,
    PrevVariation,
    NextVariation,
    To(NodeId),
}

/// Событие сессии для интерфейса (анимация, звук).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionEvent {
    /// Сделан ход, приведший в узел `node`.
    Played { node: NodeId, by_computer: bool },
    /// Позиция сменилась без хода: переход по истории, новая или загруженная партия.
    Jumped,
}

/// The computer's reply to the position it was asked about.
#[derive(Clone, Copy, Debug)]
enum Reply {
    /// The search is still running.
    Pending,
    /// The search has finished; `None` means there was no move to make. The reply
    /// waits here until the natural pause before the computer's move is over.
    Ready(Option<Move>),
}

#[derive(Debug)]
struct Thinking {
    job: SearchJob,
    node: NodeId,
    started: f64,
    reply: Reply,
}

/// Партия с компьютером.
#[derive(Debug)]
pub struct Session {
    tree: GameTree,
    human: Color,
    level: u8,
    /// Идёт игра (`true`) или просмотр истории (`false`). При просмотре компьютер не ходит.
    live: bool,
    input: MoveInput,
    thinking: Option<Thinking>,
    rng: Rng,
    events: Vec<SessionEvent>,
    min_think: f64,
}

impl Session {
    /// Новая партия: человек играет цветом `human` против уровня `level`.
    #[must_use]
    pub fn new(human: Color, level: u8, seed: u64) -> Self {
        let mut session = Self {
            tree: GameTree::new(),
            human,
            level: clamp_level(level),
            live: true,
            input: MoveInput::new(),
            thinking: None,
            rng: Rng::new(seed),
            events: Vec::new(),
            min_think: DEFAULT_MIN_THINK_SECONDS,
        };
        session.refresh_input();
        session
    }

    /// Начинает новую партию в этой же сессии.
    pub fn new_game(&mut self, human: Color, level: u8) {
        self.thinking = None;
        self.tree = GameTree::new();
        self.human = human;
        self.level = clamp_level(level);
        self.live = true;
        self.refresh_input();
        self.events.push(SessionEvent::Jumped);
    }

    /// Заменяет партию загруженной. Показывается конец основной линии в режиме просмотра.
    pub fn load(&mut self, mut tree: GameTree, human: Option<Color>) {
        self.thinking = None;
        let end = tree.main_line_end();
        tree.go_to(end);
        self.tree = tree;
        if let Some(color) = human {
            self.human = color;
        }
        self.live = false;
        self.refresh_input();
        self.events.push(SessionEvent::Jumped);
    }

    #[must_use]
    pub const fn tree(&self) -> &GameTree {
        &self.tree
    }

    /// Цвет человека.
    #[must_use]
    pub const fn human(&self) -> Color {
        self.human
    }

    /// Цвет компьютера.
    #[must_use]
    pub const fn computer(&self) -> Color {
        self.human.opponent()
    }

    #[must_use]
    pub const fn level(&self) -> u8 {
        self.level
    }

    /// Меняет уровень. Если компьютер думает, расчёт начинается заново на новом уровне.
    pub fn set_level(&mut self, level: u8) {
        let level = clamp_level(level);
        if level != self.level {
            self.level = level;
            self.thinking = None;
        }
    }

    /// Наименьшая пауза перед ответом компьютера, с.
    pub const fn set_min_think(&mut self, seconds: f64) {
        self.min_think = if seconds.is_finite() {
            seconds.max(0.0)
        } else {
            0.0
        };
    }

    /// Идёт игра (а не просмотр истории).
    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.live
    }

    /// Думает ли компьютер прямо сейчас.
    #[must_use]
    pub const fn is_thinking(&self) -> bool {
        self.thinking.is_some()
    }

    /// Результат партии в показанной позиции.
    #[must_use]
    pub fn outcome(&self) -> Option<Outcome> {
        self.tree.current_node().outcome()
    }

    /// Чья очередь хода в показанной позиции.
    #[must_use]
    pub fn side_to_move(&self) -> Color {
        self.tree.current_node().position().side
    }

    /// Может ли человек сделать ход в показанной позиции.
    #[must_use]
    pub fn human_to_move(&self) -> bool {
        self.outcome().is_none() && self.side_to_move() == self.human
    }

    /// Доступна ли команда «Продолжить отсюда»: идёт просмотр, партия в показанной
    /// позиции не закончена, и очередь хода за компьютером.
    #[must_use]
    pub fn can_continue(&self) -> bool {
        !self.live && self.outcome().is_none() && self.side_to_move() != self.human
    }

    /// «Продолжить отсюда»: компьютер делает ход из показанной позиции.
    pub fn continue_from_here(&mut self) -> bool {
        if self.can_continue() {
            self.live = true;
            true
        } else {
            false
        }
    }

    /// Состояние выбора хода мышью.
    #[must_use]
    pub const fn input(&self) -> &MoveInput {
        &self.input
    }

    fn refresh_input(&mut self) {
        if self.human_to_move() {
            self.input
                .set_moves(legal_moves(self.tree.current_node().position()));
        } else {
            self.input.clear();
        }
    }

    /// Щелчок по полю доски.
    pub fn click(&mut self, sq: Square) -> InputEvent {
        let event = self.input.click(sq);
        self.finish_input(event)
    }

    /// Начало перетаскивания шашки с поля.
    pub fn grab(&mut self, sq: Square) -> bool {
        self.input.grab(sq)
    }

    /// Конец перетаскивания над полем.
    pub fn drop_on(&mut self, sq: Square) -> InputEvent {
        let event = self.input.drop_on(sq);
        self.finish_input(event)
    }

    /// Отмена выбора шашки или незаконченного взятия.
    pub fn cancel_input(&mut self) -> InputEvent {
        self.input.cancel()
    }

    fn finish_input(&mut self, event: InputEvent) -> InputEvent {
        if let InputEvent::Move(mv) = event {
            if !self.play_human(&mv) {
                self.refresh_input();
                return InputEvent::None;
            }
        }
        event
    }

    /// Ход человека из показанной позиции. Если ход отличается от записанного
    /// продолжения, создаётся новый вариант. После хода игра продолжается.
    pub fn play_human(&mut self, mv: &Move) -> bool {
        if !self.human_to_move() {
            return false;
        }
        match self.tree.play(mv) {
            Ok(node) => {
                self.thinking = None;
                self.live = true;
                self.refresh_input();
                self.events.push(SessionEvent::Played {
                    node,
                    by_computer: false,
                });
                true
            }
            Err(_) => false,
        }
    }

    /// Переход по истории. Расчёт компьютера прерывается, включается режим просмотра.
    /// Возвращает `true`, если показанная позиция изменилась.
    pub fn navigate(&mut self, nav: Nav) -> bool {
        let before = self.tree.current();
        match nav {
            Nav::Start => self.tree.go_start(),
            Nav::Back => self.tree.back(),
            Nav::Forward => self.tree.forward(),
            Nav::End => self.tree.go_end(),
            Nav::PrevVariation => self.tree.switch_variation(-1),
            Nav::NextVariation => self.tree.switch_variation(1),
            Nav::To(id) => self.tree.go_to(id),
        };
        if self.tree.current() == before {
            return false;
        }
        self.thinking = None;
        self.live = false;
        self.refresh_input();
        self.events.push(SessionEvent::Jumped);
        true
    }

    /// Продвигает сессию: принимает ответ компьютера и при необходимости запускает расчёт.
    /// `now` — монотонное время в секундах.
    pub fn update(&mut self, now: f64) {
        if let Some(thinking) = &mut self.thinking {
            if matches!(thinking.reply, Reply::Pending) {
                match thinking.job.poll() {
                    JobStatus::Running => {}
                    JobStatus::Done(choice) => thinking.reply = Reply::Ready(choice.map(|c| c.mv)),
                    JobStatus::Failed => {
                        // Поток расчёта не ответил: чтобы партия не остановилась,
                        // делается случайный допустимый ход.
                        let legal = self.tree.get(thinking.node).map(|n| legal_moves(n.position()));
                        let fallback = legal.and_then(|moves| self.rng.pick(&moves).copied());
                        thinking.reply = Reply::Ready(fallback);
                    }
                }
            }
            if let Reply::Ready(answer) = thinking.reply {
                if now - thinking.started >= self.min_think {
                    let node = thinking.node;
                    self.thinking = None;
                    if let Some(mv) = answer {
                        if self.live && self.tree.current() == node {
                            if let Ok(next) = self.tree.play(&mv) {
                                self.refresh_input();
                                self.events.push(SessionEvent::Played {
                                    node: next,
                                    by_computer: true,
                                });
                            }
                        }
                    }
                }
            }
        }

        if self.thinking.is_none()
            && self.live
            && self.outcome().is_none()
            && self.side_to_move() != self.human
        {
            let node = self.tree.current();
            let state = *self.tree.current_node().state();
            let history = self.tree.history_before(node);
            let job = SearchJob::start(state, history, self.level, self.rng.next_u64());
            self.thinking = Some(Thinking {
                job,
                node,
                started: now,
                reply: Reply::Pending,
            });
        }
    }

    /// Забирает накопленные события.
    pub fn take_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }
}
