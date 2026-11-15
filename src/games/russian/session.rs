//! The game session: a game between a human and the computer on top of the tree of variations.
//!
//! The session decides whose move it is, starts and cancels the computer's background
//! calculation, and switches between playing and browsing the history. The time comes
//! from outside, so the logic is tested without a window.

use super::input::{InputEvent, MoveInput};
use crate::ai::clamp_level;
use crate::ai::worker::{JobStatus, SearchJob};
use crate::board::{Color, Square};
use crate::moves::{legal_moves, Move};
use crate::rng::Rng;
use crate::rules::Outcome;
use crate::tree::{GameTree, NodeId};

/// Shortest pause before the computer's reply, s: an instant reply looks unnatural.
pub const DEFAULT_MIN_THINK_SECONDS: f64 = 0.6;

/// A step through the history of the game.
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

/// A session event for the interface (animation, sound).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionEvent {
    /// A move was made that led to the node `node`.
    Played { node: NodeId, by_computer: bool },
    /// The position changed without a move: a step through the history, a new or a loaded game.
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

/// A game against the computer.
#[derive(Debug)]
pub struct Session {
    tree: GameTree,
    human: Color,
    level: u8,
    /// Playing (`true`) or browsing the history (`false`). While browsing the computer does
    /// not move.
    live: bool,
    input: MoveInput,
    thinking: Option<Thinking>,
    rng: Rng,
    events: Vec<SessionEvent>,
    min_think: f64,
}

impl Session {
    /// A new game: the human plays `human` against the level `level`.
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

    /// Starts a new game in the same session.
    pub fn new_game(&mut self, human: Color, level: u8) {
        self.thinking = None;
        self.tree = GameTree::new();
        self.human = human;
        self.level = clamp_level(level);
        self.live = true;
        self.refresh_input();
        self.events.push(SessionEvent::Jumped);
    }

    /// Replaces the game with a loaded one. The end of the main line is shown, in browsing mode.
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

    /// The human's colour.
    #[must_use]
    pub const fn human(&self) -> Color {
        self.human
    }

    /// The computer's colour.
    #[must_use]
    pub const fn computer(&self) -> Color {
        self.human.opponent()
    }

    #[must_use]
    pub const fn level(&self) -> u8 {
        self.level
    }

    /// Changes the level. If the computer is thinking, the calculation restarts on the new level.
    pub fn set_level(&mut self, level: u8) {
        let level = clamp_level(level);
        if level != self.level {
            self.level = level;
            self.thinking = None;
        }
    }

    /// Shortest pause before the computer's reply, s.
    pub const fn set_min_think(&mut self, seconds: f64) {
        self.min_think = if seconds.is_finite() {
            seconds.max(0.0)
        } else {
            0.0
        };
    }

    /// Whether the game is being played (not browsed).
    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.live
    }

    /// Whether the computer is thinking right now.
    #[must_use]
    pub const fn is_thinking(&self) -> bool {
        self.thinking.is_some()
    }

    /// Result of the game in the position shown.
    #[must_use]
    pub fn outcome(&self) -> Option<Outcome> {
        self.tree.current_node().outcome()
    }

    /// The side to move in the position shown.
    #[must_use]
    pub fn side_to_move(&self) -> Color {
        self.tree.current_node().position().side
    }

    /// Whether the human can make a move in the position shown.
    #[must_use]
    pub fn human_to_move(&self) -> bool {
        self.outcome().is_none() && self.side_to_move() == self.human
    }

    /// Whether "Continue from here" is available: the history is being browsed, the game
    /// is not over in the position shown, and it is the computer's turn.
    #[must_use]
    pub fn can_continue(&self) -> bool {
        !self.live && self.outcome().is_none() && self.side_to_move() != self.human
    }

    /// "Continue from here": the computer makes a move from the position shown.
    pub fn continue_from_here(&mut self) -> bool {
        if self.can_continue() {
            self.live = true;
            true
        } else {
            false
        }
    }

    /// State of choosing a move with the mouse.
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

    /// A click on a board square.
    pub fn click(&mut self, sq: Square) -> InputEvent {
        let event = self.input.click(sq);
        self.finish_input(event)
    }

    /// The start of dragging a piece from a square.
    pub fn grab(&mut self, sq: Square) -> bool {
        self.input.grab(sq)
    }

    /// The end of dragging over a square.
    pub fn drop_on(&mut self, sq: Square) -> InputEvent {
        let event = self.input.drop_on(sq);
        self.finish_input(event)
    }

    /// Cancels the selection of a piece or an unfinished capture.
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

    /// The human's move from the position shown. If it differs from the recorded
    /// continuation, a new variation is created. After the move the game goes on.
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

    /// A step through the history. The computer's calculation is interrupted and browsing
    /// mode is switched on. Returns `true` if the position shown has changed.
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

    /// Advances the session: takes the computer's reply and starts a calculation when
    /// needed. `now` is monotonic time in seconds.
    pub fn update(&mut self, now: f64) {
        if let Some(thinking) = &mut self.thinking {
            if matches!(thinking.reply, Reply::Pending) {
                match thinking.job.poll() {
                    JobStatus::Running => {}
                    JobStatus::Done(choice) => thinking.reply = Reply::Ready(choice.map(|c| c.mv)),
                    JobStatus::Failed => {
                        // The calculation thread did not answer: so that the game does not
                        // stall, a random legal move is played.
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

    /// Takes the accumulated events.
    pub fn take_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }
}
