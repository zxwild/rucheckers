//! The side panel: a tab with the game being played and a tab with the settings.
//!
//! The tab bar carries the button that hides the panel. Both tabs scroll when their
//! contents do not fit: the game tab keeps the navigation buttons in view and gives the
//! move list the room that is left.

use super::{home_side_radios, Action, CheckersApp, EffectKind};
use crate::gui::theme;
use eframe::egui::emath::GuiRounding as _;
use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, Context, CornerRadius, FontId, Id, Layout, Margin, Order,
    Painter, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, TextStyle, Ui, UiBuilder, Vec2, WidgetInfo,
    WidgetType,
};
use rucheckers::games::difficulty::{MAX_LEVEL, MIN_LEVEL};
use rucheckers::games::session::Nav;
use rucheckers::games::tree::{GameTree, NodeId};
use rucheckers::games::ugolki;
use rucheckers::games::{format_move, outcome_reason, outcome_title, Color, GameKind};
use rucheckers::i18n::{level_label, tr, Lang, Text};
use rucheckers::ui::wood::BoardStyle;

/// The greatest and the smallest width of the panel.
const PANEL_WIDTH: f32 = 352.0;
const PANEL_MIN_WIDTH: f32 = 250.0;
/// The greatest height of a drop-down list: all ten levels and all twelve languages
/// are visible without scrolling.
const LIST_HEIGHT: f32 = 440.0;
/// Height of the tab bar, of the row of navigation buttons and the least height of the
/// move list, points.
const TAB_BAR_HEIGHT: f32 = 34.0;
const NAV_HEIGHT: f32 = 30.0;
const MOVE_LIST_MIN_HEIGHT: f32 = 150.0;
/// Room on each side of the name of a tab, points.
const TAB_PADDING: f32 = 10.0;
/// The least width of a control beside its name; a narrower panel puts the name above.
const CONTROL_MIN_WIDTH: f32 = 150.0;
/// Room on the right of the settings for the scroll bar, points.
const SCROLL_BAR_ROOM: f32 = 8.0;
/// The font of the names on the keys in the table of keys.
const KEYCAP_FONT_SIZE: f32 = 13.0;

/// The tabs of the side panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum PanelTab {
    /// The game being played: its state and its moves.
    #[default]
    Game,
    Settings,
}

impl PanelTab {
    const ALL: [Self; 2] = [Self::Game, Self::Settings];

    const fn text_key(self) -> Text {
        match self {
            Self::Game => Text::GameTab,
            Self::Settings => Text::Settings,
        }
    }
}

impl CheckersApp {
    pub(super) fn side_panel(&mut self, ctx: &Context, actions: &mut Vec<Action>) {
        // In a narrow window the panel shrinks to leave room for the board.
        let screen = ctx.screen_rect();
        let width = (screen.width() - screen.height())
            .clamp(PANEL_MIN_WIDTH, PANEL_WIDTH)
            .min(screen.width() * 0.5);
        let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(Margin {
            left: 10,
            right: 10,
            top: 4,
            bottom: 10,
        });
        let mut scrolled = false;
        egui::SidePanel::right("moves_panel")
            .resizable(false)
            .exact_width(width)
            .frame(frame)
            .show(ctx, |ui| {
                self.tab_bar(ui, actions);
                ui.add_space(10.0);
                match self.tab {
                    PanelTab::Game => scrolled = self.game_tab(ui, actions),
                    PanelTab::Settings => self.settings_tab(ui, actions),
                }
            });
        if scrolled {
            self.scroll_to_current = false;
        }
    }

    /// The names of the tabs on the left and the button that hides the panel on the right.
    fn tab_bar(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let lang = self.lang();
        let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), TAB_BAR_HEIGHT), Sense::hover());
        let painter = ui.painter().clone();
        let base = painter.round_to_pixel_center(bar.bottom() - 0.5);
        painter.hline(bar.x_range(), base, Stroke::new(1.0_f32, theme::BORDER));

        let font = TextStyle::Button.resolve(ui.style());
        let mut left = bar.left();
        for tab in PanelTab::ALL {
            let name = tr(lang, tab.text_key());
            let selected = self.tab == tab;
            let galley = painter.layout_no_wrap(name.to_owned(), font.clone(), Color32::PLACEHOLDER);
            let rect = Rect::from_min_size(
                pos2(left, bar.top()),
                vec2(galley.size().x + 2.0 * TAB_PADDING, bar.height()),
            );
            let response = ui.interact(rect, ui.id().with(("panel_tab", tab.text_key())), Sense::click());
            response.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, name));
            let color = if selected || response.hovered() {
                theme::TEXT
            } else {
                theme::TEXT_WEAK
            };
            let text_top = rect.center().y - galley.size().y / 2.0 - 1.0;
            painter.galley(pos2(rect.left() + TAB_PADDING, text_top), galley, color);
            if selected {
                let underline = Rect::from_min_max(
                    pos2(rect.left() + 2.0, bar.bottom() - 2.5),
                    pos2(rect.right() - 2.0, bar.bottom()),
                );
                painter.rect_filled(underline, CornerRadius::same(1), theme::FRAME);
            }
            if response.clicked() && !selected {
                actions.push(Action::ShowTab(tab));
            }
            left = rect.right() + 2.0;
        }

        let hide = tr(lang, Text::HidePanel);
        let rect = Rect::from_center_size(pos2(bar.right() - 16.0, bar.center().y - 1.0), vec2(32.0, 28.0));
        let response = ui.interact(rect, ui.id().with("hide_panel"), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, hide));
        if response.hovered() {
            painter.rect_filled(
                rect,
                CornerRadius::same(6),
                ui.visuals().widgets.hovered.weak_bg_fill,
            );
        }
        let color = if response.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_WEAK
        };
        paint_panel_icon(&painter, rect.center(), color, true);
        if response.on_hover_text(format!("{hide} (F9)")).clicked() {
            actions.push(Action::TogglePanel);
        }
    }

    /// The button that shows the panel when it is hidden and the board takes the whole window.
    pub(super) fn panel_toggle(&self, ctx: &Context, actions: &mut Vec<Action>) {
        egui::Area::new(Id::new("panel_toggle"))
            .order(Order::Foreground)
            .anchor(Align2::RIGHT_TOP, [-8.0, 8.0])
            .show(ctx, |ui| {
                let response = ui.add(egui::Button::new("").min_size(vec2(36.0, 30.0)));
                let color = ui.style().interact(&response).fg_stroke.color;
                paint_panel_icon(ui.painter(), response.rect.center(), color, false);
                let hint = format!("{} (F9)", self.text(Text::ShowPanel));
                if response.on_hover_text(hint).clicked() {
                    actions.push(Action::TogglePanel);
                }
            });
    }

    // -----------------------------------------------------------------------
    // The game tab
    // -----------------------------------------------------------------------

    /// The game being played: the file buttons, what the game is, its state, the moves
    /// and the navigation. Returns `true` if the current move was scrolled into view.
    fn game_tab(&self, ui: &mut Ui, actions: &mut Vec<Action>) -> bool {
        let area = ui.available_rect_before_wrap();
        let nav_top = area.bottom() - NAV_HEIGHT;
        let middle = Rect::from_min_max(area.min, pos2(area.right(), nav_top - 8.0));
        let mut scrolled = false;
        ui.scope_builder(UiBuilder::new().max_rect(middle), |ui| {
            let viewport = ui.available_height();
            egui::ScrollArea::vertical()
                .id_salt("game_tab")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let top = ui.cursor().top();
                    self.file_buttons(ui, actions);
                    ui.add_space(8.0);
                    self.game_summary(ui);
                    ui.add_space(8.0);
                    self.game_status(ui, actions);
                    ui.add_space(4.0);
                    // The list takes the room that is left. When there is too little, it
                    // keeps a useful height and the whole tab scrolls.
                    let used = ui.cursor().top() - top;
                    let height = (viewport - used - 1.0).max(MOVE_LIST_MIN_HEIGHT);
                    scrolled = self.move_list_frame(ui, height, actions);
                });
        });
        let nav = Rect::from_min_max(pos2(area.left(), nav_top), area.max);
        ui.scope_builder(UiBuilder::new().max_rect(nav), |ui| {
            self.panel_navigation(ui, actions);
        });
        scrolled
    }

    fn file_buttons(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let lang = self.lang();
        ui.horizontal_wrapped(|ui| {
            for (key, letter, action) in [
                (Text::NewGame, 'N', Action::AskNewGame),
                (Text::Open, 'O', Action::Open),
                (Text::Save, 'S', Action::Save),
            ] {
                if ui
                    .button(tr(lang, key))
                    .on_hover_text(shortcut(lang, letter))
                    .clicked()
                {
                    actions.push(action);
                }
            }
        });
    }

    /// What is being played: the game, the player's side, the level and, in Ugolki, how
    /// far each side has come and the rules.
    fn game_summary(&self, ui: &mut Ui) {
        let lang = self.lang();
        let session = &self.session;
        ui.label(RichText::new(tr(lang, session.game().text_key())).size(18.0));
        let side = if session.human() == Color::White {
            Text::YouPlayWhite
        } else {
            Text::YouPlayBlack
        };
        let level = format!("{} {}", tr(lang, Text::Level), level_label(lang, session.level()));
        for line in [tr(lang, side), level.as_str()] {
            ui.label(RichText::new(line).color(theme::TEXT_WEAK));
        }
        if session.game() == GameKind::Ugolki {
            ui.add_space(6.0);
            self.ugolki_progress(ui);
            egui::CollapsingHeader::new(tr(lang, Text::GameRules))
                .id_salt("ugolki_rules")
                .show(ui, |ui| {
                    ui.label(RichText::new(tr(lang, Text::UgolkiHint)).color(theme::TEXT_WEAK));
                });
        }
    }

    /// Ugolki: how many pieces of each side stand in the opposite home.
    fn ugolki_progress(&self, ui: &mut Ui) {
        let lang = self.lang();
        let position = self.session.tree().current_node().position();
        ui.label(RichText::new(tr(lang, Text::UgolkiArrived)).color(theme::TEXT_WEAK));
        let sides = [(Color::White, Text::White), (Color::Black, Text::Black)];
        let body = TextStyle::Body.resolve(ui.style());
        let name_width = sides
            .iter()
            .map(|&(_, key)| text_width(ui, tr(lang, key), &body))
            .fold(0.0, f32::max);
        for (color, key) in sides {
            let target = ugolki::home_for(color.opponent(), position.home_side);
            let arrived = (position.pieces(color) & target).count_ones();
            let total = target.count_ones();
            let count = format!("{arrived}/{total}");
            ui.horizontal(|ui| {
                fixed_width(ui, name_width, |ui| {
                    ui.label(tr(lang, key));
                });
                let count_width = text_width(ui, &format!("{total}/{total}"), &body);
                let bar_width = (ui.available_width() - count_width - ui.spacing().item_spacing.x).max(24.0);
                let (rect, _) = ui.allocate_exact_size(vec2(bar_width, 8.0), Sense::hover());
                paint_progress(ui.painter(), rect, arrived, total);
                ui.label(count);
            });
        }
    }

    /// Whose move it is and what the player can do, or the result of a finished game.
    fn game_status(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let lang = self.lang();
        let session = &self.session;
        if let Some(outcome) = session.outcome() {
            ui.label(RichText::new(outcome_title(lang, &outcome, session.human())).size(16.5));
            ui.label(RichText::new(outcome_reason(lang, &outcome)).color(theme::TEXT_WEAK));
            return;
        }
        ui.horizontal(|ui| {
            if session.is_thinking() {
                ui.add(egui::Spinner::new().size(14.0).color(theme::TEXT_WEAK));
                ui.label(tr(lang, Text::Thinking));
                return;
            }
            side_disc(ui, session.side_to_move());
            if !self.started {
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

    /// The move list on a sheet of its own height. Returns `true` if the current move was
    /// scrolled into view.
    fn move_list_frame(&self, ui: &mut Ui, height: f32, actions: &mut Vec<Action>) -> bool {
        let mut scrolled = false;
        egui::Frame::new()
            .fill(theme::PAPER)
            .stroke(Stroke::new(1.0_f32, theme::BORDER))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(6))
            .show(ui, |ui| {
                ui.set_height(height - 14.0);
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical()
                    .id_salt("move_list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        scrolled = self.move_list(ui, actions);
                    });
            });
        scrolled
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
                    let button =
                        egui::Button::new(RichText::new(icon).size(17.0)).min_size(vec2(0.0, NAV_HEIGHT));
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

    // -----------------------------------------------------------------------
    // The settings tab
    // -----------------------------------------------------------------------

    /// The settings. Every change applies and is saved at once.
    fn settings_tab(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let lang = self.lang();
        egui::ScrollArea::vertical()
            .id_salt("settings_tab")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - SCROLL_BAR_ROOM);
                let body = TextStyle::Body.resolve(ui.style());
                let names = [
                    Text::Level,
                    Text::GameMode,
                    Text::YourHome,
                    Text::Language,
                    Text::Board,
                ];
                let name_width = names
                    .iter()
                    .map(|&key| text_width(ui, tr(lang, key), &body))
                    .fold(0.0, f32::max)
                    + 10.0;

                section(ui, tr(lang, Text::ComputerName), true);
                self.computer_settings(ui, name_width, actions);
                section(ui, tr(lang, Text::NewGame), false);
                self.new_game_settings(ui, name_width, actions);
                section(ui, tr(lang, Text::InterfaceSection), false);
                self.interface_settings(ui, name_width, actions);
                section(ui, tr(lang, Text::Effects), false);
                self.effect_settings(ui, actions);
                section(ui, tr(lang, Text::Keys), false);
                keys_table(ui, lang);
            });
    }

    /// The level of the computer and, for Ugolki, its style. Both apply to the game
    /// being played.
    fn computer_settings(&self, ui: &mut Ui, name_width: f32, actions: &mut Vec<Action>) {
        let lang = self.lang();
        setting_row(ui, name_width, tr(lang, Text::Level), |ui| {
            let mut level = self.session.level();
            egui::ComboBox::from_id_salt("level_combo")
                .selected_text(level_label(lang, level))
                .width(ui.available_width())
                .height(LIST_HEIGHT)
                .truncate()
                .show_ui(ui, |ui| {
                    for value in MIN_LEVEL..=MAX_LEVEL {
                        ui.selectable_value(&mut level, value, level_label(lang, value));
                    }
                });
            if level != self.session.level() {
                actions.push(Action::SetLevel(level));
            }
        });
        // The style matters for the game being played or chosen for the next one.
        if self.settings.game == GameKind::Ugolki || self.session.game() == GameKind::Ugolki {
            ui.add_space(4.0);
            ui.label(RichText::new(tr(lang, Text::Ugolki)).color(theme::TEXT_WEAK));
            let mut block = self.settings.computer_style.block_home_exit;
            if ui.checkbox(&mut block, tr(lang, Text::BlockHomeExit)).changed() {
                actions.push(Action::ToggleBlockHomeExit);
            }
            ui.indent("block_home_exit_hint", |ui| {
                ui.label(
                    RichText::new(tr(lang, Text::BlockHomeExitHint))
                        .small()
                        .color(theme::TEXT_WEAK),
                );
            });
        }
    }

    /// The game and, for Ugolki, the home side that the new game dialog offers first.
    fn new_game_settings(&self, ui: &mut Ui, name_width: f32, actions: &mut Vec<Action>) {
        let lang = self.lang();
        let mut game = self.settings.game;
        setting_row(ui, name_width, tr(lang, Text::GameMode), |ui| {
            egui::ComboBox::from_id_salt("game_mode_settings")
                .selected_text(tr(lang, game.text_key()))
                .width(ui.available_width())
                .truncate()
                .show_ui(ui, |ui| {
                    for value in GameKind::ALL {
                        ui.selectable_value(&mut game, value, tr(lang, value.text_key()));
                    }
                });
        });
        if game != self.settings.game {
            actions.push(Action::SetGamePreference(game));
        }
        if game == GameKind::Ugolki {
            let mut home_side = self.settings.ugolki_home_side;
            setting_row(ui, name_width, tr(lang, Text::YourHome), |ui| {
                home_side_radios(ui, lang, &mut home_side);
            });
            if home_side != self.settings.ugolki_home_side {
                actions.push(Action::SetHomePreference(home_side));
            }
        }
    }

    fn interface_settings(&self, ui: &mut Ui, name_width: f32, actions: &mut Vec<Action>) {
        let lang = self.lang();
        let settings = &self.settings;
        setting_row(ui, name_width, tr(lang, Text::Language), |ui| {
            // Every language is listed under its own name, so the list can be read
            // whatever the current language is.
            let mut chosen = lang;
            egui::ComboBox::from_id_salt("language_combo")
                .selected_text(lang.native_name())
                .width(ui.available_width())
                .height(LIST_HEIGHT)
                .truncate()
                .show_ui(ui, |ui| {
                    for value in Lang::ALL {
                        ui.selectable_value(&mut chosen, value, value.native_name());
                    }
                });
            if chosen != lang {
                actions.push(Action::SetLang(chosen));
            }
        });
        setting_row(ui, name_width, tr(lang, Text::Board), |ui| {
            let mut chosen = settings.board;
            egui::ComboBox::from_id_salt("board_style_combo")
                .selected_text(tr(lang, chosen.text_key()))
                .width(ui.available_width())
                .truncate()
                .show_ui(ui, |ui| {
                    for value in BoardStyle::ALL {
                        ui.selectable_value(&mut chosen, value, tr(lang, value.text_key()));
                    }
                });
            if chosen != settings.board {
                actions.push(Action::SetBoard(chosen));
            }
        });
        let mut sound = settings.sound;
        if ui.checkbox(&mut sound, tr(lang, Text::Sound)).changed() {
            actions.push(Action::ToggleSound);
        }
    }

    /// One switch for all special effects and one for each.
    fn effect_settings(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let lang = self.lang();
        let settings = &self.settings;
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
    }
}

/// The name of a key that is held for a shortcut: Command on macOS, Control elsewhere.
fn command_key(lang: Lang) -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        tr(lang, Text::KeyCtrl)
    }
}

/// A shortcut of the command key and a letter, such as `Ctrl+N`.
fn shortcut(lang: Lang, letter: char) -> String {
    format!("{}+{letter}", command_key(lang))
}

/// The heading of a group of settings, with a line above it that parts it from the
/// group before.
fn section(ui: &mut Ui, title: &str, first: bool) {
    if !first {
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(2.0);
    }
    ui.label(RichText::new(title).size(16.5));
    ui.add_space(2.0);
}

/// A setting with its name: side by side in a column of names when there is room, the
/// name above the control in a narrow panel.
fn setting_row(ui: &mut Ui, name_width: f32, name: &str, add_control: impl FnOnce(&mut Ui)) {
    if ui.available_width() - name_width >= CONTROL_MIN_WIDTH {
        ui.horizontal(|ui| {
            fixed_width(ui, name_width, |ui| {
                ui.label(name);
            });
            add_control(ui);
        });
    } else {
        ui.label(name);
        ui.horizontal(add_control);
    }
}

/// Lays out `add_contents` in a column of the given width at the height of a control.
fn fixed_width(ui: &mut Ui, width: f32, add_contents: impl FnOnce(&mut Ui)) {
    let size = vec2(width, ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
        ui.set_min_width(width);
        add_contents(ui);
    });
}

/// The width of a text in one line.
fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.fonts(|fonts| {
        fonts
            .layout_no_wrap(text.to_owned(), font.clone(), Color32::PLACEHOLDER)
            .size()
            .x
    })
}

/// What is printed on a key in the table of keys.
enum Cap {
    Name(String),
    /// An arrow key: the direction of its arrow.
    Arrow(Vec2),
}

/// What the keys do: the keys on the left as they are printed, the action on the right.
fn keys_table(ui: &mut Ui, lang: Lang) {
    let name = |key| Cap::Name(tr(lang, key).to_owned());
    let rows: [(Vec<Cap>, Text); 9] = [
        (
            vec![Cap::Arrow(Vec2::LEFT), Cap::Arrow(Vec2::RIGHT)],
            Text::KeyStepMoves,
        ),
        (
            vec![Cap::Arrow(Vec2::UP), Cap::Arrow(Vec2::DOWN)],
            Text::KeyVariations,
        ),
        (vec![name(Text::KeyHome), name(Text::KeyEnd)], Text::KeyStartEnd),
        (vec![name(Text::KeySpace)], Text::KeyContinue),
        (vec![name(Text::KeyEsc)], Text::KeyCancel),
        (vec![Cap::Name("F9".into())], Text::KeyPanel),
        (vec![Cap::Name(shortcut(lang, 'N'))], Text::NewGame),
        (vec![Cap::Name(shortcut(lang, 'O'))], Text::Open),
        (vec![Cap::Name(shortcut(lang, 'S'))], Text::Save),
    ];
    let font = FontId::proportional(KEYCAP_FONT_SIZE);
    let gap = 3.0;
    let keys_width = |keys: &[Cap]| {
        keys.iter().map(|cap| keycap_size(ui, cap, &font).x).sum::<f32>()
            + gap * (keys.len().saturating_sub(1) as f32)
    };
    let column = rows.iter().map(|(keys, _)| keys_width(keys)).fold(0.0, f32::max) + 10.0;
    for (keys, action) in rows {
        ui.horizontal(|ui| {
            fixed_width(ui, column, |ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for cap in &keys {
                    keycap(ui, cap, &font);
                }
            });
            ui.add(egui::Label::new(RichText::new(tr(lang, action)).size(14.0)).wrap());
        });
    }
}

fn keycap_size(ui: &Ui, cap: &Cap, font: &FontId) -> Vec2 {
    let height = ui.fonts(|fonts| fonts.row_height(font)) + 4.0;
    let width = match cap {
        Cap::Name(name) => text_width(ui, name, font) + 12.0,
        Cap::Arrow(_) => 0.0,
    };
    vec2(width.max(22.0), height)
}

/// A key as the keyboard shows it: its name or arrow on a small cap with a lower edge.
fn keycap(ui: &mut Ui, cap: &Cap, font: &FontId) {
    let size = keycap_size(ui, cap, font);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    let radius = CornerRadius::same(4);
    painter.rect_filled(rect.translate(vec2(0.0, 1.5)), radius, theme::BORDER);
    painter.rect(
        rect,
        radius,
        theme::PAPER,
        Stroke::new(1.0_f32, theme::BORDER),
        StrokeKind::Inside,
    );
    match cap {
        Cap::Name(name) => {
            let galley = painter.layout_no_wrap(name.clone(), font.clone(), theme::TEXT);
            painter.galley(rect.center() - galley.size() / 2.0, galley, theme::TEXT);
        }
        Cap::Arrow(direction) => {
            let stroke = Stroke::new(1.4_f32, theme::TEXT);
            let tip = rect.center() + *direction * 4.5;
            let tail = rect.center() - *direction * 4.5;
            let side = direction.rot90() * 3.5;
            let back = *direction * 3.5;
            painter.line_segment([tail, tip], stroke);
            painter.line(vec![tip - back + side, tip, tip - back - side], stroke);
        }
    }
}

/// The icon of the side panel: a window with a column on its right. The column is filled
/// while the panel is shown, so the button that hides it shows the panel in place.
fn paint_panel_icon(painter: &Painter, center: Pos2, color: Color32, shown: bool) {
    let rect = Rect::from_center_size(
        center.round_to_pixels(painter.pixels_per_point()),
        vec2(18.0, 14.0),
    );
    let stroke = Stroke::new(1.5_f32, color);
    let radius = 3;
    let column_left = rect.right() - 7.0;
    if shown {
        let column = Rect::from_min_max(pos2(column_left, rect.top()), rect.max);
        let corners = CornerRadius {
            nw: 0,
            sw: 0,
            ne: radius,
            se: radius,
        };
        painter.rect_filled(column, corners, color);
    } else {
        painter.vline(column_left + 0.75, rect.y_range(), stroke);
    }
    painter.rect_stroke(rect, CornerRadius::same(radius), stroke, StrokeKind::Inside);
}

/// A small piece of the colour of the side to move, as the mark on the board frame.
fn side_disc(ui: &mut Ui, side: Color) {
    let size = ui.spacing().interact_size.y;
    let (rect, _) = ui.allocate_exact_size(vec2(14.0, size), Sense::hover());
    let center = rect.center();
    let (face, outline) = match side {
        Color::White => (theme::WHITE_FACE, theme::WHITE_OUTLINE),
        Color::Black => (theme::BLACK_FACE, theme::BLACK_FACE),
    };
    ui.painter()
        .circle(center, 6.0, face, Stroke::new(1.0_f32, outline));
}

/// A bar that fills as `done` of `total` pieces arrive.
fn paint_progress(painter: &Painter, rect: Rect, done: u32, total: u32) {
    let radius = CornerRadius::same(4);
    painter.rect_filled(rect, radius, theme::BORDER);
    if done == 0 || total == 0 {
        return;
    }
    #[expect(clippy::cast_precision_loss, reason = "counts of pieces are small")]
    let share = (done as f32 / total as f32).min(1.0);
    let filled = Rect::from_min_size(rect.min, vec2(rect.width() * share, rect.height()));
    painter.rect_filled(filled, radius, theme::FRAME_LIGHT);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rucheckers::settings::Settings;

    /// A text drawn in a frame: what it says, where it is and the part of the window
    /// where it can be seen.
    struct Shown {
        text: String,
        rect: Rect,
        clip: Rect,
    }

    impl Shown {
        fn visible(&self) -> bool {
            self.clip.contains_rect(self.rect)
        }
    }

    fn find<'a>(shown: &'a [Shown], text: &str) -> Option<&'a Shown> {
        shown.iter().find(|item| item.text == text)
    }

    fn new_app(path: Option<std::path::PathBuf>, settings: Settings) -> (CheckersApp, Context) {
        let ctx = Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = CheckersApp::new(&cc, settings, path, None);
        app.new_game = None;
        app.started = true;
        (app, ctx)
    }

    fn quiet() -> Settings {
        Settings {
            sound: false,
            ..Settings::default()
        }
    }

    /// One frame of the window with the panel, or with the button that shows it when it is
    /// hidden: the texts drawn and the actions chosen.
    fn frame(
        app: &mut CheckersApp,
        ctx: &Context,
        size: Vec2,
        now: f64,
        events: Vec<egui::Event>,
    ) -> (Vec<Shown>, Vec<Action>) {
        let mut actions = Vec::new();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                time: Some(now),
                events,
                ..Default::default()
            },
            |ctx| {
                if app.settings.panel {
                    app.side_panel(ctx, &mut actions);
                } else {
                    app.panel_toggle(ctx, &mut actions);
                }
            },
        );
        let shown = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(Shown {
                    text: text.galley.text().to_owned(),
                    rect: clipped.shape.visual_bounding_rect(),
                    clip: clipped.clip_rect,
                }),
                _ => None,
            })
            .collect();
        (shown, actions)
    }

    fn press(at: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }

    fn click(at: Pos2) -> Vec<egui::Event> {
        vec![egui::Event::PointerMoved(at), press(at, true), press(at, false)]
    }

    fn apply(app: &mut CheckersApp, ctx: &Context, actions: Vec<Action>) {
        for action in actions {
            app.apply(action, ctx, 0.0);
        }
    }

    const WINDOW: Vec2 = vec2(1120.0, 760.0);

    #[test]
    fn the_settings_are_the_second_tab_and_the_board_is_chosen_there() {
        let path = std::env::temp_dir().join(format!("rucheckers-tabs-{}.conf", std::process::id()));
        let (mut app, ctx) = new_app(Some(path.clone()), quiet());
        let lang = app.lang();
        let name = |style: BoardStyle| tr(lang, style.text_key());

        // The panel opens on the game; its tabs are named in the bar.
        let _ = frame(&mut app, &ctx, WINDOW, 0.0, Vec::new());
        let (shown, _) = frame(&mut app, &ctx, WINDOW, 0.05, Vec::new());
        assert_eq!(app.tab, PanelTab::Game);
        assert!(find(&shown, tr(lang, Text::NewGame)).is_some());
        assert!(find(&shown, tr(lang, Text::Board)).is_none());
        let tab = find(&shown, tr(lang, Text::Settings))
            .expect("the settings tab")
            .rect;
        let (_, actions) = frame(&mut app, &ctx, WINDOW, 0.1, click(tab.center()));
        apply(&mut app, &ctx, actions);
        assert_eq!(app.tab, PanelTab::Settings);

        // The settings take the place of the game, with no dialog over the board.
        let (shown, _) = frame(&mut app, &ctx, WINDOW, 0.15, Vec::new());
        assert!(!app.modal_open());
        assert!(
            find(&shown, tr(lang, Text::NewGame)).is_some(),
            "the heading of a section"
        );
        let combo = find(&shown, name(BoardStyle::Standard))
            .expect("the chosen board is shown")
            .rect;
        // A click opens the list of all six boards, each under its name.
        let _ = frame(&mut app, &ctx, WINDOW, 0.2, click(combo.center()));
        let (shown, _) = frame(&mut app, &ctx, WINDOW, 0.3, Vec::new());
        for style in BoardStyle::ALL {
            assert!(find(&shown, name(style)).is_some(), "{style:?} is listed");
        }
        // The chosen board applies at once, is saved and comes back after a restart.
        let maple = find(&shown, name(BoardStyle::MapleWalnut)).unwrap().rect;
        let (_, actions) = frame(&mut app, &ctx, WINDOW, 0.4, click(maple.center()));
        assert!(actions
            .iter()
            .any(|action| matches!(action, Action::SetBoard(BoardStyle::MapleWalnut))));
        apply(&mut app, &ctx, actions);
        assert_eq!(app.settings.board, BoardStyle::MapleWalnut);
        let stored = Settings::load(&path, Settings::default());
        assert_eq!(stored.board, BoardStyle::MapleWalnut);
        std::fs::remove_file(path).unwrap();

        // Back in the game tab the list shows the current move again.
        let (shown, _) = frame(&mut app, &ctx, WINDOW, 0.5, Vec::new());
        let game = find(&shown, tr(lang, Text::GameTab)).unwrap().rect;
        app.scroll_to_current = false;
        let (_, actions) = frame(&mut app, &ctx, WINDOW, 0.6, click(game.center()));
        apply(&mut app, &ctx, actions);
        assert_eq!(app.tab, PanelTab::Game);
        assert!(app.scroll_to_current);
    }

    #[test]
    fn the_button_at_the_right_of_the_tabs_hides_the_panel_and_the_same_icon_shows_it() {
        let (mut app, ctx) = new_app(None, quiet());
        let _ = frame(&mut app, &ctx, WINDOW, 0.0, Vec::new());
        let (shown, _) = frame(&mut app, &ctx, WINDOW, 0.05, Vec::new());
        let settings = find(&shown, tr(app.lang(), Text::Settings)).unwrap().rect;
        // The button sits in the bar of the tabs, at its right end.
        let button = pos2(WINDOW.x - 10.0 - 16.0, settings.center().y);
        let (_, actions) = frame(&mut app, &ctx, WINDOW, 0.1, click(button));
        assert!(matches!(actions.as_slice(), [Action::TogglePanel]));
        apply(&mut app, &ctx, actions);
        assert!(!app.settings.panel);

        // The board takes the window; a button in its corner brings the panel back.
        let _ = frame(&mut app, &ctx, WINDOW, 0.2, Vec::new());
        let (shown, _) = frame(&mut app, &ctx, WINDOW, 0.25, Vec::new());
        assert!(find(&shown, tr(app.lang(), Text::Settings)).is_none());
        let corner = pos2(WINDOW.x - 8.0 - 18.0, 8.0 + 15.0);
        let (_, actions) = frame(&mut app, &ctx, WINDOW, 0.3, click(corner));
        assert!(matches!(actions.as_slice(), [Action::TogglePanel]));
        apply(&mut app, &ctx, actions);
        assert!(app.settings.panel);
    }

    /// Turns the mouse wheel over `at` until the area under it has scrolled to its end.
    /// Returns the time after the last frame.
    fn scroll_down(app: &mut CheckersApp, ctx: &Context, size: Vec2, at: Pos2, mut now: f64) -> f64 {
        for _ in 0..40 {
            let events = vec![
                egui::Event::PointerMoved(at),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0.0, -200.0),
                    modifiers: egui::Modifiers::default(),
                },
            ];
            let _ = frame(app, ctx, size, now, events);
            now += 0.05;
        }
        now + 1.0
    }

    #[test]
    fn the_settings_scroll_when_they_do_not_fit() {
        let (mut app, ctx) = new_app(None, quiet());
        app.tab = PanelTab::Settings;
        let size = vec2(640.0, 480.0);
        let save = tr(app.lang(), Text::Save);
        let _ = frame(&mut app, &ctx, size, 0.0, Vec::new());
        let (shown, _) = frame(&mut app, &ctx, size, 0.05, Vec::new());
        // Texts out of view are not drawn at all.
        assert!(
            find(&shown, save).is_none_or(|row| !row.visible()),
            "the end of the settings is below the window"
        );
        let now = scroll_down(&mut app, &ctx, size, pos2(size.x - 120.0, size.y / 2.0), 0.1);
        let (shown, _) = frame(&mut app, &ctx, size, now, Vec::new());
        let after = find(&shown, save).unwrap();
        assert!(
            after.visible(),
            "scrolled into view: {:?} in {:?}",
            after.rect,
            after.clip
        );
        // The tabs stay in place above the scrolled settings.
        let tab = find(&shown, tr(app.lang(), Text::Settings)).unwrap();
        assert!(tab.visible() && tab.rect.top() < 30.0);
    }

    #[test]
    fn the_game_tab_keeps_the_navigation_in_view_and_scrolls_in_a_small_window() {
        let settings = Settings {
            game: GameKind::Ugolki,
            ..quiet()
        };
        for size in [WINDOW, vec2(440.0, 380.0)] {
            let (mut app, ctx) = new_app(None, settings);
            app.session.start_game(
                rucheckers::games::Position::initial_with_home(GameKind::Ugolki, settings.ugolki_home_side),
                Color::White,
                settings.level,
            );
            let _ = frame(&mut app, &ctx, size, 0.0, Vec::new());
            let (shown, _) = frame(&mut app, &ctx, size, 0.05, Vec::new());
            // The navigation buttons are at the bottom of the window, whatever its size.
            for icon in ["⏮", "◀", "▶", "⏭"] {
                let button = find(&shown, icon).unwrap_or_else(|| panic!("{icon} in {size:?}"));
                assert!(button.visible(), "{icon} in {size:?}");
                assert!(button.rect.bottom() > size.y - 40.0, "{icon} in {size:?}");
            }
            let empty = tr(app.lang(), Text::NoMovesYet);
            let arrived = find(&shown, tr(app.lang(), Text::UgolkiArrived)).unwrap();
            assert!(arrived.visible(), "{size:?}");
            if size == WINDOW {
                // There is room: the list takes what the rest leaves.
                assert!(find(&shown, empty).is_some_and(Shown::visible));
                continue;
            }
            // The list keeps a useful height below the edge of the window, and the tab
            // scrolls to it while the navigation stays.
            assert!(find(&shown, empty).is_none_or(|text| !text.visible()), "{size:?}");
            let over_summary = arrived.rect.center();
            let now = scroll_down(&mut app, &ctx, size, over_summary, 0.1);
            let (shown, _) = frame(&mut app, &ctx, size, now, Vec::new());
            assert!(find(&shown, empty).is_some_and(Shown::visible));
            assert!(find(&shown, "⏮").is_some_and(Shown::visible));
        }
    }

    #[test]
    fn the_keys_table_names_the_command_key_of_the_system() {
        let expected = if cfg!(target_os = "macos") {
            "Cmd+N"
        } else {
            "Strg+N"
        };
        assert_eq!(shortcut(Lang::De, 'N'), expected);
        assert_eq!(
            command_key(Lang::En),
            if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" }
        );
    }
}
