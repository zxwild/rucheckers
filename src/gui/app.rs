//! Главное окно: доска, панель ходов, диалоги. Связывает игровую сессию с отрисовкой,
//! анимацией и звуком.

use super::audio::Audio;
use super::board;
use super::files::{with_extension, FileAction, FileJob, FileJobState};
use super::theme;
use eframe::egui::{
    self, pos2, Align, Align2, Color32, Context, CornerRadius, Id, Key, Layout, Margin, Order, Painter, Pos2,
    RichText, Stroke, Ui,
};
use rucheckers::ai::{MAX_LEVEL, MIN_LEVEL};
use rucheckers::board::{squares_of, Color, Kind, Piece, Position, Square};
use rucheckers::i18n::{level_label, outcome_reason, outcome_title, pdn_error_text, tr, Lang, Text};
use rucheckers::moves::Move;
use rucheckers::notation::format_move;
use rucheckers::pdn;
use rucheckers::rng::entropy_seed;
use rucheckers::settings::Settings;
use rucheckers::sound::Sound;
use rucheckers::tree::{GameTree, NodeId};
use rucheckers::ui::anim::{segment_seconds, MoveAnimation};
use rucheckers::ui::geometry::BoardLayout;
use rucheckers::ui::input::InputEvent;
use rucheckers::ui::session::{Nav, Session, SessionEvent};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;

/// Наибольшая и наименьшая ширина панели ходов.
const PANEL_WIDTH: f32 = 352.0;
const PANEL_MIN_WIDTH: f32 = 250.0;
/// Непрозрачность побитой шашки, пока ход не закончен.
const CAPTURED_ALPHA: f32 = 0.42;
/// Пауза между последним ходом партии и звуком её окончания, с.
const END_SOUND_DELAY: f64 = 0.45;

/// Анимация сделанного хода: позиция до хода и движение шашки.
struct Playing {
    anim: MoveAnimation,
    before: Position,
}

/// Движение шашки на промежуточном шаге серийного взятия, сделанном щелчком.
struct Glide {
    from: Square,
    to: Square,
    start: f64,
}

/// Содержимое диалога новой партии.
struct NewGameDraft {
    human: Color,
    level: u8,
}

/// Действие, выбранное в интерфейсе; выполняется после отрисовки кадра.
enum Action {
    AskNewGame,
    StartGame(Color, u8),
    CancelNewGame,
    Open,
    Save,
    ToggleSound,
    TogglePanel,
    OpenSettings,
    CloseSettings,
    SetLang(Lang),
    SetLevel(u8),
    Navigate(Nav),
    Continue,
    DismissResult,
    CloseMessage,
}

/// Приложение.
pub struct CheckersApp {
    settings: Settings,
    settings_path: Option<PathBuf>,
    session: Session,
    /// Партия начата (до первого подтверждения диалога компьютер не ходит).
    started: bool,
    audio: Audio,
    anims: VecDeque<Playing>,
    glide: Option<Glide>,
    dragging: Option<Square>,
    last_input_was_drop: bool,
    /// Отложенные звуки: время и сигнал.
    sounds: Vec<(f64, Sound)>,
    new_game: Option<NewGameDraft>,
    settings_open: bool,
    message: Option<String>,
    file_job: Option<FileJob>,
    dismissed_result: Option<NodeId>,
    scroll_to_current: bool,
}

impl CheckersApp {
    /// Создаёт приложение. `initial_file` — партия, переданная в командной строке.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        settings: Settings,
        settings_path: Option<PathBuf>,
        initial_file: Option<PathBuf>,
    ) -> CheckersApp {
        theme::apply(&cc.egui_ctx);
        let mut app = CheckersApp {
            settings,
            settings_path,
            session: Session::new(settings.human, settings.level, entropy_seed()),
            started: false,
            audio: Audio::new(),
            anims: VecDeque::new(),
            glide: None,
            dragging: None,
            last_input_was_drop: false,
            sounds: Vec::new(),
            // Перед первой партией игрок выбирает цвет и уровень.
            new_game: Some(NewGameDraft {
                human: settings.human,
                level: settings.level,
            }),
            settings_open: false,
            message: None,
            file_job: None,
            dismissed_result: None,
            scroll_to_current: false,
        };
        if let Some(path) = initial_file {
            app.open_file(&path, 0.0);
            if app.started {
                app.new_game = None;
            }
        }
        app
    }

    fn lang(&self) -> Lang {
        self.settings.lang
    }

    fn text(&self, key: Text) -> &'static str {
        tr(self.settings.lang, key)
    }

    fn save_settings(&self) {
        if let Some(path) = &self.settings_path {
            // Невозможность сохранить настройки игре не мешает.
            let _ = self.settings.save(path);
        }
    }

    /// Открыт ли диалог, перекрывающий доску.
    fn modal_open(&self) -> bool {
        self.new_game.is_some() || self.settings_open || self.message.is_some() || self.file_job.is_some()
    }

    fn reset_view(&mut self) {
        self.anims.clear();
        self.glide = None;
        self.dragging = None;
        self.sounds.clear();
        self.dismissed_result = None;
        self.scroll_to_current = true;
    }

    // -----------------------------------------------------------------------
    // События сессии: анимация и звук
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

        // Ход компьютера показывается целиком. У хода человека уже показаны все шаги,
        // кроме последнего, а при перетаскивании — и он тоже.
        let steps = mv.path().len();
        let first_step = if by_computer {
            0
        } else if self.last_input_was_drop {
            steps
        } else {
            steps - 1
        };
        let queued_end = self.anims.back().map(|p| p.anim.start() + p.anim.total_seconds());
        let start = queued_end.map_or(now, |end| end.max(now));
        let anim = MoveAnimation::new(mv, piece, first_step, start);

        let landings = anim.landing_times();
        let mut last_landing = start;
        if landings.is_empty() {
            self.sounds.push((start, step_sound(&mv, steps - 1)));
        }
        for (i, &time) in landings.iter().enumerate() {
            self.sounds.push((time, step_sound(&mv, first_step + i)));
            last_landing = time;
        }
        if let Some(outcome) = outcome {
            let sound = match outcome.winner() {
                Some(winner) if winner == human => Sound::Win,
                Some(_) => Sound::Loss,
                None => Sound::Draw,
            };
            self.sounds.push((last_landing + END_SOUND_DELAY, sound));
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
    // Действия
    // -----------------------------------------------------------------------

    fn apply(&mut self, action: Action, ctx: &Context, now: f64) {
        match action {
            Action::AskNewGame => {
                self.new_game = Some(NewGameDraft {
                    human: self.settings.human,
                    level: self.settings.level,
                });
            }
            Action::StartGame(human, level) => {
                self.new_game = None;
                self.settings.human = human;
                self.settings.level = level;
                self.save_settings();
                self.session.new_game(human, level);
                self.started = true;
                self.process_events(now);
            }
            Action::CancelNewGame => {
                self.new_game = None;
                // Отказ от выбора перед первой партией: играем с сохранёнными настройками.
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
            Action::OpenSettings => self.settings_open = true,
            Action::CloseSettings => self.settings_open = false,
            Action::SetLang(lang) => {
                if lang != self.settings.lang {
                    self.settings.lang = lang;
                    self.save_settings();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title(tr(lang, Text::AppTitle).to_string()));
                }
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

    /// Загружает партию из файла PDN. Нечитаемый или повреждённый файл не меняет
    /// текущую партию: показывается сообщение с причиной.
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
                    self.session.load(game.tree, human);
                    self.started = true;
                    self.process_events(now);
                }
            },
        }
    }

    // -----------------------------------------------------------------------
    // Клавиатура
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
            self.session.cancel_input();
            self.glide = None;
            self.dragging = None;
        }
    }

    // -----------------------------------------------------------------------
    // Доска
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
        if !self.started || self.modal_open() || !self.anims.is_empty() {
            self.dragging = None;
            return;
        }
        let square_at = |point: Option<Pos2>| point.and_then(|p| layout.square_at(p.x, p.y));
        let promoted_before = self.session.input().promoted_so_far();

        if response.drag_started() {
            let origin = ctx.input(|i| i.pointer.press_origin()).or(pointer);
            if let Some(sq) = square_at(origin) {
                if self.session.grab(sq) {
                    self.dragging = Some(sq);
                    self.glide = None;
                }
            }
        }
        if response.drag_stopped() && self.dragging.take().is_some() {
            if let Some(sq) = square_at(pointer) {
                let event = self.session.drop_on(sq);
                self.after_input(event, true, promoted_before, now);
            }
        }
        if response.clicked() {
            match square_at(response.interact_pointer_pos().or(pointer)) {
                Some(sq) => {
                    let event = self.session.click(sq);
                    self.after_input(event, false, promoted_before, now);
                }
                None => {
                    // Щелчок мимо игровых полей снимает выбор, но не прерывает начатое взятие.
                    if !self.session.input().in_progress() {
                        self.session.cancel_input();
                    }
                }
            }
        }
        if response.secondary_clicked() {
            self.session.cancel_input();
            self.glide = None;
            self.dragging = None;
        }
    }

    fn after_input(&mut self, event: InputEvent, via_drop: bool, promoted_before: bool, now: f64) {
        match event {
            InputEvent::Step { from, to } => {
                let landing = if via_drop {
                    now
                } else {
                    now + segment_seconds(from, to)
                };
                self.glide = if via_drop {
                    None
                } else {
                    Some(Glide { from, to, start: now })
                };
                let promoted = self.session.input().promoted_so_far() && !promoted_before;
                self.sounds.push((
                    landing,
                    if promoted {
                        Sound::Promotion
                    } else {
                        Sound::Capture
                    },
                ));
            }
            InputEvent::Move(_) => {
                self.last_input_was_drop = via_drop;
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

        // Законченные анимации убираются из очереди.
        while self.anims.front().is_some_and(|p| p.anim.frame(now).finished) {
            self.anims.pop_front();
        }
        if self
            .glide
            .as_ref()
            .is_some_and(|g| now - g.start >= segment_seconds(g.from, g.to))
        {
            self.glide = None;
        }

        let tree = self.session.tree();
        let node = tree.current_node();
        let input = self.session.input();
        let interactive = self.started && !self.modal_open() && self.anims.is_empty();

        // Что показывать: позицию, скрытое поле (шашка в движении) и саму движущуюся шашку.
        let mut position = *node.position();
        let mut hidden: Option<Square> = None;
        let mut dimmed: u32 = 0;
        let mut dim_alpha = CAPTURED_ALPHA;
        let mut mover: Option<(Pos2, Piece, bool)> = None;
        let mut highlighted: Option<Move> = node.mv().copied();

        if let Some(playing) = self.anims.front() {
            let frame = playing.anim.frame(now);
            let mv = playing.anim.mv();
            position = playing.before;
            hidden = Some(mv.from());
            dimmed = frame.jumped;
            if frame.arrived {
                dim_alpha = CAPTURED_ALPHA * frame.captured_alpha;
            }
            let mut piece = playing.anim.piece();
            if frame.is_king {
                piece.kind = Kind::King;
            }
            let (x, y) = layout.point_at(frame.file, frame.rank);
            mover = Some((pos2(x, y), piece, false));
            highlighted = Some(*mv);
        } else if let Some(origin) = input.origin() {
            if let Some(mut piece) = position.piece_at(origin) {
                if input.promoted_so_far() {
                    piece.kind = Kind::King;
                }
                let current = input.current_square().unwrap_or(origin);
                dimmed = input.captured_so_far();
                let dragged = self.dragging.is_some();
                let at = match (&self.glide, dragged, pointer) {
                    (_, true, Some(p)) => Some(p),
                    (Some(glide), false, _) => {
                        let t = ((now - glide.start) / segment_seconds(glide.from, glide.to)).clamp(0.0, 1.0)
                            as f32;
                        let eased = t * t * (3.0 - 2.0 * t);
                        let a = board::square_center(layout, glide.from);
                        let b = board::square_center(layout, glide.to);
                        Some(a + (b - a) * eased)
                    }
                    _ => None,
                };
                // Шашка рисуется отдельно, если она в движении или уже ушла с исходного поля.
                if at.is_some() || current != origin {
                    hidden = Some(origin);
                    mover = Some((
                        at.unwrap_or(board::square_center(layout, current)),
                        piece,
                        dragged,
                    ));
                }
            }
        }

        // Подсветка последнего хода и выбранной шашки.
        if let Some(mv) = highlighted {
            board::tint_square(painter, layout, mv.from(), theme::LAST_MOVE);
            board::tint_square(painter, layout, mv.to(), theme::LAST_MOVE);
        }
        if interactive {
            if let Some(current) = input.current_square() {
                board::tint_square(painter, layout, current, theme::SELECTED);
            }
        }

        // Неподвижные шашки.
        for color in [Color::White, Color::Black] {
            for sq in squares_of(position.pieces(color)) {
                if hidden == Some(sq) {
                    continue;
                }
                let Some(piece) = position.piece_at(sq) else {
                    continue;
                };
                let alpha = if dimmed & sq.bit() != 0 { dim_alpha } else { 1.0 };
                if alpha > 0.01 {
                    board::draw_piece(
                        painter,
                        board::square_center(layout, sq),
                        layout.cell,
                        piece,
                        alpha,
                        false,
                        ppp,
                    );
                }
            }
        }

        // Подсказки ввода: шашки, обязанные бить, и доступные поля.
        let mut hover_hand = false;
        if interactive && input.is_enabled() {
            let hovered = pointer.and_then(|p| layout.square_at(p.x, p.y));
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
            if let Some(sq) = hovered {
                let over_piece = pointer.is_some_and(|p| {
                    p.distance(board::square_center(layout, sq)) <= board::piece_hit_radius(layout.cell)
                });
                let can_pick = if input.in_progress() {
                    input.current_square() == Some(sq)
                } else {
                    movable.contains(&sq)
                };
                hover_hand = targets.contains(&sq) || (can_pick && over_piece);
            }
        }

        // Отметка очереди хода и индикатор размышления в левых углах рамки.
        let frame_width = layout.frame();
        if self.started && node.outcome().is_none() && frame_width >= 12.0 {
            let side = node.position().side;
            let outer = board::outer_rect(layout);
            let bottom = side == self.session.human();
            let y = if bottom {
                outer.bottom() - frame_width / 2.0
            } else {
                outer.top() + frame_width / 2.0
            };
            let center = pos2(outer.left() + frame_width / 2.0, y);
            board::draw_turn_marker(
                painter,
                center,
                frame_width,
                side,
                self.session.is_thinking(),
                now,
                ppp,
            );
        }

        // Движущаяся шашка — поверх всего.
        if let Some((at, piece, lifted)) = mover {
            board::draw_piece(painter, at, layout.cell, piece, 1.0, lifted, ppp);
        }

        if interactive {
            let dragging = self.dragging.is_some();
            if dragging || hover_hand {
                ctx.set_cursor_icon(board::cursor_for(hover_hand, dragging));
            }
        }
    }

    /// Результат партии поверх доски: итог и причина, включая правило ничьей.
    fn result_banner(&self, ctx: &Context, layout: &BoardLayout, actions: &mut Vec<Action>) {
        let node = self.session.tree().current();
        let Some(outcome) = self.session.outcome() else {
            return;
        };
        if !self.started || !self.anims.is_empty() || self.dismissed_result == Some(node) || self.modal_open()
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
    // Панель ходов
    // -----------------------------------------------------------------------

    fn side_panel(&mut self, ctx: &Context, actions: &mut Vec<Action>) {
        // В узком окне панель сужается, уступая место доске.
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
                // Подпись занимает оставшееся место слева и при нехватке ширины обрезается.
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

    /// Список ходов активной ветки с вариантами. Возвращает `true`, если текущий ход
    /// был показан и к нему выполнена прокрутка.
    fn move_list(&self, ui: &mut Ui, actions: &mut Vec<Action>) -> bool {
        let lang = self.lang();
        let tree = self.session.tree();
        let current = tree.current();
        let line = tree.active_line();
        if line.len() <= 1 {
            ui.label(RichText::new(tr(lang, Text::NoMovesYet)).color(theme::TEXT_WEAK));
            return true;
        }
        let first_index: u32 = if tree.root().position().side == Color::Black {
            1
        } else {
            0
        };
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
            // Варианты ходов этой строки.
            for &id in &row {
                let siblings = tree.siblings(id);
                if siblings.len() < 2 {
                    continue;
                }
                let is_white = tree.get(id).is_some_and(|n| (n.ply() - 1 + first_index) % 2 == 0);
                // Отступ сохраняется и при переносе длинного списка вариантов на новую строку.
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
        // Текущая позиция — начало партии: прокручивать не к чему.
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

    /// Кнопка показа панели, когда панель скрыта и доска занимает всё окно.
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
    // Диалоги
    // -----------------------------------------------------------------------

    fn dialogs(&mut self, ctx: &Context, actions: &mut Vec<Action>) {
        let lang = self.lang();

        if let Some(text) = &self.message {
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
            return;
        }

        if let Some(draft) = &mut self.new_game {
            let mut start = false;
            let modal = modal_window("new_game_dialog").show(ctx, |ui| {
                ui.set_width(330.0);
                ui.heading(tr(lang, Text::NewGame));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(tr(lang, Text::YourColor));
                    ui.radio_value(&mut draft.human, Color::White, tr(lang, Text::White));
                    ui.radio_value(&mut draft.human, Color::Black, tr(lang, Text::Black));
                });
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
                actions.push(Action::StartGame(draft.human, draft.level));
            } else if modal.inner || modal.should_close() {
                actions.push(Action::CancelNewGame);
            }
            return;
        }

        if self.settings_open {
            let modal = modal_window("settings_dialog").show(ctx, |ui| {
                ui.set_width(430.0);
                ui.heading(tr(lang, Text::Settings));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(tr(lang, Text::Language));
                    for value in [Lang::Ru, Lang::En] {
                        if ui.radio(lang == value, value.native_name()).clicked() {
                            actions.push(Action::SetLang(value));
                        }
                    }
                });
                let mut sound = self.settings.sound;
                if ui.checkbox(&mut sound, tr(lang, Text::Sound)).changed() {
                    actions.push(Action::ToggleSound);
                }
                let mut panel = self.settings.panel;
                if ui.checkbox(&mut panel, tr(lang, Text::MovesPanel)).changed() {
                    actions.push(Action::TogglePanel);
                }
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
}

/// Диалог поверх окна с затемнением остального интерфейса.
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

/// Звук шага хода: превращение, взятие или тихий ход.
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
        let mut actions: Vec<Action> = Vec::new();

        self.poll_file_job(now);
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

        // Перерисовка: непрерывно во время анимации и перетаскивания, реже — пока
        // компьютер думает или открыт системный диалог.
        let animating = !self.anims.is_empty() || self.glide.is_some() || self.dragging.is_some();
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
            // Очередь компьютера: следующий кадр запустит расчёт.
            ctx.request_repaint();
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::BACKGROUND.to_normalized_gamma_f32()
    }
}
