//! The main window: the board, the move panel and the dialogs. It ties the game session
//! to drawing, animation and sound.

use super::audio::Audio;
use super::board;
use super::files::{with_extension, FileAction, FileJob, FileJobState};
use super::fonts;
use super::theme;
use eframe::egui::{
    self, pos2, Align, Align2, Color32, Context, CornerRadius, Id, Key, Layout, Margin, Order, Painter, Pos2,
    RichText, Stroke, Ui,
};
use rucheckers::games::anim::{segment_seconds, MoveAnimation, CAPTURED_ALPHA};
use rucheckers::games::difficulty::{MAX_LEVEL, MIN_LEVEL};
use rucheckers::games::input::InputEvent;
use rucheckers::games::record as pdn;
use rucheckers::games::session::{Nav, Session, SessionEvent};
use rucheckers::games::tree::{GameTree, NodeId};
use rucheckers::games::ugolki::{self, HomeSide};
use rucheckers::games::{format_move, outcome_reason, outcome_title};
use rucheckers::games::{squares_of, Color, GameKind, Kind, Move, Piece, Position, Square};
use rucheckers::i18n::{level_label, pdn_error_text, tr, Lang, Text};
use rucheckers::rng::entropy_seed;
use rucheckers::settings::Settings;
use rucheckers::sound::Sound;
use rucheckers::ui::fx::{ease, Discharge};
use rucheckers::ui::geometry::BoardLayout;
use rucheckers::ui::promotion::Promotion;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;

/// The greatest and the smallest width of the move panel.
const PANEL_WIDTH: f32 = 352.0;
const PANEL_MIN_WIDTH: f32 = 250.0;
/// The greatest height of a drop-down list: all ten levels and all twelve languages
/// are visible without scrolling.
const LIST_HEIGHT: f32 = 440.0;
/// Pause between the last move of a game and the sound of its end, s.
const END_SOUND_DELAY: f64 = 0.45;
/// Distance the pointer travels with the button held before the piece follows it, points.
const DRAG_START_DISTANCE: f32 = 6.0;
/// A press released sooner than this is a click on the pressed square wherever the pointer
/// has gone, s: a click made while the mouse still moves selects the piece instead of
/// dropping it on the next square. Dragging a piece to a square takes longer.
const MIN_DROP_SECONDS: f64 = 0.2;

/// Animation of a move that has been made: the position before the move and the travel of
/// the piece.
struct Playing {
    anim: MoveAnimation,
    before: Position,
}

/// Travel of the piece at an intermediate step of a multi-capture made with a click.
struct Glide {
    from: Square,
    to: Square,
    start: f64,
    /// The piece first becomes a king at the end of this glide.
    promotes: bool,
}

/// A press of the primary button on the board, followed until the button comes up.
#[derive(Clone, Copy)]
struct Press {
    /// The square under the pointer when the button went down.
    square: Square,
    /// Where and when the button went down.
    origin: Pos2,
    time: f64,
    /// The piece was selected before the press: released without a drag, the press is a
    /// click on the selected piece and clears the selection.
    was_selected: bool,
}

/// A press made while a move was still being shown. It acts as soon as the board takes
/// input again, so a click that comes a moment early is not lost.
#[derive(Clone, Copy)]
struct EarlyPress {
    at: Pos2,
    /// The button has come up meanwhile: the press is a click.
    released: bool,
}

/// A change of the primary button over the board, in the order of the window events.
#[derive(Clone, Copy, Debug)]
enum Button {
    Down(Pos2),
    Up(Pos2),
}

/// What the board shows in one frame.
struct Scene {
    /// The position whose pieces are drawn.
    position: Position,
    /// The square whose piece is drawn separately because it is on the move.
    hidden: Option<Square>,
    /// The pieces captured so far by an unfinished step-by-step capture.
    dimmed: u64,
    /// The piece on the move: where it is, what it is and whether it is lifted.
    mover: Option<(Pos2, Piece, bool)>,
    /// The move whose first and last squares are highlighted.
    highlighted: Option<Move>,
}

/// Contents of the new game dialog.
#[derive(Clone, Copy)]
struct NewGameDraft {
    game: GameKind,
    home_side: HomeSide,
    human: Color,
    level: u8,
}

/// A special effect that can be switched on or off on its own.
#[derive(Clone, Copy)]
enum EffectKind {
    Lightning,
    Sparks,
    Flashes,
}

/// An action chosen in the interface; it is carried out after the frame is drawn.
#[derive(Clone, Copy)]
enum Action {
    AskNewGame,
    StartGame(GameKind, HomeSide, Color, u8),
    CancelNewGame,
    Open,
    Save,
    ToggleSound,
    TogglePanel,
    OpenSettings,
    CloseSettings,
    SetLang(Lang),
    /// The main switch of the special effects.
    ToggleEffects,
    /// A single special effect.
    ToggleEffect(EffectKind),
    SetLevel(u8),
    SetGamePreference(GameKind),
    SetHomePreference(HomeSide),
    Navigate(Nav),
    Continue,
    DismissResult,
    CloseMessage,
}

/// The application.
#[expect(
    clippy::struct_excessive_bools,
    reason = "the flags are independent facts about the window, not states of one machine"
)]
pub struct CheckersApp {
    settings: Settings,
    settings_path: Option<PathBuf>,
    session: Session,
    /// The game has started (the computer does not move until the first dialog is confirmed).
    started: bool,
    audio: Audio,
    anims: VecDeque<Playing>,
    glide: Option<Glide>,
    dragging: Option<Square>,
    press: Option<Press>,
    early_press: Option<EarlyPress>,
    /// How many jumps of the human's move were already shown on the board when it was completed.
    human_first_step: Option<usize>,
    /// The electric discharges visible on the board now.
    effects: Vec<Discharge>,
    /// Compass roses beneath pieces that have just become kings.
    promotions: Vec<Promotion>,
    /// Delayed sounds: the time and the signal.
    sounds: Vec<(f64, Sound)>,
    new_game: Option<NewGameDraft>,
    settings_open: bool,
    message: Option<String>,
    /// The player has been told that the settings cannot be saved; the message is not
    /// repeated until saving works again.
    save_failure_reported: bool,
    file_job: Option<FileJob>,
    dismissed_result: Option<NodeId>,
    scroll_to_current: bool,
    /// Images of the pieces for the current cell size.
    sprites: Option<board::PieceSprites>,
}

impl CheckersApp {
    /// Creates the application. `initial_file` is a game given on the command line.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        settings: Settings,
        settings_path: Option<PathBuf>,
        initial_file: Option<PathBuf>,
    ) -> Self {
        theme::apply(&cc.egui_ctx);
        fonts::install(&cc.egui_ctx, settings.lang);
        let mut app = Self {
            settings,
            settings_path,
            session: Session::from_position(
                Position::initial_with_home(settings.game, settings.ugolki_home_side),
                settings.human,
                settings.level,
                entropy_seed(),
            ),
            started: false,
            audio: Audio::new(),
            anims: VecDeque::new(),
            glide: None,
            dragging: None,
            press: None,
            early_press: None,
            human_first_step: None,
            effects: Vec::new(),
            promotions: Vec::new(),
            sounds: Vec::new(),
            // Before the first game the player chooses a game, colour and level.
            new_game: Some(NewGameDraft {
                game: settings.game,
                home_side: settings.ugolki_home_side,
                human: settings.human,
                level: settings.level,
            }),
            settings_open: false,
            message: None,
            save_failure_reported: false,
            file_job: None,
            dismissed_result: None,
            scroll_to_current: false,
            sprites: None,
        };
        if let Some(path) = initial_file {
            app.open_file(&path, 0.0);
            if app.started {
                app.new_game = None;
            }
        }
        cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Title(
            tr(app.lang(), app.session.game().text_key()).into(),
        ));
        app
    }

    const fn lang(&self) -> Lang {
        self.settings.lang
    }

    const fn text(&self, key: Text) -> &'static str {
        tr(self.settings.lang, key)
    }

    /// Writes the settings to disk after every change, so that nothing is lost however
    /// the program ends. A failure does not get in the way of the game, but the player
    /// is told about it once: otherwise the settings would silently be gone at the
    /// next start.
    fn save_settings(&mut self) {
        let Some(path) = &self.settings_path else { return };
        match self.settings.save(path) {
            Ok(()) => self.save_failure_reported = false,
            Err(error) if !self.save_failure_reported => {
                self.save_failure_reported = true;
                self.message = Some(format!(
                    "{}:\n{}\n\n{}",
                    self.text(Text::SettingsNotSaved),
                    path.display(),
                    error
                ));
            }
            Err(_) => {}
        }
    }

    /// Whether a dialog covers the board, the system file dialog included.
    const fn modal_open(&self) -> bool {
        self.new_game.is_some() || self.settings_open || self.message.is_some() || self.file_job.is_some()
    }

    fn reset_view(&mut self) {
        self.anims.clear();
        self.effects.clear();
        self.promotions.clear();
        self.glide = None;
        self.dragging = None;
        self.press = None;
        self.early_press = None;
        self.sounds.clear();
        self.dismissed_result = None;
        self.scroll_to_current = true;
    }

    fn cancel_board_input(&mut self) {
        // Cancelling an unfinished capture restores the original man. Its provisional
        // promotion must disappear too, including a rose waiting for a clicked landing.
        let input = self.session.input();
        if input.in_progress() {
            let human = self.session.human();
            self.promotions.retain(|promotion| {
                promotion.color != human || !input.steps().contains(&Square::from(promotion.square))
            });
        }
        self.session.cancel_input();
        self.glide = None;
        self.dragging = None;
        self.press = None;
        self.early_press = None;
    }

    // -----------------------------------------------------------------------
    // Session events: animation and sound
    // -----------------------------------------------------------------------

    fn process_events(&mut self, now: f64) {
        for event in self.session.take_events() {
            match event {
                SessionEvent::Jumped => self.reset_view(),
                SessionEvent::Played { node, by_computer } => self.on_played(node, by_computer, now),
            }
        }
    }

    fn on_played(&mut self, node: NodeId, by_computer: bool, now: f64) {
        let tree = self.session.tree();
        let Some(played) = tree.get(node) else { return };
        let Some(mv) = played.mv().copied() else { return };
        let Some(parent) = played.parent().and_then(|id| tree.get(id)) else {
            return;
        };
        let before = *parent.position();
        let Some(piece) = before.piece_at(mv.from()) else {
            return;
        };
        let outcome = played.outcome();
        let human = self.session.human();

        // A computer move is shown in full. Of a human move only the jumps not yet seen
        // on the board are shown: all the remaining ones if the final square was
        // clicked, and none if the piece was dragged there.
        let steps = mv.path().len();
        let shown = self.human_first_step.take();
        let first_step = if by_computer {
            0
        } else {
            shown.unwrap_or(steps - 1).min(steps)
        };
        let queued_end = self.anims.back().map(|p| p.anim.start() + p.anim.total_seconds());
        let start = queued_end.map_or(now, |end| end.max(now));
        let effects = self.settings.active_effects();
        let anim = MoveAnimation::new(mv, piece, first_step, start).with_discharge(effects.any());
        if !by_computer {
            // The pause before the reply starts once this move has been shown in full.
            self.session.hold_reply_until(start + anim.total_seconds());
        }

        if self.settings.effects_enabled {
            if let Some(promotion) = anim.promotion() {
                self.promotions.push(promotion);
            }
        }

        let landings = anim.landing_times();
        let mut last_landing = start;
        if landings.is_empty() {
            self.sounds.push((start, step_sound(&mv, steps - 1)));
        }
        for (i, &time) in landings.iter().enumerate() {
            self.sounds.push((time, step_sound(&mv, first_step + i)));
            last_landing = time;
        }
        // Several pieces are captured: lightning trails the flying piece and strikes them.
        if let Some(effect) = anim.discharge(human == Color::Black, entropy_seed()) {
            if let (true, Some(time)) = (effects.lightning, anim.first_strike_time()) {
                self.sounds.push((time, Sound::Discharge));
            }
            self.effects.push(effect);
        }
        if let Some(outcome) = outcome {
            let sound = match outcome.winner() {
                Some(winner) if winner == human => Sound::Win,
                Some(_) => Sound::Loss,
                None => Sound::Draw,
            };
            // The sound of the end of the game does not overlap the discharge.
            let after_move = start + anim.total_seconds() + 0.15;
            self.sounds
                .push(((last_landing + END_SOUND_DELAY).max(after_move), sound));
        }

        self.anims.push_back(Playing { anim, before });
        self.glide = None;
        self.dragging = None;
        self.dismissed_result = None;
        self.scroll_to_current = true;
    }

    fn flush_sounds(&mut self, now: f64) {
        let enabled = self.settings.sound;
        let audio = &self.audio;
        self.sounds.retain(|&(time, sound)| {
            if time > now {
                return true;
            }
            if enabled {
                audio.play(sound);
            }
            false
        });
    }

    // -----------------------------------------------------------------------
    // Actions
    // -----------------------------------------------------------------------

    fn start_game(&mut self, draft: NewGameDraft, ctx: &Context, now: f64) {
        self.settings.game = draft.game;
        self.settings.ugolki_home_side = draft.home_side;
        self.settings.human = draft.human;
        self.settings.level = draft.level;
        self.new_game = None;
        self.save_settings();
        self.session.start_game(
            Position::initial_with_home(draft.game, draft.home_side),
            draft.human,
            draft.level,
        );
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(
            tr(self.lang(), draft.game.text_key()).into(),
        ));
        self.started = true;
        self.process_events(now);
    }

    fn apply(&mut self, action: Action, ctx: &Context, now: f64) {
        match action {
            Action::AskNewGame => {
                self.new_game = Some(NewGameDraft {
                    game: self.settings.game,
                    home_side: self.settings.ugolki_home_side,
                    human: self.settings.human,
                    level: self.settings.level,
                });
            }
            Action::StartGame(game, home_side, human, level) => {
                self.start_game(
                    NewGameDraft {
                        game,
                        home_side,
                        human,
                        level,
                    },
                    ctx,
                    now,
                );
            }
            Action::CancelNewGame => {
                self.new_game = None;
                // The choice before the first game is declined: play with the saved settings.
                self.started = true;
            }
            Action::Open => {
                if self.file_job.is_none() {
                    self.file_job = Some(FileJob::start(FileAction::Open, self.lang(), ""));
                }
            }
            Action::Save => {
                if self.file_job.is_none() {
                    let lang = self.lang();
                    let computer = format!("{} ({})", tr(lang, Text::ComputerName), self.session.level());
                    let tags = pdn::standard_tags(self.session.human(), tr(lang, Text::HumanName), &computer);
                    let text = pdn::to_pdn(self.session.tree(), &tags);
                    let name = format!("game-{}.pdn", pdn::today().replace('.', "-"));
                    self.file_job = Some(FileJob::start(FileAction::Save(text), lang, &name));
                }
            }
            Action::ToggleSound => {
                self.settings.sound = !self.settings.sound;
                self.save_settings();
                if self.settings.sound {
                    self.audio.play(Sound::Move);
                }
            }
            Action::TogglePanel => {
                self.settings.panel = !self.settings.panel;
                self.scroll_to_current = true;
                self.save_settings();
            }
            Action::ToggleEffects => {
                self.settings.effects_enabled = !self.settings.effects_enabled;
                self.save_settings();
            }
            Action::ToggleEffect(kind) => {
                let chosen = &mut self.settings.effects;
                match kind {
                    EffectKind::Lightning => chosen.lightning = !chosen.lightning,
                    EffectKind::Sparks => chosen.sparks = !chosen.sparks,
                    EffectKind::Flashes => chosen.flashes = !chosen.flashes,
                }
                self.save_settings();
            }
            Action::OpenSettings => self.settings_open = true,
            Action::CloseSettings => self.settings_open = false,
            Action::SetLang(lang) => {
                if lang != self.settings.lang {
                    self.settings.lang = lang;
                    self.save_settings();
                    fonts::install(ctx, lang);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                        tr(lang, self.session.game().text_key()).to_string(),
                    ));
                }
            }
            Action::SetGamePreference(game) => {
                self.settings.game = game;
                self.save_settings();
            }
            Action::SetHomePreference(side) => {
                self.settings.ugolki_home_side = side;
                self.save_settings();
            }
            Action::SetLevel(level) => {
                self.session.set_level(level);
                self.settings.level = self.session.level();
                self.save_settings();
            }
            Action::Navigate(nav) => {
                self.session.navigate(nav);
                self.process_events(now);
            }
            Action::Continue => {
                self.session.continue_from_here();
            }
            Action::DismissResult => self.dismissed_result = Some(self.session.tree().current()),
            Action::CloseMessage => self.message = None,
        }
    }

    fn poll_file_job(&mut self, now: f64) {
        let Some(job) = &self.file_job else { return };
        let path = match job.poll() {
            FileJobState::Pending => return,
            FileJobState::Cancelled => {
                self.file_job = None;
                return;
            }
            FileJobState::Chosen(path) => path,
        };
        let Some(job) = self.file_job.take() else { return };
        let lang = self.lang();
        match job.action {
            FileAction::Open => self.open_file(&path, now),
            FileAction::Save(text) => {
                let path = with_extension(path);
                if let Err(error) = std::fs::write(&path, text) {
                    self.message = Some(format!(
                        "{}:\n{}\n\n{}",
                        tr(lang, Text::WriteFailed),
                        path.display(),
                        error
                    ));
                }
            }
        }
    }

    /// Loads a game from a PDN file. An unreadable or damaged file does not change the
    /// current game: a message with the reason is shown.
    fn open_file(&mut self, path: &std::path::Path, now: f64) {
        let lang = self.lang();
        match std::fs::read(path) {
            Err(error) => {
                self.message = Some(format!(
                    "{}:\n{}\n\n{}",
                    tr(lang, Text::ReadFailed),
                    path.display(),
                    error
                ));
            }
            Ok(bytes) => match pdn::from_bytes(&bytes) {
                Err(error) => {
                    self.message = Some(format!("{}\n\n{}", path.display(), pdn_error_text(lang, &error)));
                }
                Ok(game) => {
                    let human = game.human_color();
                    self.settings.game = game.tree.root().position().game;
                    if self.settings.game == GameKind::Ugolki {
                        self.settings.ugolki_home_side = game.tree.root().position().home_side;
                    }
                    self.save_settings();
                    self.session.load(game.tree, human);
                    self.started = true;
                    self.process_events(now);
                }
            },
        }
    }

    // -----------------------------------------------------------------------
    // Keyboard
    // -----------------------------------------------------------------------

    fn handle_keys(&mut self, ctx: &Context, actions: &mut Vec<Action>) {
        if self.modal_open() {
            return;
        }
        ctx.input(|input| {
            let command = input.modifiers.command;
            if command {
                if input.key_pressed(Key::N) {
                    actions.push(Action::AskNewGame);
                }
                if input.key_pressed(Key::O) {
                    actions.push(Action::Open);
                }
                if input.key_pressed(Key::S) {
                    actions.push(Action::Save);
                }
                return;
            }
            for (key, nav) in [
                (Key::ArrowLeft, Nav::Back),
                (Key::ArrowRight, Nav::Forward),
                (Key::ArrowUp, Nav::PrevVariation),
                (Key::ArrowDown, Nav::NextVariation),
                (Key::Home, Nav::Start),
                (Key::End, Nav::End),
            ] {
                if input.key_pressed(key) {
                    actions.push(Action::Navigate(nav));
                }
            }
            if input.key_pressed(Key::F9) {
                actions.push(Action::TogglePanel);
            }
            if input.key_pressed(Key::Space) {
                actions.push(Action::Continue);
            }
        });
        if ctx.input(|input| input.key_pressed(Key::Escape)) {
            self.cancel_board_input();
        }
    }

    // -----------------------------------------------------------------------
    // The board
    // -----------------------------------------------------------------------

    fn board_panel(&mut self, ctx: &Context, now: f64, actions: &mut Vec<Action>) {
        let frame = egui::Frame::new()
            .fill(theme::BACKGROUND)
            .inner_margin(Margin::same(10));
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            let area = ui.available_rect_before_wrap();
            let ppp = ctx.pixels_per_point();
            let flipped = self.session.human() == Color::Black;
            let layout = board::layout_for(area, flipped, ppp);
            let response = ui.allocate_rect(area, egui::Sense::click_and_drag());
            let painter = ui.painter_at(area);
            let pointer = ctx.input(|i| i.pointer.latest_pos());

            self.handle_board_input(ctx, &response, &layout, pointer, now);
            self.paint_board(ctx, &painter, &layout, pointer, now);
            self.result_banner(ctx, &layout, actions);
        });
    }

    fn handle_board_input(
        &mut self,
        ctx: &Context,
        response: &egui::Response,
        layout: &BoardLayout,
        pointer: Option<Pos2>,
        now: f64,
    ) {
        // The primary button is read from the window events. egui counts a click only
        // while the pointer stands still, and a press released a frame or two after the
        // pointer moved on is neither a click nor a drag for it: a click made while the
        // mouse was moving used to select nothing.
        let (events, held) = ctx.input(|i| {
            let events: Vec<(Pos2, bool)> = i
                .events
                .iter()
                .filter_map(|event| match *event {
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        ..
                    } => Some((pos, pressed)),
                    _ => None,
                })
                .collect();
            (events, i.pointer.primary_down())
        });
        // Only presses on the board itself count, not on a window or a menu above it.
        let on_board = |pos: Pos2| {
            response.rect.contains(pos) && ctx.layer_id_at(pos).is_none_or(|top| top == response.layer_id)
        };
        let buttons: Vec<Button> = events
            .into_iter()
            .filter_map(|(pos, pressed)| {
                if pressed {
                    on_board(pos).then_some(Button::Down(pos))
                } else {
                    Some(Button::Up(pos))
                }
            })
            .collect();
        self.board_buttons(&buttons, held, layout, pointer, now);
        if response.secondary_clicked() && self.started && !self.modal_open() && !self.animating(now) {
            self.cancel_board_input();
        }
    }

    /// Whether a move is still being animated.
    fn animating(&self, now: f64) -> bool {
        self.anims.iter().any(|playing| !playing.anim.frame(now).finished)
    }

    /// Whether a move or a capture step made with a click is still being shown. A press
    /// waits for the landing: a quick second click must not crown the piece early and
    /// leave the rose behind.
    fn showing_move(&self, now: f64) -> bool {
        self.animating(now)
            || self
                .glide
                .as_ref()
                .is_some_and(|glide| now < glide.start + segment_seconds(glide.from, glide.to))
    }

    /// Applies the primary-button events of one frame. A press acts at once: it selects a
    /// piece, makes the move to a marked square or clears the selection. The release drops
    /// a dragged piece, or clears the selection after a click on the selected piece.
    /// `held` tells whether the button is still down after these events.
    fn board_buttons(
        &mut self,
        buttons: &[Button],
        held: bool,
        layout: &BoardLayout,
        pointer: Option<Pos2>,
        now: f64,
    ) {
        if !self.started || self.modal_open() {
            self.press = None;
            self.early_press = None;
            self.dragging = None;
            return;
        }
        for &button in buttons {
            if self.showing_move(now) {
                match button {
                    Button::Down(at) => self.early_press = Some(EarlyPress { at, released: false }),
                    Button::Up(_) => {
                        if let Some(early) = &mut self.early_press {
                            early.released = true;
                        }
                    }
                }
                continue;
            }
            self.apply_early_press(layout, now);
            match button {
                Button::Down(at) => self.press_at(at, layout, now),
                Button::Up(at) => self.release_at(at, layout, now),
            }
        }
        if !self.showing_move(now) {
            self.apply_early_press(layout, now);
        }
        if let Some(press) = self.press {
            if !held {
                // The button came up where the window could not see it, for instance
                // after switching to another window: nothing is dropped.
                self.press = None;
                self.dragging = None;
            } else if self.dragging.is_none()
                && pointer.is_some_and(|p| p.distance(press.origin) > DRAG_START_DISTANCE)
            {
                self.dragging = Some(press.square);
                self.glide = None;
            }
        }
    }

    fn apply_early_press(&mut self, layout: &BoardLayout, now: f64) {
        if let Some(early) = self.early_press.take() {
            self.press_at(early.at, layout, now);
            if early.released {
                self.release_at(early.at, layout, now);
            }
        }
    }

    /// The primary button went down at `at`.
    fn press_at(&mut self, at: Pos2, layout: &BoardLayout, now: f64) {
        self.press = None;
        self.dragging = None;
        let input = self.session.input();
        let Some(square) = layout.game_square_at(at.x, at.y) else {
            // A press off the playing squares clears the selection but does not break off
            // a capture in progress.
            if !input.in_progress() {
                self.session.cancel_input();
            }
            return;
        };
        if input.current_square() == Some(square) {
            // The selected piece may be dragged on; released in place, it is a click.
            self.press = Some(Press {
                square,
                origin: at,
                time: now,
                was_selected: true,
            });
            return;
        }
        let promoted_before = input.promoted_so_far();
        let steps_before = input.steps().len();
        let event = self.session.click(square);
        let selected = matches!(event, InputEvent::Selected(_));
        self.after_input(event, false, promoted_before, steps_before, now);
        if selected {
            self.press = Some(Press {
                square,
                origin: at,
                time: now,
                was_selected: false,
            });
        }
    }

    /// The primary button came up at `at`.
    fn release_at(&mut self, at: Pos2, layout: &BoardLayout, now: f64) {
        let Some(press) = self.press.take() else {
            return;
        };
        let dragged = self.dragging.take().is_some() && now - press.time >= MIN_DROP_SECONDS;
        let input = self.session.input();
        let promoted_before = input.promoted_so_far();
        let steps_before = input.steps().len();
        if dragged {
            if let Some(square) = layout.game_square_at(at.x, at.y) {
                let event = self.session.drop_on(square);
                self.after_input(event, true, promoted_before, steps_before, now);
            }
        } else if press.was_selected {
            let event = self.session.click(press.square);
            self.after_input(event, false, promoted_before, steps_before, now);
        }
    }

    fn after_input(
        &mut self,
        event: InputEvent,
        via_drop: bool,
        promoted_before: bool,
        steps_before: usize,
        now: f64,
    ) {
        match event {
            InputEvent::Step { from, to } => {
                let landing = if via_drop {
                    now
                } else {
                    now + segment_seconds(from, to)
                };
                let promoted = self.session.input().promoted_so_far() && !promoted_before;
                self.glide = if via_drop {
                    None
                } else {
                    Some(Glide {
                        from,
                        to,
                        start: now,
                        promotes: promoted,
                    })
                };
                if promoted && self.settings.effects_enabled {
                    self.promotions.push(Promotion::new(
                        to.draughts().expect("promotion square"),
                        self.session.human(),
                        landing,
                    ));
                }
                self.sounds.push((
                    landing,
                    if promoted {
                        Sound::Promotion
                    } else {
                        Sound::Capture
                    },
                ));
            }
            InputEvent::Move(mv) => {
                // A dragged piece already stands on the final square; after a click the
                // jumps not yet made step by step remain to be shown.
                self.human_first_step = Some(if via_drop { mv.path().len() } else { steps_before });
                // A shortcut drop can skip the promotion landing altogether. Show the
                // rose under the final king; MoveAnimation omits those skipped steps.
                if via_drop
                    && !promoted_before
                    && self.settings.effects_enabled
                    && mv.promotion_step().is_some_and(|step| step + 1 < mv.path().len())
                {
                    self.promotions.push(Promotion::new(
                        mv.to().draughts().expect("promotion square"),
                        self.session.human(),
                        now,
                    ));
                }
                self.process_events(now);
            }
            InputEvent::Selected(_) | InputEvent::Cleared | InputEvent::None => self.glide = None,
        }
    }

    fn paint_board(
        &mut self,
        ctx: &Context,
        painter: &Painter,
        layout: &BoardLayout,
        pointer: Option<Pos2>,
        now: f64,
    ) {
        let ppp = ctx.pixels_per_point();
        board::draw_board(painter, layout, ppp);
        self.prepare_frame(ctx, layout, now);
        let Some(sprites) = self.sprites.as_ref() else {
            return;
        };
        let interactive = self.started && !self.modal_open() && self.anims.is_empty();
        let scene = self.scene(layout, pointer, now);

        // Highlights: the last move and the selected piece.
        if let Some(mv) = scene.highlighted {
            board::tint_square(painter, layout, mv.from(), theme::LAST_MOVE);
            board::tint_square(painter, layout, mv.to(), theme::LAST_MOVE);
        }
        if interactive {
            if let Some(current) = self.session.input().current_square() {
                board::tint_square(painter, layout, current, theme::SELECTED);
            }
        }

        // The king and all neighbouring pieces occlude the compass rose, including
        // while it fades after the move animation itself has ended.
        if self.settings.effects_enabled {
            for promotion in &self.promotions {
                if let Some(frame) = promotion.frame_at_scale(now, layout.flipped, layout.cell * ppp) {
                    board::draw_promotion(painter, layout, &frame);
                }
            }
        }
        self.paint_resting_pieces(painter, sprites, layout, &scene, now);
        let hover_hand = interactive && self.paint_input_hints(painter, layout, pointer, scene.hidden);
        self.paint_turn_marker(painter, layout, now, ppp);

        // Flashes, the lightning and the halo go under the flying piece; sparks go on
        // top of everything.
        let parts = self.settings.active_effects();
        let frames: Vec<_> = self
            .effects
            .iter()
            .map(|effect| effect.frame(now, parts))
            .collect();
        for frame in &frames {
            board::draw_lightning(painter, layout, frame, ppp);
        }
        if let Some((at, piece, lifted)) = scene.mover {
            board::draw_piece(painter, sprites, at, layout.cell, piece, 1.0, lifted);
        }
        for frame in &frames {
            board::draw_sparks(painter, layout, frame, ppp);
        }

        if interactive {
            let dragging = self.dragging.is_some();
            if dragging || hover_hand {
                ctx.set_cursor_icon(board::cursor_for(hover_hand, dragging));
            }
        }
    }

    /// Brings the per-frame state up to date: piece images for the current cell size,
    /// and no finished animations or burnt-out effects.
    fn prepare_frame(&mut self, ctx: &Context, layout: &BoardLayout, now: f64) {
        // The piece images are rendered anew when the cell size in pixels changes.
        let cell_px = (layout.cell * ctx.pixels_per_point()).round().max(2.0) as usize;
        if self.sprites.as_ref().map(board::PieceSprites::size_px) != Some(cell_px) {
            self.sprites = Some(board::PieceSprites::new(ctx, cell_px));
        }
        while self
            .anims
            .front()
            .is_some_and(|playing| playing.anim.frame(now).finished)
        {
            self.anims.pop_front();
        }
        self.effects.retain(|effect| !effect.is_finished(now));
        self.promotions.retain(|promotion| !promotion.is_finished(now));
        if self
            .glide
            .as_ref()
            .is_some_and(|glide| now - glide.start >= segment_seconds(glide.from, glide.to))
        {
            self.glide = None;
        }
    }

    /// Works out what the board shows in this frame: the position, the piece on the
    /// move and the move to highlight.
    fn scene(&self, layout: &BoardLayout, pointer: Option<Pos2>, now: f64) -> Scene {
        let node = self.session.tree().current_node();
        let input = self.session.input();
        let mut scene = Scene {
            position: *node.position(),
            hidden: None,
            dimmed: 0,
            mover: None,
            highlighted: node.mv().copied(),
        };

        if let Some(playing) = self.anims.front() {
            // A move is being animated: the position before it, with the piece in flight.
            let frame = playing.anim.frame(now);
            let mv = playing.anim.mv();
            let mut piece = playing.anim.piece();
            if frame.is_king {
                piece.kind = Kind::King;
            }
            let (x, y) = layout.point_at(frame.file, frame.rank);
            scene.position = playing.before;
            scene.hidden = Some(mv.from());
            scene.mover = Some((pos2(x, y), piece, false));
            scene.highlighted = Some(*mv);
        } else if let Some(origin) = input.origin() {
            // The player is making a move: the piece follows the pointer, glides to the
            // square of a capture step, or waits on the square it has reached.
            let Some(mut piece) = scene.position.piece_at(origin) else {
                return scene;
            };
            // A clicked capture step may still be gliding to its promotion square.
            // The crown and the rose appear together on landing, not on the click.
            let promotion_in_flight = self.glide.as_ref().is_some_and(|glide| {
                glide.promotes && now < glide.start + segment_seconds(glide.from, glide.to)
            });
            if input.promoted_so_far() && !promotion_in_flight {
                piece.kind = Kind::King;
            }
            let current = input.current_square().unwrap_or(origin);
            scene.dimmed = input.captured_so_far();
            let dragged = self.dragging.is_some();
            let at = match (&self.glide, dragged, pointer) {
                (_, true, Some(pointer)) => Some(pointer),
                (Some(glide), false, _) => {
                    let progress = (now - glide.start) / segment_seconds(glide.from, glide.to);
                    let from = board::square_center(layout, glide.from);
                    let to = board::square_center(layout, glide.to);
                    Some(from + (to - from) * ease(progress) as f32)
                }
                _ => None,
            };
            // The piece is drawn separately while it moves and once it has left its
            // starting square.
            if at.is_some() || current != origin {
                scene.hidden = Some(origin);
                scene.mover = Some((
                    at.unwrap_or_else(|| board::square_center(layout, current)),
                    piece,
                    dragged,
                ));
            }
        }
        scene
    }

    /// Draws the pieces that stand on the board.
    fn paint_resting_pieces(
        &self,
        painter: &Painter,
        sprites: &board::PieceSprites,
        layout: &BoardLayout,
        scene: &Scene,
        now: f64,
    ) {
        for color in [Color::White, Color::Black] {
            for sq in squares_of(scene.position.pieces(color)) {
                if scene.hidden == Some(sq) {
                    continue;
                }
                let Some(piece) = scene.position.piece_at(sq) else {
                    continue;
                };
                // A captured piece: while a move is animated the animation decides how
                // opaque it is; during a step-by-step capture it stands dimmed.
                let alpha = match self.anims.front() {
                    Some(playing) => playing.anim.captured_alpha(sq, now),
                    None if scene.dimmed & sq.bit() != 0 => CAPTURED_ALPHA,
                    None => 1.0,
                };
                if alpha > 0.01 {
                    let center = board::square_center(layout, sq);
                    board::draw_piece(painter, sprites, center, layout.cell, piece, alpha, false);
                }
            }
        }
    }

    /// Draws the input hints: pieces that must capture and the squares a move can go
    /// to. Returns whether the pointer is over something that can be clicked.
    fn paint_input_hints(
        &self,
        painter: &Painter,
        layout: &BoardLayout,
        pointer: Option<Pos2>,
        hidden: Option<Square>,
    ) -> bool {
        let input = self.session.input();
        if !input.is_enabled() {
            return false;
        }
        let hovered = pointer.and_then(|p| layout.game_square_at(p.x, p.y));
        let movable = input.movable();
        if input.must_capture() && !input.in_progress() {
            for &sq in &movable {
                if hidden != Some(sq) {
                    board::draw_obligation_ring(painter, layout, sq);
                }
            }
        }
        let targets = input.targets();
        for &sq in &targets {
            board::draw_target(painter, layout, sq, hovered == Some(sq));
        }
        // The final squares of a multi-capture can be chosen at once.
        let finals = input.final_targets();
        for &sq in &finals {
            board::draw_final_target(painter, layout, sq, hovered == Some(sq));
        }
        let Some(sq) = hovered else {
            return false;
        };
        let over_piece = pointer.is_some_and(|p| {
            p.distance(board::square_center(layout, sq)) <= board::piece_hit_radius(layout.cell)
        });
        let can_pick = if input.in_progress() {
            input.current_square() == Some(sq)
        } else {
            movable.contains(&sq)
        };
        targets.contains(&sq) || finals.contains(&sq) || (can_pick && over_piece)
    }

    /// Draws the side-to-move mark and the thinking indicator in the left corners of
    /// the board frame.
    fn paint_turn_marker(&self, painter: &Painter, layout: &BoardLayout, now: f64, ppp: f32) {
        let node = self.session.tree().current_node();
        let frame_width = layout.frame();
        if !self.started || node.outcome().is_some() || frame_width < 12.0 {
            return;
        }
        let side = node.position().side;
        let outer = board::outer_rect(layout);
        let y = if side == self.session.human() {
            outer.bottom() - frame_width / 2.0
        } else {
            outer.top() + frame_width / 2.0
        };
        let center = pos2(outer.left() + frame_width / 2.0, y);
        let thinking = self.session.is_thinking();
        board::draw_turn_marker(painter, center, frame_width, side, thinking, now, ppp);
    }

    /// The result of the game over the board: the outcome and the reason, the draw rule included.
    fn result_banner(&self, ctx: &Context, layout: &BoardLayout, actions: &mut Vec<Action>) {
        let node = self.session.tree().current();
        let Some(outcome) = self.session.outcome() else {
            return;
        };
        if !self.started
            || !self.anims.is_empty()
            || (self.settings.effects_enabled && !self.promotions.is_empty())
            || self.dismissed_result == Some(node)
            || self.modal_open()
        {
            return;
        }
        let lang = self.lang();
        let center = pos2(
            layout.outer_x + layout.outer_size / 2.0,
            layout.outer_y + layout.outer_size / 2.0,
        );
        let frame = egui::Frame::new()
            .fill(theme::PAPER)
            .stroke(Stroke::new(1.0_f32, theme::BORDER))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::symmetric(22, 16))
            .shadow(egui::Shadow {
                offset: [0, 6],
                blur: 24,
                spread: 0,
                color: Color32::from_black_alpha(90),
            });
        egui::Area::new(Id::new("result_banner"))
            .order(Order::Foreground)
            .pivot(Align2::CENTER_CENTER)
            .fixed_pos(center)
            .show(ctx, |ui| {
                frame.show(ui, |ui| {
                    ui.set_max_width(340.0);
                    ui.vertical_centered(|ui| {
                        ui.heading(outcome_title(lang, &outcome, self.session.human()));
                        ui.add_space(2.0);
                        ui.label(outcome_reason(lang, &outcome));
                        ui.add_space(8.0);
                        if ui.button(tr(lang, Text::NewGame)).clicked() {
                            actions.push(Action::AskNewGame);
                        }
                        if ui
                            .add(egui::Button::new(tr(lang, Text::Close)).frame(false))
                            .clicked()
                        {
                            actions.push(Action::DismissResult);
                        }
                    });
                });
            });
    }

    // -----------------------------------------------------------------------
    // The move panel
    // -----------------------------------------------------------------------

    fn side_panel(&mut self, ctx: &Context, actions: &mut Vec<Action>) {
        // In a narrow window the panel shrinks to leave room for the board.
        let screen = ctx.screen_rect();
        let width = (screen.width() - screen.height())
            .clamp(PANEL_MIN_WIDTH, PANEL_WIDTH)
            .min(screen.width() * 0.5);
        let frame = egui::Frame::new()
            .fill(theme::PANEL)
            .inner_margin(Margin::same(10));
        let mut scrolled = false;
        egui::SidePanel::right("moves_panel")
            .resizable(false)
            .exact_width(width)
            .frame(frame)
            .show(ctx, |ui| {
                self.panel_header(ui, actions);
                ui.separator();
                self.panel_status(ui, actions);
                ui.separator();
                let nav_height = 44.0;
                let list_height = (ui.available_height() - nav_height).max(60.0);
                egui::Frame::new()
                    .fill(theme::PAPER)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(6))
                    .show(ui, |ui| {
                        ui.set_height(list_height - 14.0);
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                scrolled = self.move_list(ui, actions);
                            });
                    });
                ui.add_space(2.0);
                self.panel_navigation(ui, actions);
            });
        if scrolled {
            self.scroll_to_current = false;
        }
    }

    fn panel_header(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let lang = self.lang();
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr(lang, Text::NewGame)).clicked() {
                actions.push(Action::AskNewGame);
            }
            if ui.button(tr(lang, Text::Open)).clicked() {
                actions.push(Action::Open);
            }
            if ui.button(tr(lang, Text::Save)).clicked() {
                actions.push(Action::Save);
            }
        });
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button("⏵")
                    .on_hover_text(format!("{} (F9)", tr(lang, Text::HidePanel)))
                    .clicked()
                {
                    actions.push(Action::TogglePanel);
                }
                if ui.button("⚙").on_hover_text(tr(lang, Text::Settings)).clicked() {
                    actions.push(Action::OpenSettings);
                }
                let (icon, hint) = if self.settings.sound {
                    ("🔊", Text::SoundOn)
                } else {
                    ("🔇", Text::SoundOff)
                };
                if ui.button(icon).on_hover_text(tr(lang, hint)).clicked() {
                    actions.push(Action::ToggleSound);
                }
                // The caption takes the remaining room on the left and is cut off when it
                // does not fit.
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let who = if self.session.human() == Color::White {
                        Text::YouPlayWhite
                    } else {
                        Text::YouPlayBlack
                    };
                    ui.add(egui::Label::new(RichText::new(tr(lang, who)).color(theme::TEXT_WEAK)).truncate());
                });
            });
        });
        ui.horizontal(|ui| {
            ui.label(tr(lang, Text::Level));
            let mut level = self.session.level();
            egui::ComboBox::from_id_salt("level_combo")
                .selected_text(level_label(lang, level))
                .width(ui.available_width() - 8.0)
                .height(LIST_HEIGHT)
                .show_ui(ui, |ui| {
                    for value in MIN_LEVEL..=MAX_LEVEL {
                        ui.selectable_value(&mut level, value, level_label(lang, value));
                    }
                });
            if level != self.session.level() {
                actions.push(Action::SetLevel(level));
            }
        });
    }

    fn panel_status(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let lang = self.lang();
        let session = &self.session;
        ui.label(RichText::new(tr(lang, session.game().text_key())).strong());
        if session.game() == GameKind::Ugolki {
            let pos = session.tree().current_node().position();
            let arrived =
                |color| (pos.pieces(color) & ugolki::home_for(color.opponent(), pos.home_side)).count_ones();
            ui.label(format!(
                "{}: {}/12 · {}: {}/12",
                tr(lang, Text::White),
                arrived(Color::White),
                tr(lang, Text::Black),
                arrived(Color::Black)
            ));
            egui::CollapsingHeader::new(tr(lang, Text::GameRules))
                .id_salt("ugolki_rules")
                .show(ui, |ui| {
                    ui.label(tr(lang, Text::UgolkiHint));
                });
        }
        if let Some(outcome) = session.outcome() {
            ui.label(RichText::new(outcome_title(lang, &outcome, session.human())).strong());
            ui.label(RichText::new(outcome_reason(lang, &outcome)).color(theme::TEXT_WEAK));
            return;
        }
        ui.horizontal(|ui| {
            if session.is_thinking() {
                ui.add(egui::Spinner::new().size(14.0).color(theme::TEXT_WEAK));
                ui.label(tr(lang, Text::Thinking));
            } else if !self.started {
                ui.label(RichText::new(tr(lang, Text::NewGame)).color(theme::TEXT_WEAK));
            } else if session.human_to_move() {
                let key = if session.input().must_capture() {
                    Text::MustCapture
                } else {
                    Text::YourMove
                };
                ui.label(tr(lang, key));
            } else if session.is_live() {
                ui.label(tr(lang, Text::ComputerToMove));
            } else {
                ui.label(RichText::new(format!("{}:", tr(lang, Text::Review))).color(theme::TEXT_WEAK));
                ui.label(tr(lang, Text::ComputerToMove));
            }
        });
        if session.can_continue() && ui.button(tr(lang, Text::ContinueHere)).clicked() {
            actions.push(Action::Continue);
        }
    }

    /// The list of moves of the active branch with variations. Returns `true` if the
    /// current move was shown and scrolled into view.
    fn move_list(&self, ui: &mut Ui, actions: &mut Vec<Action>) -> bool {
        let lang = self.lang();
        let tree = self.session.tree();
        let current = tree.current();
        let line = tree.active_line();
        if line.len() <= 1 {
            ui.label(RichText::new(tr(lang, Text::NoMovesYet)).color(theme::TEXT_WEAK));
            return true;
        }
        let first_index: u32 = u32::from(tree.root().position().side == Color::Black);
        let number_width = 34.0;
        let cell_width =
            ((ui.available_width() - number_width - 2.0 * ui.spacing().item_spacing.x) / 2.0).max(60.0);
        let row_height = 24.0;
        let mut scrolled = false;

        let cell = |ui: &mut Ui, id: NodeId, actions: &mut Vec<Action>, scrolled: &mut bool| {
            let Some(mv) = tree.get(id).and_then(|n| n.mv()) else {
                return;
            };
            let selected = id == current;
            let label = egui::SelectableLabel::new(selected, format_move(mv));
            let response = ui.add_sized([cell_width, row_height], label);
            if response.clicked() {
                actions.push(Action::Navigate(Nav::To(id)));
            }
            if selected && self.scroll_to_current {
                response.scroll_to_me(Some(Align::Center));
                *scrolled = true;
            }
        };

        let mut index = 1;
        while index < line.len() {
            let id = line[index];
            let Some(node) = tree.get(id) else { break };
            let move_index = node.ply() - 1 + first_index;
            let number = move_index / 2 + 1;
            let white_first = move_index % 2 == 0;
            let mut row: Vec<NodeId> = vec![id];
            if white_first && index + 1 < line.len() {
                row.push(line[index + 1]);
            }
            ui.horizontal(|ui| {
                ui.add_sized(
                    [number_width, row_height],
                    egui::Label::new(RichText::new(format!("{number}.")).color(theme::TEXT_WEAK)),
                );
                if !white_first {
                    ui.add_sized(
                        [cell_width, row_height],
                        egui::Label::new(RichText::new("…").color(theme::TEXT_WEAK)),
                    );
                }
                for &id in &row {
                    cell(ui, id, actions, &mut scrolled);
                }
            });
            // The variations of the moves of this row.
            for &id in &row {
                let siblings = tree.siblings(id);
                if siblings.len() < 2 {
                    continue;
                }
                let is_white = tree.get(id).is_some_and(|n| (n.ply() - 1 + first_index) % 2 == 0);
                // The indent is kept when a long list of variations wraps to a new line.
                ui.indent(("variations", id), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let prefix = if is_white {
                            format!("{number}.")
                        } else {
                            format!("{number}…")
                        };
                        let caption = format!("{} {prefix}", tr(lang, Text::Variations));
                        ui.label(RichText::new(caption).small().color(theme::TEXT_WEAK));
                        for &sibling in siblings {
                            let Some(mv) = tree.get(sibling).and_then(|n| n.mv()) else {
                                continue;
                            };
                            let text = RichText::new(format_move(mv)).small();
                            if ui.selectable_label(sibling == id, text).clicked() && sibling != id {
                                actions.push(Action::Navigate(Nav::To(sibling)));
                            }
                        }
                    });
                });
            }
            index += row.len();
        }
        // The current position is the start of the game: there is nothing to scroll to.
        scrolled || current == GameTree::ROOT
    }

    fn panel_navigation(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let lang = self.lang();
        let tree = self.session.tree();
        let at_start = tree.current() == GameTree::ROOT;
        let at_end = tree.current_node().preferred_child().is_none();
        ui.columns(4, |columns| {
            let buttons = [
                ("⏮", Text::ToStart, Nav::Start, !at_start),
                ("◀", Text::StepBack, Nav::Back, !at_start),
                ("▶", Text::StepForward, Nav::Forward, !at_end),
                ("⏭", Text::ToEnd, Nav::End, !at_end),
            ];
            for (column, (icon, hint, nav, enabled)) in columns.iter_mut().zip(buttons) {
                column.vertical_centered_justified(|ui| {
                    let button = egui::Button::new(RichText::new(icon).size(17.0));
                    if ui
                        .add_enabled(enabled, button)
                        .on_hover_text(tr(lang, hint))
                        .clicked()
                    {
                        actions.push(Action::Navigate(nav));
                    }
                });
            }
        });
    }

    /// The button that shows the panel when it is hidden and the board takes the whole window.
    fn panel_toggle(&self, ctx: &Context, actions: &mut Vec<Action>) {
        egui::Area::new(Id::new("panel_toggle"))
            .order(Order::Foreground)
            .anchor(Align2::RIGHT_TOP, [-6.0, 6.0])
            .show(ctx, |ui| {
                let button = egui::Button::new(RichText::new("☰").size(16.0));
                let hint = format!("{} (F9)", self.text(Text::ShowPanel));
                if ui.add(button).on_hover_text(hint).clicked() {
                    actions.push(Action::TogglePanel);
                }
            });
    }

    // -----------------------------------------------------------------------
    // Dialogs
    // -----------------------------------------------------------------------

    /// Shows the dialog that is open, if any. An error message comes first, then the
    /// new game dialog, then the settings.
    fn dialogs(&mut self, ctx: &Context, actions: &mut Vec<Action>) {
        if self.message.is_some() {
            self.message_dialog(ctx, actions);
        } else if self.new_game.is_some() {
            self.new_game_dialog(ctx, actions);
        } else if self.settings_open {
            self.settings_dialog(ctx, actions);
        }
    }

    fn message_dialog(&self, ctx: &Context, actions: &mut Vec<Action>) {
        let Some(text) = &self.message else { return };
        let lang = self.lang();
        let modal = modal_window("message_dialog").show(ctx, |ui| {
            ui.set_max_width(440.0);
            ui.heading(tr(lang, Text::ErrorTitle));
            ui.add_space(4.0);
            ui.label(text.as_str());
            ui.add_space(10.0);
            ui.vertical_centered(|ui| ui.button(tr(lang, Text::Ok)).clicked())
                .inner
        });
        if modal.inner || modal.should_close() {
            actions.push(Action::CloseMessage);
        }
    }

    fn new_game_dialog(&mut self, ctx: &Context, actions: &mut Vec<Action>) {
        let lang = self.lang();
        let Some(draft) = &mut self.new_game else { return };
        let mut start = false;
        let modal = modal_window("new_game_dialog").show(ctx, |ui| {
            ui.set_width(330.0);
            ui.heading(tr(lang, Text::NewGame));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(tr(lang, Text::GameMode));
                for game in GameKind::ALL {
                    ui.radio_value(&mut draft.game, game, tr(lang, game.text_key()));
                }
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(tr(lang, Text::YourColor));
                ui.radio_value(&mut draft.human, Color::White, tr(lang, Text::White));
                ui.radio_value(&mut draft.human, Color::Black, tr(lang, Text::Black));
            });
            if draft.game == GameKind::Ugolki {
                ui.add_space(8.0);
                home_side_choice(ui, lang, &mut draft.home_side);
                ui.add_space(6.0);
                ui.label(
                    RichText::new(tr(lang, Text::UgolkiHint))
                        .small()
                        .color(theme::TEXT_WEAK),
                );
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(tr(lang, Text::Level));
                ui.add(egui::Slider::new(&mut draft.level, MIN_LEVEL..=MAX_LEVEL).show_value(false));
                ui.label(level_label(lang, draft.level));
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new(tr(lang, Text::Start)).strong()).clicked() {
                    start = true;
                }
                ui.button(tr(lang, Text::Cancel)).clicked()
            })
            .inner
        });
        if start || ctx.input(|i| i.key_pressed(Key::Enter)) {
            actions.push(Action::StartGame(
                draft.game,
                draft.home_side,
                draft.human,
                draft.level,
            ));
        } else if modal.inner || modal.should_close() {
            actions.push(Action::CancelNewGame);
        }
    }

    fn settings_dialog(&self, ctx: &Context, actions: &mut Vec<Action>) {
        let lang = self.lang();
        let settings = &self.settings;
        let modal = modal_window("settings_dialog").show(ctx, |ui| {
            ui.set_width(430.0);
            ui.heading(tr(lang, Text::Settings));
            ui.add_space(8.0);
            let mut game = settings.game;
            ui.horizontal(|ui| {
                ui.label(tr(lang, Text::GameMode));
                egui::ComboBox::from_id_salt("game_mode_settings")
                    .selected_text(tr(lang, game.text_key()))
                    .show_ui(ui, |ui| {
                        for value in GameKind::ALL {
                            ui.selectable_value(&mut game, value, tr(lang, value.text_key()));
                        }
                    });
            });
            ui.label(
                RichText::new(tr(lang, Text::GameAppliesNew))
                    .small()
                    .color(theme::TEXT_WEAK),
            );
            if game == GameKind::Ugolki {
                let mut home_side = settings.ugolki_home_side;
                home_side_choice(ui, lang, &mut home_side);
                if home_side != settings.ugolki_home_side {
                    actions.push(Action::SetHomePreference(home_side));
                }
            }
            if game != settings.game {
                actions.push(Action::SetGamePreference(game));
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(tr(lang, Text::Language));
                // Every language is listed under its own name, so the list can be
                // read whatever the current language is.
                let mut chosen = lang;
                egui::ComboBox::from_id_salt("language_combo")
                    .selected_text(lang.native_name())
                    .height(LIST_HEIGHT)
                    .show_ui(ui, |ui| {
                        for value in Lang::ALL {
                            ui.selectable_value(&mut chosen, value, value.native_name());
                        }
                    });
                if chosen != lang {
                    actions.push(Action::SetLang(chosen));
                }
            });
            let mut sound = settings.sound;
            if ui.checkbox(&mut sound, tr(lang, Text::Sound)).changed() {
                actions.push(Action::ToggleSound);
            }
            let mut panel = settings.panel;
            if ui.checkbox(&mut panel, tr(lang, Text::MovesPanel)).changed() {
                actions.push(Action::TogglePanel);
            }

            // Special effects: one switch for all of them and one for each.
            ui.add_space(8.0);
            ui.label(RichText::new(tr(lang, Text::Effects)).strong());
            let mut all = settings.effects_enabled;
            if ui.checkbox(&mut all, tr(lang, Text::EffectsEnabled)).changed() {
                actions.push(Action::ToggleEffects);
            }
            ui.indent("effects_list", |ui| {
                ui.add_enabled_ui(settings.effects_enabled, |ui| {
                    for (kind, text, on) in [
                        (
                            EffectKind::Lightning,
                            Text::EffectLightning,
                            settings.effects.lightning,
                        ),
                        (EffectKind::Sparks, Text::EffectSparks, settings.effects.sparks),
                        (EffectKind::Flashes, Text::EffectFlashes, settings.effects.flashes),
                    ] {
                        let mut value = on;
                        if ui.checkbox(&mut value, tr(lang, text)).changed() {
                            actions.push(Action::ToggleEffect(kind));
                        }
                    }
                });
            });
            ui.add_space(8.0);
            ui.label(RichText::new(tr(lang, Text::Keys)).strong());
            ui.label(
                RichText::new(tr(lang, Text::KeysHelp))
                    .small()
                    .color(theme::TEXT_WEAK),
            );
            ui.add_space(10.0);
            ui.button(tr(lang, Text::Close)).clicked()
        });
        if modal.inner || modal.should_close() {
            actions.push(Action::CloseSettings);
        }
    }
}

/// A dialog over the window that dims the rest of the interface.
fn modal_window(id: &str) -> egui::Modal {
    let frame = egui::Frame::new()
        .fill(theme::PAPER)
        .stroke(Stroke::new(1.0_f32, theme::BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(20, 16))
        .shadow(egui::Shadow {
            offset: [0, 6],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(90),
        });
    egui::Modal::new(Id::new(id))
        .frame(frame)
        .backdrop_color(Color32::from_black_alpha(90))
}

/// The side is relative to the player, independent of their colour.
fn home_side_choice(ui: &mut Ui, lang: Lang, side: &mut HomeSide) {
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(lang, Text::YourHome));
        for value in HomeSide::ALL {
            ui.radio_value(side, value, tr(lang, value.text_key()));
        }
    });
}

/// The sound of a step of a move: a promotion, a capture or a quiet move.
fn step_sound(mv: &Move, step: usize) -> Sound {
    if mv.promotion_step() == Some(step) {
        Sound::Promotion
    } else if mv.is_capture() {
        Sound::Capture
    } else {
        Sound::Move
    }
}

impl eframe::App for CheckersApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let now = ctx.input(|i| i.time);
        let before_game = self.session.game();
        let mut actions: Vec<Action> = Vec::new();

        self.poll_file_job(now);
        if self.session.game() != before_game {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                tr(self.lang(), self.session.game().text_key()).into(),
            ));
        }
        if self.started {
            self.session.update(now);
        }
        self.process_events(now);
        self.handle_keys(ctx, &mut actions);

        if self.settings.panel {
            self.side_panel(ctx, &mut actions);
        }
        self.board_panel(ctx, now, &mut actions);
        if !self.settings.panel {
            self.panel_toggle(ctx, &mut actions);
        }
        self.dialogs(ctx, &mut actions);

        for action in actions {
            self.apply(action, ctx, now);
        }
        self.flush_sounds(now);

        // Repainting: continuous during animation and dragging, less frequent while the
        // computer is thinking or a system dialog is open.
        let animating = !self.anims.is_empty()
            || !self.effects.is_empty()
            || !self.promotions.is_empty()
            || self.glide.is_some()
            || self.dragging.is_some();
        if animating {
            ctx.request_repaint();
        } else if self.session.is_thinking() || self.file_job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(33));
        } else if let Some(next) = self.sounds.iter().map(|&(time, _)| time).reduce(f64::min) {
            ctx.request_repaint_after(Duration::from_secs_f64((next - now).max(0.0)));
        } else if self.started
            && self.session.is_live()
            && !self.session.human_to_move()
            && self.session.outcome().is_none()
        {
            // It is the computer's turn: the next frame starts the calculation.
            ctx.request_repaint();
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::BACKGROUND.to_normalized_gamma_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{vec2, Rect};
    use rucheckers::games::session::DEFAULT_MIN_THINK_SECONDS;
    use rucheckers::games::tree::GameTree;
    use rucheckers::games::Position;

    fn square(name: &str) -> Square {
        Square::parse(name).unwrap()
    }

    fn app(fen: &str, human: Color) -> CheckersApp {
        let cc = eframe::CreationContext::_new_kittest(Context::default());
        let settings = Settings {
            human,
            sound: false,
            ..Settings::default()
        };
        let mut app = CheckersApp::new(&cc, settings, None, None);
        app.session.load(
            GameTree::from_position(Position::from_fen(fen).unwrap()),
            Some(human),
        );
        app.started = true;
        app.new_game = None;
        app.process_events(0.0);
        app
    }

    fn choose(app: &mut CheckersApp, to: &str, via_drop: bool, now: f64) {
        let promoted_before = app.session.input().promoted_so_far();
        let steps_before = app.session.input().steps().len();
        let event = if via_drop {
            app.session.drop_on(square(to))
        } else {
            app.session.click(square(to))
        };
        app.after_input(event, via_drop, promoted_before, steps_before, now);
    }

    #[test]
    fn starting_another_game_remembers_mode_colour_level_and_other_preferences() {
        let path = std::env::temp_dir().join(format!("rucheckers-modes-{}.conf", std::process::id()));
        let ctx = Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let settings = Settings {
            lang: Lang::Ru,
            human: Color::White,
            sound: false,
            panel: false,
            effects_enabled: false,
            ..Settings::default()
        };
        let mut app = CheckersApp::new(&cc, settings, Some(path.clone()), None);
        app.apply(
            Action::StartGame(GameKind::Ugolki, HomeSide::Right, Color::Black, 8),
            &ctx,
            0.0,
        );
        assert_eq!(app.session.game(), GameKind::Ugolki);
        assert_eq!(app.session.human(), Color::Black);
        assert_eq!(app.session.level(), 8);
        let stored = Settings::load(&path, Settings::default());
        assert_eq!(
            stored,
            Settings {
                game: GameKind::Ugolki,
                ugolki_home_side: HomeSide::Right,
                human: Color::Black,
                level: 8,
                ..settings
            }
        );
        let restarted = CheckersApp::new(&cc, stored, None, None);
        let draft = restarted.new_game.unwrap();
        assert_eq!(
            (draft.game, draft.home_side, draft.human, draft.level),
            (GameKind::Ugolki, HomeSide::Right, Color::Black, 8)
        );

        assert_eq!(app.session.tree().root().position().home_side, HomeSide::Right);
        app.apply(Action::SetHomePreference(HomeSide::Left), &ctx, 1.0);
        assert_eq!(app.session.tree().root().position().home_side, HomeSide::Right);
        app.apply(Action::AskNewGame, &ctx, 1.0);
        assert_eq!(app.new_game.as_ref().unwrap().home_side, HomeSide::Left);
        assert_eq!(
            Settings::load(&path, Settings::default()).ugolki_home_side,
            HomeSide::Left
        );
        app.apply(Action::SetGamePreference(GameKind::RussianDraughts), &ctx, 1.0);
        assert_eq!(app.session.game(), GameKind::Ugolki);
        app.apply(Action::AskNewGame, &ctx, 1.0);
        assert_eq!(app.new_game.as_ref().unwrap().game, GameKind::RussianDraughts);
        app.apply(
            Action::StartGame(GameKind::RussianDraughts, HomeSide::Left, Color::White, 8),
            &ctx,
            1.0,
        );
        assert_eq!(app.session.game(), GameKind::RussianDraughts);
        assert_eq!(app.session.level(), 8);
        assert_eq!(app.session.tree().len(), 1);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn opening_an_ugolki_record_switches_the_engine_and_keeps_level_and_preferences() {
        let path = std::env::temp_dir().join(format!("rucheckers-ugolki-{}.pdn", std::process::id()));
        let mut tree = GameTree::from_position(ugolki::initial_with_home(HomeSide::Right));
        let mv = tree.current_node().state().legal_moves()[0];
        tree.play(&mv).unwrap();
        let tags = pdn::standard_tags(Color::Black, "Player", "Computer");
        std::fs::write(&path, pdn::to_pdn(&tree, &tags)).unwrap();
        let ctx = Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx);
        let settings = Settings {
            level: 7,
            sound: false,
            ..Settings::default()
        };
        let app = CheckersApp::new(&cc, settings, None, Some(path.clone()));
        assert!(app.started);
        assert!(app.new_game.is_none());
        assert_eq!(app.session.game(), GameKind::Ugolki);
        assert_eq!(app.session.human(), Color::Black);
        assert_eq!(app.session.level(), 7);
        assert_eq!(app.settings.game, GameKind::Ugolki);
        assert_eq!(app.settings.ugolki_home_side, HomeSide::Right);
        assert!(app.session.tree().same_structure(&tree));
        assert!(!app.session.is_live());
        assert_eq!(app.session.tree().current(), app.session.tree().main_line_end());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn the_reply_waits_a_whole_pause_after_a_long_human_chain_lands() {
        // Three jumps a1-a3-c3-c5 take 0.63 s to show, longer than the pause itself. The
        // first level answers within milliseconds, yet its piece starts a full pause
        // after the human's piece has landed.
        let mut app = app("W:Wd6:Be7,g7", Color::White);
        let position = Position::from_fen_for(GameKind::Ugolki, "W:Wa1,h1:Ba2,b3,c4,h8").unwrap();
        app.session.start_game(position, Color::White, 1);
        app.process_events(0.0);
        assert_eq!(
            app.session.click(square("a1")),
            InputEvent::Selected(square("a1"))
        );
        choose(&mut app, "c5", false, 1.0);
        let human = &app.anims.front().unwrap().anim;
        let landed = human.start() + human.total_seconds();
        assert!((landed - 1.63).abs() < 1e-9, "{landed}");
        app.session.update(1.0);
        std::thread::sleep(Duration::from_millis(400));
        app.session.update(landed + 0.55);
        app.process_events(landed + 0.55);
        assert_eq!(app.anims.len(), 1, "the pause after the landing is not over");
        let now = landed + DEFAULT_MIN_THINK_SECONDS + 0.01;
        for _ in 0..500 {
            app.session.update(now);
            app.process_events(now);
            if app.anims.len() == 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(app.anims.len(), 2, "the computer has answered");
        assert!(app.anims[1].anim.start() - landed >= DEFAULT_MIN_THINK_SECONDS);
    }

    #[test]
    fn clicked_and_dropped_promotions_start_with_the_crown_for_both_colours() {
        for (fen, human, from, to) in [
            ("W:Wg7:BKa1", Color::White, "g7", "h8"),
            ("B:WKh8:Bb2", Color::Black, "b2", "a1"),
        ] {
            for via_drop in [false, true] {
                let mut app = app(fen, human);
                assert!(app.session.grab(square(from)));
                choose(&mut app, to, via_drop, 2.0);
                assert_eq!(app.promotions.len(), 1);
                let rose = app.promotions[0];
                let anim = &app.anims.front().unwrap().anim;
                assert_eq!(Square::from(rose.square), square(to));
                assert_eq!(rose.color, human);
                assert_eq!(rose.start, 2.0 + anim.travel_seconds());
                assert!(anim.frame(rose.start + 0.001).is_king);
                if !via_drop {
                    assert!(!anim.frame(rose.start - 0.001).is_king);
                }
            }
        }
    }

    #[test]
    fn intermediate_promotion_waits_for_landing_and_is_not_replayed() {
        let layout = BoardLayout::fit(0.0, 0.0, 640.0, 640.0, false, 1.0);
        for via_drop in [false, true] {
            let mut app = app("W:Wd6:Be7,g7", Color::White);
            assert!(app.session.grab(square("d6")));
            choose(&mut app, "f8", via_drop, 2.0);
            assert_eq!(app.promotions.len(), 1);
            let rose = app.promotions[0];
            assert_eq!(Square::from(rose.square), square("f8"));
            if !via_drop {
                assert_eq!(
                    app.scene(&layout, None, rose.start - 0.001).mover.unwrap().1.kind,
                    Kind::Man
                );
            }
            assert_eq!(
                app.scene(&layout, None, rose.start).mover.unwrap().1.kind,
                Kind::King
            );
            choose(&mut app, "h6", via_drop, rose.start + 0.1);
            assert_eq!(
                app.promotions,
                vec![rose],
                "finishing the capture must not replay promotion"
            );
        }
    }

    #[test]
    fn dropped_shortcut_shows_promotion_beneath_the_final_king() {
        for (fen, human, from, to) in [
            ("W:Wd6:Be7,g7", Color::White, "d6", "h6"),
            ("B:Wb2,d2:Be3", Color::Black, "e3", "a3"),
        ] {
            let mut app = app(fen, human);
            assert!(app.session.grab(square(from)));
            choose(&mut app, to, true, 2.0);
            assert_eq!(
                app.promotions,
                vec![Promotion::new(square(to).draughts().unwrap(), human, 2.0)]
            );
        }
    }

    #[test]
    fn cancelling_an_unfinished_capture_removes_its_promotion() {
        let mut app = app("W:Wd6:Be7,g7", Color::White);
        assert!(app.session.grab(square("d6")));
        choose(&mut app, "f8", false, 2.0);
        let ctx = Context::default();
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                }],
                ..Default::default()
            },
            |ctx| app.handle_keys(ctx, &mut Vec::new()),
        );
        assert!(
            app.promotions.is_empty(),
            "a cancelled landing must not flash on an empty square"
        );
        assert!(app.session.input().origin().is_none());
    }

    #[test]
    fn a_quick_second_click_cannot_skip_the_promotion_landing() {
        let mut app = app("W:Wd6:Be7,g7", Color::White);
        assert!(app.session.grab(square("d6")));
        choose(&mut app, "f8", false, 2.0);
        let layout = BoardLayout::fit(0.0, 0.0, 640.0, 640.0, false, 1.0);
        let pointer = board::square_center(&layout, square("h6"));
        let ctx = Context::default();
        for (i, pressed) in [None, Some(true), Some(false)].into_iter().enumerate() {
            let mut events = vec![egui::Event::PointerMoved(pointer)];
            if let Some(pressed) = pressed {
                events.push(egui::Event::PointerButton {
                    pos: pointer,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                });
            }
            let now = 2.01 + i as f64 * 0.01;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(640.0, 640.0))),
                    time: Some(now),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let response =
                            ui.interact(ui.max_rect(), Id::new("board"), egui::Sense::click_and_drag());
                        if pressed == Some(false) {
                            assert!(response.clicked());
                        }
                        app.handle_board_input(ctx, &response, &layout, Some(pointer), now);
                    });
                },
            );
        }
        assert_eq!(app.session.input().steps(), &[square("f8")]);
        assert_eq!(app.scene(&layout, None, 2.04).mover.unwrap().1.kind, Kind::Man);
        assert_eq!(app.promotions.len(), 1);
    }

    fn primary(at: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }

    /// One frame of the board with the given window events, as `board_panel` runs it.
    fn board_frame(app: &mut CheckersApp, ctx: &Context, now: f64, events: Vec<egui::Event>) {
        let layout = BoardLayout::fit(0.0, 0.0, 640.0, 640.0, false, 1.0);
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(640.0, 640.0))),
                time: Some(now),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response =
                        ui.interact(ui.max_rect(), Id::new("board"), egui::Sense::click_and_drag());
                    let pointer = ctx.input(|i| i.pointer.latest_pos());
                    app.handle_board_input(ctx, &response, &layout, pointer, now);
                });
            },
        );
    }

    /// The Ugolki start with the human playing White, a context that has already shown
    /// the board once (as in the running window), and the centres of two cells.
    fn ugolki_app(from: &str, to: &str) -> (CheckersApp, Context, Pos2, Pos2) {
        let mut app = app("W:Wd6:Be7,g7", Color::White);
        app.session.start_game(
            Position::initial_with_home(GameKind::Ugolki, HomeSide::Left),
            Color::White,
            1,
        );
        app.process_events(0.0);
        let ctx = Context::default();
        board_frame(&mut app, &ctx, 0.5, Vec::new());
        let layout = BoardLayout::fit(0.0, 0.0, 640.0, 640.0, false, 1.0);
        let at = |name| board::square_center(&layout, square(name));
        (app, ctx, at(from), at(to))
    }

    #[test]
    fn a_click_made_while_the_mouse_moves_selects_the_piece() {
        // The pointer passes over c3, the button goes down and comes up 18 points further
        // on, all within one frame: egui reports neither a click nor a drag for this.
        let (mut app, ctx, c3, _) = ugolki_app("c3", "c4");
        let events = vec![
            egui::Event::PointerMoved(c3 - vec2(15.0, 0.0)),
            primary(c3, true),
            egui::Event::PointerMoved(c3 + vec2(18.0, 0.0)),
            primary(c3 + vec2(18.0, 0.0), false),
        ];
        board_frame(&mut app, &ctx, 1.0, events);
        assert_eq!(app.session.input().origin(), Some(square("c3")));
        assert_eq!(app.session.tree().len(), 1);
    }

    #[test]
    fn a_quick_press_that_slides_onto_a_target_is_a_click_not_a_drop() {
        // Down on c3 and up over c4 a tenth of a second later: a sloppy click, not a move.
        let (mut app, ctx, c3, c4) = ugolki_app("c3", "c4");
        board_frame(
            &mut app,
            &ctx,
            1.0,
            vec![egui::Event::PointerMoved(c3), primary(c3, true)],
        );
        let middle = c3 + (c4 - c3) * 0.5;
        board_frame(&mut app, &ctx, 1.04, vec![egui::Event::PointerMoved(middle)]);
        assert_eq!(app.dragging, Some(square("c3")), "the piece follows the pointer");
        board_frame(
            &mut app,
            &ctx,
            1.1,
            vec![egui::Event::PointerMoved(c4), primary(c4, false)],
        );
        assert_eq!(app.session.tree().len(), 1, "no move was made");
        assert_eq!(app.session.input().origin(), Some(square("c3")));
        assert_eq!(app.dragging, None);
    }

    #[test]
    fn a_deliberate_drag_still_moves_the_piece() {
        let (mut app, ctx, c3, c4) = ugolki_app("c3", "c4");
        board_frame(
            &mut app,
            &ctx,
            1.0,
            vec![egui::Event::PointerMoved(c3), primary(c3, true)],
        );
        board_frame(
            &mut app,
            &ctx,
            1.1,
            vec![egui::Event::PointerMoved(c3 + (c4 - c3) * 0.5)],
        );
        board_frame(
            &mut app,
            &ctx,
            1.3,
            vec![egui::Event::PointerMoved(c4), primary(c4, false)],
        );
        assert_eq!(app.session.tree().len(), 2);
        let mv = *app.session.tree().current_node().mv().unwrap();
        assert_eq!((mv.from(), mv.to()), (square("c3"), square("c4")));
    }

    #[test]
    fn a_press_acts_at_once_and_a_click_on_the_selected_piece_clears_it() {
        let (mut app, ctx, c3, c4) = ugolki_app("c3", "c4");
        board_frame(
            &mut app,
            &ctx,
            1.0,
            vec![egui::Event::PointerMoved(c3), primary(c3, true)],
        );
        assert_eq!(
            app.session.input().origin(),
            Some(square("c3")),
            "selected while the button is still down"
        );
        board_frame(&mut app, &ctx, 1.05, vec![primary(c3, false)]);
        board_frame(&mut app, &ctx, 1.5, vec![primary(c3, true)]);
        assert_eq!(
            app.session.input().origin(),
            Some(square("c3")),
            "kept for a drag"
        );
        board_frame(&mut app, &ctx, 1.55, vec![primary(c3, false)]);
        assert_eq!(
            app.session.input().origin(),
            None,
            "a click on it clears the selection"
        );

        board_frame(&mut app, &ctx, 2.0, vec![primary(c3, true), primary(c3, false)]);
        board_frame(
            &mut app,
            &ctx,
            2.5,
            vec![egui::Event::PointerMoved(c4), primary(c4, true)],
        );
        assert_eq!(
            app.session.tree().len(),
            2,
            "the move to a marked cell is made on the press"
        );
    }

    #[test]
    fn a_press_during_a_move_animation_acts_once_the_piece_has_landed() {
        let (mut app, ctx, c3, _) = ugolki_app("c3", "c4");
        let before = *app.session.tree().current_node().position();
        let mv = app.session.input().moves()[0];
        let piece = before.piece_at(mv.from()).unwrap();
        app.anims.push_back(Playing {
            anim: MoveAnimation::new(mv, piece, 0, 1.0),
            before,
        });
        board_frame(&mut app, &ctx, 1.05, vec![primary(c3, true), primary(c3, false)]);
        assert_eq!(app.session.input().origin(), None, "the piece is still in flight");
        board_frame(&mut app, &ctx, 1.1, Vec::new());
        assert_eq!(app.session.input().origin(), None);
        board_frame(&mut app, &ctx, 1.3, Vec::new());
        assert_eq!(app.session.input().origin(), Some(square("c3")));
    }

    #[test]
    fn a_click_during_a_capture_step_glide_is_kept_for_the_landing() {
        let mut app = app("W:Wd6:Be7,g7", Color::White);
        assert!(app.session.grab(square("d6")));
        choose(&mut app, "f8", false, 2.0);
        let layout = BoardLayout::fit(0.0, 0.0, 640.0, 640.0, false, 1.0);
        let h6 = board::square_center(&layout, square("h6"));
        let ctx = Context::default();
        board_frame(&mut app, &ctx, 2.01, Vec::new());
        board_frame(
            &mut app,
            &ctx,
            2.05,
            vec![egui::Event::PointerMoved(h6), primary(h6, true)],
        );
        board_frame(&mut app, &ctx, 2.08, vec![primary(h6, false)]);
        assert_eq!(
            app.session.input().steps(),
            &[square("f8")],
            "the landing comes first"
        );
        board_frame(&mut app, &ctx, 2.3, Vec::new());
        assert_eq!(app.session.tree().len(), 2, "then the second jump is made");
        assert_eq!(app.session.tree().current_node().mv().unwrap().to(), square("h6"));
    }

    #[test]
    fn presses_beside_the_board_or_over_a_window_do_not_reach_it() {
        let (mut app, ctx, c3, _) = ugolki_app("c3", "c4");
        let layout = BoardLayout::fit(0.0, 0.0, 640.0, 640.0, false, 1.0);
        // The first frame only shows the window: egui finds it from the next frame on.
        for (now, click) in [(1.0, false), (1.1, true)] {
            let mut events = vec![egui::Event::PointerMoved(c3)];
            if click {
                events.extend([primary(c3, true), primary(c3, false)]);
            }
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(640.0, 640.0))),
                    time: Some(now),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::Window::new("over the board")
                        .fixed_pos(c3 - vec2(40.0, 40.0))
                        .fixed_size(vec2(80.0, 80.0))
                        .show(ctx, |ui| ui.label("…"));
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let response =
                            ui.interact(ui.max_rect(), Id::new("board"), egui::Sense::click_and_drag());
                        let pointer = ctx.input(|i| i.pointer.latest_pos());
                        app.handle_board_input(ctx, &response, &layout, pointer, now);
                    });
                },
            );
        }
        assert_eq!(app.session.input().origin(), None, "the window took the click");
        let away = Pos2::new(c3.x, 700.0);
        board_frame(
            &mut app,
            &ctx,
            2.0,
            vec![primary(away, true), primary(away, false)],
        );
        assert_eq!(app.session.input().origin(), None);
    }

    #[test]
    fn effects_switch_and_view_reset_control_promotion() {
        let mut app = app("W:Wg7:BKa1", Color::White);
        app.settings.effects_enabled = false;
        assert!(app.session.grab(square("g7")));
        choose(&mut app, "h8", true, 2.0);
        assert!(app.promotions.is_empty());
        app.promotions.push(Promotion::new(
            square("h8").draughts().unwrap(),
            Color::White,
            2.0,
        ));
        app.reset_view();
        assert!(app.promotions.is_empty());
    }
}
