//! Логика интерфейса без окна: пересчёт координат мыши, выбор хода щелчками и
//! перетаскиванием, игровая сессия (ходы, просмотр истории, варианты).

mod common;

use common::{pos, sq};
use rucheckers::board::{Color, Position, Square};
use rucheckers::moves::legal_moves;
use rucheckers::notation::format_move;
use rucheckers::tree::GameTree;
use rucheckers::ui::geometry::{BoardLayout, FRAME_FRACTION};
use rucheckers::ui::input::{InputEvent, MoveInput};
use rucheckers::ui::session::{Nav, Session, SessionEvent};
use std::thread::sleep;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Геометрия: координаты мыши → поле
// ---------------------------------------------------------------------------

const WINDOW_SIZES: [(f32, f32); 8] = [
    (320.0, 240.0),
    (800.0, 600.0),
    (1024.0, 768.0),
    (1920.0, 1080.0),
    (500.0, 900.0),
    (2560.0, 1440.0),
    (733.0, 611.0),
    (641.5, 977.25),
];
const DENSITIES: [f32; 4] = [1.0, 1.25, 1.5, 2.0];

fn layouts() -> Vec<BoardLayout> {
    let mut all = Vec::new();
    for &(w, h) in &WINDOW_SIZES {
        for &ppp in &DENSITIES {
            for flipped in [false, true] {
                // Доска занимает область со смещением: слева панель или отступ.
                all.push(BoardLayout::fit(0.0, 0.0, w, h, flipped, ppp));
                all.push(BoardLayout::fit(13.0, 37.5, w - 13.0, h - 37.5, flipped, ppp));
            }
        }
    }
    all
}

#[test]
fn every_square_is_found_under_its_own_cell() {
    for layout in layouts() {
        for square in Square::all() {
            let (cx, cy) = layout.square_center(square);
            assert_eq!(layout.square_at(cx, cy), Some(square), "{layout:?}");
            // Точки у самых углов клетки тоже принадлежат ей.
            let d = layout.cell * 0.47;
            for (dx, dy) in [(-d, -d), (d, -d), (-d, d), (d, d)] {
                assert_eq!(layout.square_at(cx + dx, cy + dy), Some(square), "{layout:?}");
            }
            // Угол клетки согласован с центром.
            let (ox, oy) = layout.cell_origin(square.file(), square.rank());
            assert!((ox + layout.cell / 2.0 - cx).abs() < 1e-3 && (oy + layout.cell / 2.0 - cy).abs() < 1e-3);
            assert_eq!(layout.cell_at(cx, cy), Some((square.file(), square.rank())));
        }
    }
}

#[test]
fn light_cells_frame_and_outside_give_no_square() {
    for layout in layouts() {
        for rank in 0..8u8 {
            for file in 0..8u8 {
                let (ox, oy) = layout.cell_origin(file, rank);
                let (cx, cy) = (ox + layout.cell / 2.0, oy + layout.cell / 2.0);
                assert_eq!(layout.cell_at(cx, cy), Some((file, rank)));
                if (file + rank) % 2 == 1 {
                    assert_eq!(layout.square_at(cx, cy), None, "светлая клетка");
                } else {
                    assert_eq!(layout.square_at(cx, cy), Square::from_file_rank(file, rank));
                }
            }
        }
        // Рамка с координатами и всё, что за доской.
        let frame = layout.frame();
        let end = layout.field_x + layout.field_size();
        let bottom = layout.field_y + layout.field_size();
        let mid_x = layout.field_x + layout.field_size() / 2.0;
        let mid_y = layout.field_y + layout.field_size() / 2.0;
        for (x, y) in [
            (layout.field_x - frame / 2.0 - 0.01, mid_y),
            (end + frame / 2.0, mid_y),
            (mid_x, layout.field_y - frame / 2.0 - 0.01),
            (mid_x, bottom + frame / 2.0),
            (layout.field_x - 0.01, layout.field_y - 0.01),
            (end + 0.01, bottom + 0.01),
            (-1000.0, -1000.0),
            (1e9, 1e9),
            (f32::NAN, mid_y),
            (mid_x, f32::INFINITY),
        ] {
            assert_eq!(layout.cell_at(x, y), None, "({x}, {y}) в {layout:?}");
            assert_eq!(layout.square_at(x, y), None);
        }
    }
}

#[test]
fn orientation_puts_the_players_colour_at_the_bottom() {
    for &(w, h) in &WINDOW_SIZES {
        for &ppp in &DENSITIES {
            let eps = 0.3;
            // Белые внизу: a1 — левый нижний угол, h8 — правый верхний.
            let l = BoardLayout::fit(0.0, 0.0, w, h, false, ppp);
            let (left, top) = (l.field_x + eps, l.field_y + eps);
            let (right, bottom) = (l.field_x + l.field_size() - eps, l.field_y + l.field_size() - eps);
            assert_eq!(l.square_at(left, bottom), Some(sq("a1")));
            assert_eq!(l.square_at(right, top), Some(sq("h8")));
            assert_eq!(
                l.cell_at(left, top),
                Some((0, 7)),
                "a8 — светлое поле слева вверху"
            );
            assert_eq!(l.square_at(left, top), None);
            assert_eq!(l.cell_at(right, bottom), Some((7, 0)));
            assert_eq!((l.file_of_column(0), l.rank_of_row(7)), (0, 0));
            // Белые шашки ниже чёрных.
            assert!(l.square_center(sq("c3")).1 > l.square_center(sq("f6")).1);
            assert!(l.square_center(sq("a1")).0 < l.square_center(sq("g1")).0);

            // Чёрные внизу: доска повёрнута на 180°, h8 — левый нижний угол.
            let f = BoardLayout::fit(0.0, 0.0, w, h, true, ppp);
            assert_eq!(f.square_at(left, bottom), Some(sq("h8")));
            assert_eq!(f.square_at(right, top), Some(sq("a1")));
            assert_eq!(f.cell_at(left, top), Some((7, 0)));
            assert_eq!((f.file_of_column(0), f.rank_of_row(7)), (7, 7));
            assert!(f.square_center(sq("c3")).1 < f.square_center(sq("f6")).1);
            assert!(f.square_center(sq("a1")).0 > f.square_center(sq("g1")).0);

            // Поворот — центральная симметрия.
            for square in Square::all() {
                let (x1, y1) = l.square_center(square);
                let (x2, y2) = f.square_center(square);
                let (cx, cy) = (l.field_x + l.field_size() / 2.0, l.field_y + l.field_size() / 2.0);
                assert!((x1 + x2 - 2.0 * cx).abs() < 1e-2 && (y1 + y2 - 2.0 * cy).abs() < 1e-2);
            }
        }
    }
}

#[test]
fn board_stays_square_and_scales_with_the_window() {
    let mut previous = 0.0;
    for side in [200.0f32, 400.0, 800.0, 1600.0, 3200.0] {
        let l = BoardLayout::fit(0.0, 0.0, side * 1.6, side, false, 1.0);
        assert!(l.cell > previous, "клетка растёт вместе с окном");
        previous = l.cell;
        assert!(l.outer_size <= side && l.outer_size > side * 0.97);
        assert!((l.frame() / l.outer_size - FRAME_FRACTION).abs() < 0.02);
        // Дробные координаты анимации лежат между центрами клеток.
        let (ax, ay) = l.square_center(sq("c3"));
        let (bx, by) = l.square_center(sq("d4"));
        let (mx, my) = l.point_at(2.5, 2.5);
        assert!((mx - (ax + bx) / 2.0).abs() < 1e-3 && (my - (ay + by) / 2.0).abs() < 1e-3);
    }
}

// ---------------------------------------------------------------------------
// Выбор хода мышью
// ---------------------------------------------------------------------------

fn input_for(fen: &str) -> MoveInput {
    let mut input = MoveInput::new();
    input.set_moves(legal_moves(&pos(fen)));
    input
}

fn names(squares: &[Square]) -> Vec<String> {
    let mut v: Vec<String> = squares.iter().map(|s| s.name()).collect();
    v.sort();
    v
}

fn completed(event: InputEvent) -> String {
    match event {
        InputEvent::Move(m) => format_move(&m),
        other => panic!("ожидался законченный ход, получено {other:?}"),
    }
}

#[test]
fn quiet_move_by_two_clicks() {
    let mut input = MoveInput::new();
    input.set_moves(legal_moves(&Position::initial()));
    assert!(input.is_enabled() && !input.must_capture());
    assert_eq!(names(&input.movable()), ["a3", "c3", "e3", "g3"]);
    assert!(input.targets().is_empty());

    assert_eq!(input.click(sq("a1")), InputEvent::None, "шашке a1 некуда ходить");
    assert_eq!(
        input.click(sq("d4")),
        InputEvent::None,
        "пустое поле без выбранной шашки"
    );
    assert_eq!(input.click(sq("f6")), InputEvent::None, "чужая шашка");
    assert_eq!(input.click(sq("c3")), InputEvent::Selected(sq("c3")));
    assert_eq!(input.origin(), Some(sq("c3")));
    assert_eq!(input.current_square(), Some(sq("c3")));
    assert_eq!(names(&input.targets()), ["b4", "d4"]);
    assert_eq!(completed(input.click(sq("d4"))), "c3-d4");
    assert_eq!(input.origin(), None, "после хода выбор снят");
}

#[test]
fn selection_can_be_changed_and_cleared() {
    let mut input = MoveInput::new();
    input.set_moves(legal_moves(&Position::initial()));
    assert_eq!(input.click(sq("c3")), InputEvent::Selected(sq("c3")));
    // Щелчок по другой своей шашке переносит выбор.
    assert_eq!(input.click(sq("e3")), InputEvent::Selected(sq("e3")));
    assert_eq!(names(&input.targets()), ["d4", "f4"]);
    // Повторный щелчок по выбранной шашке снимает выбор.
    assert_eq!(input.click(sq("e3")), InputEvent::Cleared);
    assert_eq!(input.origin(), None);
    // Щелчок по недоступному полю тоже снимает выбор.
    assert_eq!(input.click(sq("g3")), InputEvent::Selected(sq("g3")));
    assert_eq!(input.click(sq("a5")), InputEvent::Cleared);
    assert_eq!(input.click(sq("a5")), InputEvent::None);
    assert_eq!(input.cancel(), InputEvent::None);
    assert_eq!(input.click(sq("g3")), InputEvent::Selected(sq("g3")));
    assert_eq!(input.cancel(), InputEvent::Cleared);
}

#[test]
fn only_pieces_that_must_capture_can_be_selected() {
    let mut input = input_for("W:Wa1,c3,g1:Bd4");
    assert!(input.must_capture());
    assert_eq!(
        names(&input.movable()),
        ["c3"],
        "подсвечивается только шашка, обязанная бить"
    );
    assert_eq!(input.click(sq("a1")), InputEvent::None);
    assert_eq!(input.click(sq("g1")), InputEvent::None);
    assert_eq!(input.click(sq("c3")), InputEvent::Selected(sq("c3")));
    assert_eq!(names(&input.targets()), ["e5"], "тихий ход c3-b4 недоступен");
    assert_eq!(input.click(sq("b4")), InputEvent::Cleared);
    assert_eq!(input.click(sq("c3")), InputEvent::Selected(sq("c3")));
    assert_eq!(completed(input.click(sq("e5"))), "c3:e5");
}

#[test]
fn multi_capture_is_entered_step_by_step_and_the_player_chooses_the_route() {
    // После взятия d4 шашка стоит на e5 и может бить d6 (на c7) или f6 (на g7).
    let fen = "W:Wc3:Bd4,d6,f6";
    let mut input = input_for(fen);
    assert_eq!(input.moves().len(), 2);
    assert_eq!(input.click(sq("c3")), InputEvent::Selected(sq("c3")));
    assert_eq!(names(&input.targets()), ["e5"]);
    assert_eq!(
        input.click(sq("e5")),
        InputEvent::Step {
            from: sq("c3"),
            to: sq("e5")
        }
    );
    assert!(input.in_progress());
    assert_eq!(input.origin(), Some(sq("c3")));
    assert_eq!(input.current_square(), Some(sq("e5")));
    assert_eq!(input.steps(), [sq("e5")]);
    assert_eq!(
        input.captured_so_far(),
        sq("d4").bit(),
        "побитая шашка остаётся на доске до конца хода"
    );
    assert_eq!(names(&input.targets()), ["c7", "g7"]);
    // Посреди взятия посторонние щелчки ничего не меняют.
    for stray in ["a1", "c3", "e5", "d6", "h8", "b4"] {
        assert_eq!(input.click(sq(stray)), InputEvent::None, "{stray}");
        assert_eq!(input.current_square(), Some(sq("e5")));
    }
    assert_eq!(completed(input.click(sq("g7"))), "c3:e5:g7");
    assert!(!input.in_progress());

    // Другой маршрут.
    let mut input = input_for(fen);
    input.click(sq("c3"));
    input.click(sq("e5"));
    assert_eq!(completed(input.click(sq("c7"))), "c3:e5:c7");

    // Незаконченное взятие можно отменить и начать заново.
    let mut input = input_for(fen);
    input.click(sq("c3"));
    input.click(sq("e5"));
    assert_eq!(input.cancel(), InputEvent::Cleared);
    assert!(!input.in_progress());
    assert_eq!(input.origin(), None);
    assert_eq!(input.captured_so_far(), 0);
    assert_eq!(input.click(sq("c3")), InputEvent::Selected(sq("c3")));
}

#[test]
fn routes_that_split_at_the_first_jump() {
    let mut input = input_for("W:Wc3:Bb4,d4,f6");
    input.click(sq("c3"));
    assert_eq!(names(&input.targets()), ["a5", "e5"]);
    assert_eq!(
        completed(input.click(sq("a5"))),
        "c3:a5",
        "короткое взятие заканчивается сразу"
    );
    let mut input = input_for("W:Wc3:Bb4,d4,f6");
    input.click(sq("c3"));
    assert_eq!(
        input.click(sq("e5")),
        InputEvent::Step {
            from: sq("c3"),
            to: sq("e5")
        }
    );
    assert_eq!(names(&input.targets()), ["g7"]);
    assert_eq!(completed(input.click(sq("g7"))), "c3:e5:g7");
}

#[test]
fn king_landing_squares_follow_the_rules() {
    // Свободный выбор поля за побитой шашкой.
    let mut input = input_for("W:WKa1:Bc3");
    input.click(sq("a1"));
    assert_eq!(names(&input.targets()), ["d4", "e5", "f6", "g7", "h8"]);
    assert_eq!(completed(input.click(sq("g7"))), "a1:g7");

    // Обязательное поле: только с f6 бой продолжается.
    let mut input = input_for("W:WKa1:Bc3,e7");
    input.click(sq("a1"));
    assert_eq!(names(&input.targets()), ["f6"]);
    assert_eq!(input.click(sq("d4")), InputEvent::Cleared, "поле d4 недоступно");
    input.click(sq("a1"));
    assert_eq!(
        input.click(sq("f6")),
        InputEvent::Step {
            from: sq("a1"),
            to: sq("f6")
        }
    );
    assert_eq!(names(&input.targets()), ["d8"]);
    assert_eq!(completed(input.click(sq("d8"))), "a1:f6:d8");

    // Обход ромба: два маршрута с одинаковым концом выбираются первым прыжком.
    let mut input = input_for("W:WKd2:Bc3,c5,e5,e3");
    input.click(sq("d2"));
    assert_eq!(names(&input.targets()), ["b4", "f4"]);
    input.click(sq("f4"));
    assert_eq!(names(&input.targets()), ["d6"]);
    input.click(sq("d6"));
    input.click(sq("b4"));
    assert_eq!(input.captured_so_far().count_ones(), 3);
    // За последней шашкой свободны исходное поле d2 (во время боя оно пусто) и e1.
    assert_eq!(names(&input.targets()), ["d2", "e1"]);
    assert_eq!(completed(input.click(sq("d2"))), "d2:f4:d6:b4:d2");
}

#[test]
fn promotion_in_the_middle_of_a_capture_is_visible_to_the_view() {
    let mut input = input_for("W:Wb6:Bc7,g5,g3");
    input.click(sq("b6"));
    assert!(!input.promoted_so_far());
    assert_eq!(
        input.click(sq("d8")),
        InputEvent::Step {
            from: sq("b6"),
            to: sq("d8")
        }
    );
    assert!(input.promoted_so_far(), "шашка стала дамкой и рисуется дамкой");
    assert_eq!(input.captured_so_far(), sq("c7").bit());
    assert_eq!(names(&input.targets()), ["h4"]);
    assert_eq!(
        input.click(sq("h4")),
        InputEvent::Step {
            from: sq("d8"),
            to: sq("h4")
        }
    );
    assert_eq!(input.captured_so_far(), sq("c7").bit() | sq("g5").bit());
    assert_eq!(names(&input.targets()), ["e1", "f2"]);
    assert_eq!(completed(input.click(sq("f2"))), "b6:d8:h4:f2");
}

#[test]
fn moves_can_be_made_by_dragging() {
    let mut input = MoveInput::new();
    input.set_moves(legal_moves(&Position::initial()));
    assert!(!input.grab(sq("d4")), "пустое поле");
    assert!(!input.grab(sq("a1")), "шашка без ходов");
    assert!(!input.grab(sq("f6")), "чужая шашка");
    assert_eq!(input.origin(), None);
    assert!(input.grab(sq("c3")));
    assert_eq!(input.origin(), Some(sq("c3")));
    // Бросок на недоступное поле: шашка возвращается, выбор остаётся.
    assert_eq!(input.drop_on(sq("e5")), InputEvent::None);
    assert_eq!(input.drop_on(sq("c3")), InputEvent::None);
    assert_eq!(input.origin(), Some(sq("c3")));
    // Перетаскивание другой шашки переносит выбор.
    assert!(input.grab(sq("e3")));
    assert_eq!(input.origin(), Some(sq("e3")));
    assert_eq!(completed(input.drop_on(sq("f4"))), "e3-f4");

    // Серийное взятие перетаскиванием: тащить можно только бьющую шашку с её текущего поля.
    let mut input = input_for("W:Wc3,a1:Bd4,d6,f6");
    assert!(!input.grab(sq("a1")));
    assert!(input.grab(sq("c3")));
    assert_eq!(
        input.drop_on(sq("e5")),
        InputEvent::Step {
            from: sq("c3"),
            to: sq("e5")
        }
    );
    assert!(!input.grab(sq("c3")), "шашка уже ушла с c3");
    assert!(input.grab(sq("e5")));
    assert_eq!(input.drop_on(sq("a1")), InputEvent::None);
    assert!(input.in_progress());
    assert_eq!(completed(input.drop_on(sq("c7"))), "c3:e5:c7");
    // Щелчки и перетаскивание можно сочетать.
    let mut input = input_for("W:Wc3,a1:Bd4,d6,f6");
    input.click(sq("c3"));
    assert!(input.grab(sq("c3")));
    input.drop_on(sq("e5"));
    assert_eq!(completed(input.click(sq("g7"))), "c3:e5:g7");
}

#[test]
fn disabled_input_ignores_everything() {
    let mut input = MoveInput::new();
    assert!(!input.is_enabled());
    assert!(!input.must_capture());
    assert!(input.movable().is_empty() && input.targets().is_empty());
    for square in Square::all() {
        assert_eq!(input.click(square), InputEvent::None);
        assert!(!input.grab(square));
        assert_eq!(input.drop_on(square), InputEvent::None);
    }
    // Запрет ввода сбрасывает начатый выбор.
    input.set_moves(legal_moves(&Position::initial()));
    input.click(sq("c3"));
    input.clear();
    assert_eq!(input.origin(), None);
    assert!(!input.is_enabled());
}

#[test]
fn every_legal_move_can_be_entered_by_clicks() {
    // Для каждой позиции случайных партий любой допустимый ход вводится щелчками по
    // полям его маршрута.
    let mut rng = rucheckers::rng::Rng::new(2024);
    for _ in 0..150 {
        let mut state = rucheckers::rules::GameState::initial();
        for _ in 0..200 {
            let moves = legal_moves(&state.pos);
            if moves.is_empty() {
                break;
            }
            for m in &moves {
                let mut input = MoveInput::new();
                input.set_moves(moves.clone());
                assert_eq!(input.click(m.from()), InputEvent::Selected(m.from()));
                let (last, steps) = m.path().split_last().unwrap();
                let mut at = m.from();
                for step in steps {
                    assert_eq!(
                        input.click(*step),
                        InputEvent::Step { from: at, to: *step },
                        "{m:?}"
                    );
                    at = *step;
                }
                assert_eq!(input.click(*last), InputEvent::Move(*m), "{m:?}");
            }
            state = state.play(rng.pick(&moves).unwrap());
        }
    }
}

// ---------------------------------------------------------------------------
// Игровая сессия
// ---------------------------------------------------------------------------

/// Крутит сессию, пока компьютер не сделает ход. Возвращает модельное время.
fn wait_for_computer(session: &mut Session, mut now: f64) -> f64 {
    for _ in 0..4_000 {
        session.update(now);
        let events = session.take_events();
        if events.iter().any(|e| {
            matches!(
                e,
                SessionEvent::Played {
                    by_computer: true,
                    ..
                }
            )
        }) {
            return now;
        }
        sleep(Duration::from_millis(5));
        now += 0.005;
    }
    panic!("компьютер не сделал ход");
}

/// Убеждается, что компьютер не ходит: сессию крутят, дерево не меняется.
fn assert_computer_idle(session: &mut Session, now: f64) {
    let len = session.tree().len();
    let current = session.tree().current();
    for i in 0..30 {
        session.update(now + f64::from(i));
        sleep(Duration::from_millis(3));
    }
    assert!(!session.is_thinking());
    assert_eq!(session.tree().len(), len);
    assert_eq!(session.tree().current(), current);
}

fn fast_session(human: Color, level: u8) -> Session {
    let mut session = Session::new(human, level, 12345);
    session.set_min_think(0.0);
    session
}

fn click_move(session: &mut Session, from: &str, to: &str) {
    session.click(sq(from));
    match session.click(sq(to)) {
        InputEvent::Move(_) => {}
        other => panic!("ход {from}-{to} не сделан: {other:?}"),
    }
}

#[test]
fn human_moves_and_the_computer_answers_in_the_background() {
    let mut session = fast_session(Color::White, 2);
    assert!(session.is_live() && session.human_to_move() && !session.is_thinking());
    assert_eq!(session.human(), Color::White);
    assert_eq!(session.computer(), Color::Black);
    session.update(0.0);
    assert!(!session.is_thinking(), "на ходу человека компьютер не думает");
    assert_eq!(session.input().movable().len(), 4);

    click_move(&mut session, "c3", "d4");
    assert_eq!(
        session.take_events(),
        [SessionEvent::Played {
            node: 1,
            by_computer: false
        }]
    );
    assert!(!session.human_to_move());
    assert!(!session.input().is_enabled(), "на ходу компьютера ввод закрыт");
    assert_eq!(session.click(sq("e3")), InputEvent::None);

    wait_for_computer(&mut session, 0.0);
    assert_eq!(session.tree().len(), 3);
    assert!(session.human_to_move() && session.input().is_enabled());
    assert!(!session.is_thinking());
    assert_eq!(session.side_to_move(), Color::White);
}

#[test]
fn computer_moves_first_when_the_human_plays_black() {
    let mut session = fast_session(Color::Black, 1);
    assert!(!session.human_to_move());
    assert!(!session.input().is_enabled());
    wait_for_computer(&mut session, 0.0);
    assert_eq!(session.tree().len(), 2);
    assert!(session.human_to_move());
    assert_eq!(session.side_to_move(), Color::Black);
    // Чёрные ходят «вниз»: доступны шашки шестой горизонтали.
    let mut movable: Vec<String> = session.input().movable().iter().map(|s| s.name()).collect();
    movable.sort();
    assert!(movable.iter().all(|name| name.ends_with('6')), "{movable:?}");
}

#[test]
fn weak_levels_answer_after_a_short_natural_pause() {
    let mut session = Session::new(Color::White, 1, 7);
    session.set_min_think(0.5);
    click_move(&mut session, "c3", "d4");
    session.take_events();
    session.update(10.0);
    assert!(session.is_thinking());
    // Расчёт первого уровня занимает миллисекунды, но ответ придерживается.
    sleep(Duration::from_millis(300));
    session.update(10.2);
    assert_eq!(session.tree().len(), 2, "пауза ещё не истекла");
    assert!(
        session.is_thinking(),
        "индикатор размышления виден во время паузы"
    );
    session.update(10.49);
    assert_eq!(session.tree().len(), 2);
    session.update(10.5);
    assert_eq!(session.tree().len(), 3, "по истечении паузы ход сделан");
    assert!(!session.is_thinking());
}

#[test]
fn computer_does_not_move_while_history_is_reviewed() {
    let mut session = fast_session(Color::White, 2);
    click_move(&mut session, "c3", "d4");
    let now = wait_for_computer(&mut session, 0.0);
    let live_node = session.tree().current();

    // Шаг назад: показана позиция после хода человека, очередь компьютера.
    assert!(session.navigate(Nav::Back));
    assert_eq!(session.take_events(), [SessionEvent::Jumped]);
    assert!(!session.is_live());
    assert!(!session.human_to_move() && !session.input().is_enabled());
    assert!(session.can_continue());
    assert_computer_idle(&mut session, now);

    // Ещё шаг назад: начальная позиция, ход человека; «Продолжить» не нужно.
    assert!(session.navigate(Nav::Back));
    assert!(session.human_to_move() && session.input().is_enabled());
    assert!(!session.can_continue());
    assert!(!session.continue_from_here());
    assert_computer_idle(&mut session, now);
    assert!(!session.navigate(Nav::Back), "дальше назад некуда");
    assert!(!session.navigate(Nav::Start));

    // Вперёд до конца: игра не возобновляется сама.
    assert!(session.navigate(Nav::End));
    assert_eq!(session.tree().current(), live_node);
    assert!(!session.is_live());
    assert!(session.human_to_move(), "но человек может просто сделать ход");
    assert_computer_idle(&mut session, now);
    assert!(!session.navigate(Nav::End), "позиция не изменилась");
    assert!(session.navigate(Nav::To(1)));
    assert!(!session.navigate(Nav::To(1)));
    assert!(!session.navigate(Nav::To(9999)));
}

#[test]
fn continue_from_here_lets_the_computer_move_from_the_viewed_position() {
    let mut session = fast_session(Color::White, 3);
    click_move(&mut session, "c3", "d4");
    let now = wait_for_computer(&mut session, 0.0);
    let recorded = session.tree().current();

    assert!(session.navigate(Nav::Back));
    assert!(session.can_continue());
    assert!(session.continue_from_here());
    assert!(session.is_live());
    assert!(!session.can_continue());
    let now = wait_for_computer(&mut session, now + 1.0);
    // Компьютер сделал ход из показанной позиции: либо тот же, что записан, либо новый вариант.
    let node = session.tree().current();
    assert_eq!(session.tree().get(node).unwrap().parent(), Some(1));
    assert!(session.tree().get(1).unwrap().children().contains(&recorded));
    assert!(session.human_to_move());
    assert!(now > 0.0);
}

#[test]
fn a_different_human_move_creates_a_variation_and_resumes_the_game() {
    let mut session = fast_session(Color::White, 2);
    click_move(&mut session, "c3", "d4");
    let now = wait_for_computer(&mut session, 0.0);
    let old_reply = session.tree().current();

    assert!(session.navigate(Nav::Start));
    session.take_events();
    click_move(&mut session, "e3", "f4");
    let variation = session.tree().current();
    assert_eq!(
        session.tree().root().children(),
        [1, variation],
        "новый вариант рядом с прежним ходом"
    );
    assert_eq!(
        session.tree().get(1).unwrap().children(),
        [old_reply],
        "прежнее продолжение сохранено"
    );
    assert!(session.is_live(), "после хода игра продолжается");
    wait_for_computer(&mut session, now + 1.0);
    assert_eq!(
        session.tree().get(session.tree().current()).unwrap().parent(),
        Some(variation)
    );

    // Повтор записанного хода ведёт по существующей ветке и тоже возобновляет игру.
    assert!(session.navigate(Nav::Start));
    let before = session.tree().len();
    click_move(&mut session, "c3", "d4");
    assert_eq!(session.tree().current(), 1);
    assert_eq!(session.tree().len(), before);
    assert!(session.is_live());

    // Клавиши вариантов.
    assert!(session.navigate(Nav::NextVariation));
    assert_eq!(session.tree().current(), variation);
    assert!(session.navigate(Nav::PrevVariation));
    assert_eq!(session.tree().current(), 1);
    assert!(session.navigate(Nav::Forward));
    assert_eq!(session.tree().current(), old_reply);
}

#[test]
fn navigation_and_new_game_interrupt_a_long_think() {
    // Десятый уровень думает несколько секунд — этого хватает, чтобы его прервать.
    let mut session = fast_session(Color::White, 10);
    click_move(&mut session, "c3", "d4");
    session.update(0.0);
    assert!(session.is_thinking());
    sleep(Duration::from_millis(50));
    session.update(0.05);
    assert!(
        session.is_thinking(),
        "компьютер всё ещё думает, интерфейс не ждёт"
    );

    assert!(session.navigate(Nav::Back));
    assert!(!session.is_thinking(), "переход по истории прерывает расчёт");
    assert_computer_idle(&mut session, 1.0);
    assert_eq!(session.tree().len(), 2);

    // Новая партия во время размышления.
    click_move(&mut session, "c3", "d4");
    session.update(2.0);
    assert!(session.is_thinking());
    session.new_game(Color::White, 1);
    assert!(!session.is_thinking());
    assert!(session.tree().is_empty());
    assert!(session.is_live() && session.human_to_move());
    assert_eq!(session.level(), 1);
    assert!(session.take_events().contains(&SessionEvent::Jumped));
}

#[test]
fn level_can_be_changed_during_the_game() {
    let mut session = fast_session(Color::White, 10);
    click_move(&mut session, "c3", "d4");
    session.update(0.0);
    assert!(session.is_thinking());
    // Смена уровня во время размышления: расчёт начинается заново на новом уровне.
    session.set_level(1);
    assert_eq!(session.level(), 1);
    assert!(!session.is_thinking());
    let started = std::time::Instant::now();
    wait_for_computer(&mut session, 0.1);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "первый уровень отвечает быстро"
    );
    session.set_level(200);
    assert_eq!(session.level(), 10);
    session.set_level(0);
    assert_eq!(session.level(), 1);
}

#[test]
fn finished_game_accepts_no_moves() {
    // Человек (белые) бьёт последнюю шашку.
    let mut session = fast_session(Color::White, 1);
    session.load(GameTree::from_position(pos("W:Wc3:Bd4")), Some(Color::White));
    assert!(
        !session.is_live(),
        "загруженная партия открывается в режиме просмотра"
    );
    assert!(session.human_to_move());
    assert!(session.input().must_capture());
    click_move(&mut session, "c3", "e5");
    assert!(session.outcome().is_some());
    assert!(!session.human_to_move() && !session.can_continue() && !session.input().is_enabled());
    assert!(!session.continue_from_here());
    assert_computer_idle(&mut session, 0.0);
    assert_eq!(session.click(sq("e5")), InputEvent::None);
}

#[test]
fn loaded_game_opens_at_the_end_of_the_main_line_in_review_mode() {
    let mut tree = GameTree::new();
    for text in ["c3-d4", "f6-e5", "d4:f6"] {
        common::play(&mut tree, text);
    }
    tree.go_to(GameTree::ROOT);
    let mut session = fast_session(Color::White, 1);
    session.load(tree, Some(Color::Black));
    assert_eq!(session.human(), Color::Black);
    assert_eq!(session.tree().current(), 3);
    assert!(!session.is_live());
    assert!(
        session.human_to_move(),
        "ход чёрных, а человек теперь играет чёрными"
    );
    assert!(session.input().must_capture());
    assert_computer_idle(&mut session, 0.0);
    // Цвет не указан в файле — остаётся прежним.
    let mut session = fast_session(Color::White, 1);
    session.load(GameTree::new(), None);
    assert_eq!(session.human(), Color::White);
}

#[test]
fn unfinished_capture_is_dropped_on_navigation() {
    let mut session = fast_session(Color::White, 1);
    session.load(
        GameTree::from_position(pos("W:Wc3,a1:Bd4,d6,f6,h8")),
        Some(Color::White),
    );
    session.click(sq("c3"));
    assert_eq!(
        session.click(sq("e5")),
        InputEvent::Step {
            from: sq("c3"),
            to: sq("e5")
        }
    );
    assert!(session.input().in_progress());
    assert_eq!(session.cancel_input(), InputEvent::Cleared);
    assert!(!session.input().in_progress());
    // Перетаскиванием.
    assert!(session.grab(sq("c3")));
    assert_eq!(
        session.drop_on(sq("e5")),
        InputEvent::Step {
            from: sq("c3"),
            to: sq("e5")
        }
    );
    assert!(session.grab(sq("e5")));
    match session.drop_on(sq("g7")) {
        InputEvent::Move(m) => assert_eq!(format_move(&m), "c3:e5:g7"),
        other => panic!("{other:?}"),
    }
    assert_eq!(session.tree().len(), 2);
    // Возврат к началу сбрасывает выбор и снова разрешает ввод.
    assert!(session.navigate(Nav::Back));
    assert!(session.input().is_enabled() && session.input().origin().is_none());
}
