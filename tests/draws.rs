//! Автоматические ничьи: каждое правило срабатывает точно на нужном ходу, не раньше,
//! а счётчики сбрасываются при изменении соотношения сил.
//!
//! Длинные последовательности тихих ходов строятся детерминированным перебором
//! (`common::quiet_line`): в них нет взятий и повторений позиций, поэтому каждое
//! правило проверяется отдельно от остальных. Номер полухода считается от единицы.

mod common;

use common::{is_king_move, mv, play, play_all, pos, quiet_line};
use rucheckers::board::{Color, Position};
use rucheckers::moves::Move;
use rucheckers::rules::{
    DrawCounters, DrawReason, GameState, Outcome, WinReason, BALANCE_MOVES_2_3, BALANCE_MOVES_4_5,
    BALANCE_MOVES_6_7, KINGS_ONLY_MOVES, KINGS_VS_KING_MOVES, MAIN_ROAD_MOVES, REPETITION_LIMIT,
};
use rucheckers::tree::GameTree;

/// Условие на ход: номер полухода в строящейся линии (от 1), позиция до хода и ход.
type Allow<'a> = &'a dyn Fn(usize, &Position, &Move) -> bool;

/// Продолжает партию из текущего узла тихой линией до `max_plies` полуходов и
/// возвращает, на каком полуходе линии партия закончилась и чем.
fn run(tree: &mut GameTree, max_plies: usize, allow: Allow) -> Option<(usize, Outcome)> {
    let path = tree.path_to(tree.current());
    let avoid: Vec<Position> = path
        .iter()
        .filter_map(|&id| tree.get(id))
        .map(|n| *n.position())
        .collect();
    let start = *tree.current_node().position();
    let line = quiet_line(&start, &avoid, max_plies, allow);
    for (i, m) in line.iter().enumerate() {
        tree.play(m).unwrap_or_else(|e| panic!("полуход {}: {e}", i + 1));
        if let Some(result) = tree.current_node().outcome() {
            return Some((i + 1, result));
        }
    }
    None
}

/// Проверяет, что ничья наступает ровно на полуходе `expected` тихой линии.
fn assert_draw_at(tree: &mut GameTree, expected: usize, reason: DrawReason, allow: Allow) {
    let result = run(tree, expected + 6, allow);
    assert_eq!(
        result,
        Some((expected, Outcome::Draw(reason))),
        "ожидалась ничья на полуходе {expected}"
    );
}

const fn any_quiet(_: usize, _: &Position, m: &Move) -> bool {
    !m.promotes()
}

const fn kings_only(_: usize, p: &Position, m: &Move) -> bool {
    is_king_move(p, m)
}

/// Одинокая дамка чёрных не выходит на большую дорогу (иначе начнёт действовать правило 5).
fn black_off_road(_: usize, p: &Position, m: &Move) -> bool {
    is_king_move(p, m) && (p.side == Color::White || !m.to().on_main_road())
}

/// Одинокая дамка чёрных остаётся на большой дороге; простые белых не доходят до дамок.
fn black_on_road(_: usize, p: &Position, m: &Move) -> bool {
    !m.promotes() && (p.side == Color::White || m.to().on_main_road())
}

// ---------------------------------------------------------------------------
// Правило 1: троекратное повторение
// ---------------------------------------------------------------------------

const SHUFFLE_FEN: &str = "W:WKg1,a1,c1,e1:BKb8,d8,f8,h8";

#[test]
fn rule1_third_occurrence_is_a_draw_and_not_earlier() {
    assert_eq!(REPETITION_LIMIT, 3);
    let mut tree = GameTree::from_position(pos(SHUFFLE_FEN));
    let cycle = ["g1-h2", "b8-a7", "h2-g1", "a7-b8"];
    // Первый круг: исходная позиция возникла во второй раз.
    play_all(&mut tree, &cycle);
    assert_eq!(tree.current_node().occurrences(), 2);
    assert_eq!(
        tree.current_node().outcome(),
        None,
        "второе появление — ещё не ничья"
    );
    // Второй круг: до последнего хода партия продолжается.
    for text in &cycle[..3] {
        play(&mut tree, text);
        assert_eq!(tree.current_node().outcome(), None, "после {text}");
        assert_eq!(tree.current_node().occurrences(), 2);
    }
    play(&mut tree, cycle[3]);
    assert_eq!(tree.current_node().occurrences(), 3);
    assert_eq!(
        tree.current_node().outcome(),
        Some(Outcome::Draw(DrawReason::Repetition))
    );
    assert_eq!(tree.current_node().ply(), 8);
}

#[test]
fn rule1_third_occurrence_need_not_be_consecutive() {
    let mut tree = GameTree::from_position(pos(SHUFFLE_FEN));
    // Между повторениями партия уходит в другие позиции.
    play_all(&mut tree, &["g1-h2", "b8-a7", "h2-g1", "a7-b8"]);
    play_all(&mut tree, &["g1-f2", "b8-a7", "f2-g1"]);
    assert_eq!(tree.current_node().outcome(), None);
    play(&mut tree, "a7-b8");
    assert_eq!(
        tree.current_node().outcome(),
        Some(Outcome::Draw(DrawReason::Repetition))
    );
}

#[test]
fn rule1_same_arrangement_with_the_other_side_to_move_does_not_count() {
    // Белая дамка возвращается на место за три хода (c1-e3-d2-c1), чёрная — за два:
    // расстановка та же, но очередь хода другая.
    let start = pos("W:WKc1:BKh8");
    let mut tree = GameTree::from_position(start);
    play_all(&mut tree, &["c1-e3", "h8-g7", "e3-d2", "g7-h8", "d2-c1"]);
    let now = *tree.current_node().position();
    assert_eq!(
        (now.white, now.black, now.kings),
        (start.white, start.black, start.kings)
    );
    assert_ne!(now.side, start.side);
    assert_eq!(tree.current_node().occurrences(), 1);
    assert_eq!(tree.current_node().outcome(), None);
}

#[test]
fn rule1_repetitions_are_counted_along_the_current_branch_only() {
    let mut tree = GameTree::from_position(pos(SHUFFLE_FEN));
    // Ветка A: позиция Q (дамка на h2, ход чёрных).
    let a = play(&mut tree, "g1-h2");
    assert_eq!(tree.current_node().occurrences(), 1);
    // Ветка B из начала: один круг другим маршрутом, затем та же позиция Q.
    tree.go_to(GameTree::ROOT);
    play_all(&mut tree, &["g1-f2", "b8-a7", "f2-g1", "a7-b8"]);
    assert_eq!(
        tree.current_node().occurrences(),
        2,
        "исходная позиция — второй раз на ветке B"
    );
    let q = play(&mut tree, "g1-h2");
    assert_ne!(q, a);
    assert_eq!(tree.get(q).unwrap().position(), tree.get(a).unwrap().position());
    assert_eq!(
        tree.current_node().occurrences(),
        1,
        "появление Q на ветке A не считается"
    );
    // На ветке B исходная позиция возникает в третий раз — ничья именно здесь.
    play_all(&mut tree, &["b8-a7", "h2-g1"]);
    assert_eq!(tree.current_node().outcome(), None);
    play(&mut tree, "a7-b8");
    assert_eq!(
        tree.current_node().outcome(),
        Some(Outcome::Draw(DrawReason::Repetition))
    );
    // Ветка A при этом осталась незаконченной.
    assert_eq!(tree.get(a).unwrap().outcome(), None);
}

// ---------------------------------------------------------------------------
// Правило 2: три и более дамки против одной дамки — 15 ходов сильнейшей стороны
// ---------------------------------------------------------------------------

#[test]
fn rule2_draw_on_the_fifteenth_move_of_the_stronger_side() {
    assert_eq!(KINGS_VS_KING_MOVES, 15);
    // Ход белых (сильнейших): их 15-й ход — полуход 29.
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,Kg1:BKb8"));
    assert_draw_at(&mut tree, 29, DrawReason::KingsVsKing, &black_off_road);
    assert_eq!(tree.current_node().state().counters.kings_vs_king, 15);
}

#[test]
fn rule2_counts_moves_of_the_stronger_side_when_the_weaker_moves_first() {
    // Ход чёрных: 15-й ход белых — полуход 30.
    let mut tree = GameTree::from_position(pos("B:WKc1,Ke1,Kg1:BKb8"));
    assert_draw_at(&mut tree, 30, DrawReason::KingsVsKing, &black_off_road);
}

#[test]
fn rule2_applies_to_more_than_three_kings_and_to_black_as_the_stronger_side() {
    let mut tree = GameTree::from_position(pos("W:WKa3,Kc1,Ke1,Kg1:BKb8"));
    assert_draw_at(&mut tree, 29, DrawReason::KingsVsKing, &black_off_road);

    // Сильнейшая сторона — чёрные; одинокая белая дамка держится вне большой дороги.
    let white_off_road = |_: usize, p: &Position, m: &Move| {
        is_king_move(p, m) && (p.side == Color::Black || !m.to().on_main_road())
    };
    let mut tree = GameTree::from_position(pos("B:WKg1:BKb8,Kd8,Kf8"));
    assert_draw_at(&mut tree, 29, DrawReason::KingsVsKing, &white_off_road);
}

#[test]
fn rule2_does_not_apply_to_two_kings_or_to_a_lone_man() {
    // Две дамки и простая против дамки: правило 2 не отсчитывается.
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,g1:BKb8"));
    let result = run(&mut tree, 20, &black_off_road);
    assert_eq!(result, None);
    assert_eq!(tree.current_node().state().counters.kings_vs_king, 0);
}

#[test]
fn rule2_counter_restarts_when_the_balance_changes_by_capture() {
    // Четыре дамки против одной; чёрные бьют одну из них — соотношение стало «три против
    // одной», и 15 ходов белых отсчитываются заново от этого момента.
    let start = pos("B:WKc1,Ke1,Kg1,Kf6:BKh8");
    let seasoned = GameState {
        pos: start,
        counters: DrawCounters {
            kings_vs_king: 12,
            ..DrawCounters::default()
        },
    };
    let capture = mv(&start, "h8:e5");
    assert_eq!(seasoned.play(&capture).counters.kings_vs_king, 0);

    let mut tree = GameTree::from_position(start);
    tree.play(&capture).unwrap();
    assert_eq!(tree.current_node().outcome(), None);
    // Чёрная дамка сразу уходит с большой дороги и больше на неё не возвращается.
    // 15-й ход белых после взятия — полуход 29 этой линии (30-й от начала).
    assert_draw_at(&mut tree, 29, DrawReason::KingsVsKing, &black_off_road);
    assert_eq!(tree.current_node().ply(), 30);
}

#[test]
fn rule2_counter_restarts_when_the_balance_changes_by_promotion() {
    // Три дамки и простая против дамки. Пять ходов белых дамками, затем (11-й полуход)
    // простая проходит в дамки: соотношение изменилось, отсчёт начинается заново.
    let schedule = |ply: usize, p: &Position, m: &Move| {
        if ply == 11 {
            return m.promotes();
        }
        is_king_move(p, m) && (p.side == Color::White || !m.to().on_main_road())
    };
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,Kg1,a7:BKh2"));
    // Без сброса ничья наступила бы на 29-м полуходе; со сбросом 15-й ход белых после
    // превращения — 41-й полуход.
    assert_draw_at(&mut tree, 41, DrawReason::KingsVsKing, &schedule);
    let path = tree.path_to(tree.current());
    let counter = |ply: usize| tree.get(path[ply]).unwrap().state().counters.kings_vs_king;
    assert_eq!(counter(10), 5);
    assert_eq!(counter(11), 0, "превращение изменило соотношение сил");
    assert_eq!(counter(13), 1);
    assert_eq!(counter(29), 9, "на 29-м полуходе до лимита далеко");
}

#[test]
fn rule2_winning_on_the_fifteenth_move_is_a_win_not_a_draw() {
    // 14 ходов сделано. Пятнадцатым ходом белые запирают дамку — это выигрыш.
    let start = pos("W:WKc1,Kc3,Kg1:BKa1");
    let state = GameState {
        pos: start,
        counters: DrawCounters {
            kings_vs_king: 14,
            ..DrawCounters::default()
        },
    };
    let locked = state.play(&mv(&start, "c1-b2"));
    assert_eq!(
        locked.outcome(1),
        Some(Outcome::Win {
            winner: Color::White,
            reason: WinReason::NoMoves
        })
    );
    // Любой другой пятнадцатый ход — ничья.
    let idle = state.play(&mv(&start, "g1-h2"));
    assert_eq!(idle.outcome(1), Some(Outcome::Draw(DrawReason::KingsVsKing)));
    // А четырнадцатый ход ничьей ещё не даёт.
    let earlier = GameState {
        pos: start,
        counters: DrawCounters {
            kings_vs_king: 13,
            ..DrawCounters::default()
        },
    };
    assert_eq!(earlier.play(&mv(&start, "g1-h2")).outcome(1), None);
}

// ---------------------------------------------------------------------------
// Правило 3: 15 ходов только дамками
// ---------------------------------------------------------------------------

#[test]
fn rule3_draw_after_fifteen_king_only_moves_by_each_side() {
    assert_eq!(KINGS_ONLY_MOVES, 15);
    // На доске восемь фигур: правило 4 не действует, проверяется только правило 3.
    let mut tree = GameTree::from_position(pos(SHUFFLE_FEN));
    assert_draw_at(&mut tree, 30, DrawReason::KingsOnly, &kings_only);
    assert_eq!(tree.current_node().state().counters.kings_only, 30);
}

#[test]
fn rule3_counter_restarts_after_a_man_move() {
    // Двадцать полуходов дамками, на 21-м ходит простая — серия прервана.
    let schedule = |ply: usize, p: &Position, m: &Move| !m.promotes() && ((ply == 21) != is_king_move(p, m));
    let mut tree = GameTree::from_position(pos(SHUFFLE_FEN));
    // Без хода простой ничья была бы на 30-м полуходе; теперь — на 51-м.
    assert_draw_at(&mut tree, 51, DrawReason::KingsOnly, &schedule);
    let path = tree.path_to(tree.current());
    let counter = |ply: usize| tree.get(path[ply]).unwrap().state().counters.kings_only;
    assert_eq!(counter(20), 20);
    assert_eq!(counter(21), 0);
    assert_eq!(counter(50), 29);
}

#[test]
fn rule3_counter_restarts_after_a_capture_even_by_a_king() {
    let start = pos("W:WKa1,c1:Bc3,Kh2,h8");
    let state = GameState {
        pos: start,
        counters: DrawCounters {
            kings_only: 29,
            ..DrawCounters::default()
        },
    };
    // Взятие дамкой серию прерывает…
    let capture = state.play(&mv(&start, "a1:d4"));
    assert_eq!(capture.counters.kings_only, 0);
    assert_eq!(capture.outcome(1), None);
    // …а тихий ход дамки в такой же ситуации стал бы тридцатым.
    let quiet_start = pos("W:WKa1,c1:BKh2,h8,f8");
    let quiet = GameState {
        pos: quiet_start,
        counters: DrawCounters {
            kings_only: 29,
            ..DrawCounters::default()
        },
    };
    assert_eq!(
        quiet.play(&mv(&quiet_start, "a1-b2")).outcome(1),
        Some(Outcome::Draw(DrawReason::KingsOnly))
    );
    assert_eq!(
        quiet.play(&mv(&quiet_start, "c1-d2")).outcome(1),
        None,
        "ход простой"
    );
}

// ---------------------------------------------------------------------------
// Правило 4: соотношение сил не меняется (у обеих сторон дамки)
// ---------------------------------------------------------------------------

#[test]
fn rule4_two_and_three_pieces_five_moves() {
    assert_eq!(BALANCE_MOVES_2_3, 5);
    let mut tree = GameTree::from_position(pos("W:WKc1:BKf8"));
    assert_draw_at(
        &mut tree,
        10,
        DrawReason::Balance { pieces: 2, moves: 5 },
        &any_quiet,
    );

    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1:BKf8"));
    assert_draw_at(
        &mut tree,
        10,
        DrawReason::Balance { pieces: 3, moves: 5 },
        &any_quiet,
    );

    // Ходы простой соотношения сил не меняют и отсчёт не прерывают.
    let man_on_third_ply =
        |ply: usize, p: &Position, m: &Move| !m.promotes() && ((ply == 3) != is_king_move(p, m));
    let mut tree = GameTree::from_position(pos("W:WKc1,a1:BKf8"));
    assert_draw_at(
        &mut tree,
        10,
        DrawReason::Balance { pieces: 3, moves: 5 },
        &man_on_third_ply,
    );
}

/// Ходят дамки, но на указанных полуходах — простые: так правило 3 не срабатывает раньше.
fn men_at(plies: &'static [usize]) -> impl Fn(usize, &Position, &Move) -> bool {
    move |ply, p, m| !m.promotes() && (plies.contains(&ply) != is_king_move(p, m))
}

#[test]
fn rule4_four_and_five_pieces_thirty_moves() {
    assert_eq!(BALANCE_MOVES_4_5, 30);
    let mut tree = GameTree::from_position(pos("W:WKc1,a1:BKf8,h8"));
    assert_draw_at(
        &mut tree,
        60,
        DrawReason::Balance { pieces: 4, moves: 30 },
        &men_at(&[25, 50]),
    );

    let mut tree = GameTree::from_position(pos("W:WKc1,a1,g1:BKf8,h8"));
    assert_draw_at(
        &mut tree,
        60,
        DrawReason::Balance { pieces: 5, moves: 30 },
        &men_at(&[25, 50]),
    );
}

#[test]
fn rule4_six_and_seven_pieces_sixty_moves() {
    assert_eq!(BALANCE_MOVES_6_7, 60);
    let schedule = men_at(&[25, 50, 75, 100]);
    let mut tree = GameTree::from_position(pos("W:WKc1,a1,g1:BKf8,h8,b8"));
    assert_draw_at(
        &mut tree,
        120,
        DrawReason::Balance { pieces: 6, moves: 60 },
        &schedule,
    );

    let mut tree = GameTree::from_position(pos("W:WKc1,a1,g1,e1:BKf8,h8,b8"));
    assert_draw_at(
        &mut tree,
        120,
        DrawReason::Balance { pieces: 7, moves: 60 },
        &schedule,
    );
}

#[test]
fn rule4_needs_kings_on_both_sides_and_at_most_seven_pieces() {
    // У чёрных нет дамки — отсчёт не ведётся.
    let mut tree = GameTree::from_position(pos("W:WKc1,a1:Bh8,f8"));
    let no_promotion = |_: usize, _: &Position, m: &Move| !m.promotes();
    assert_eq!(run(&mut tree, 8, &no_promotion), None);
    assert_eq!(tree.current_node().state().counters.balance, 0);
    // Восемь фигур — отсчёт не ведётся.
    let mut tree = GameTree::from_position(pos(SHUFFLE_FEN));
    assert_eq!(run(&mut tree, 12, &any_quiet), None);
    assert_eq!(tree.current_node().state().counters.balance, 0);
}

#[test]
fn rule4_counter_restarts_after_promotion() {
    // Дамка и простая против дамки. Шесть полуходов дамками, на седьмом простая
    // становится дамкой: соотношение изменилось, пять ходов отсчитываются заново.
    let schedule = |ply: usize, p: &Position, m: &Move| {
        if ply == 7 {
            m.promotes()
        } else {
            is_king_move(p, m)
        }
    };
    let mut tree = GameTree::from_position(pos("W:WKc1,a7:BKf8"));
    // Без сброса ничья была бы на 10-м полуходе; со сбросом — на 17-м.
    assert_draw_at(
        &mut tree,
        17,
        DrawReason::Balance { pieces: 3, moves: 5 },
        &schedule,
    );
    let path = tree.path_to(tree.current());
    let counter = |ply: usize| tree.get(path[ply]).unwrap().state().counters.balance;
    assert_eq!(counter(6), 6);
    assert_eq!(counter(7), 0);
    assert_eq!(counter(10), 3);
}

#[test]
fn rule4_counter_restarts_after_capture_and_uses_the_new_piece_count() {
    // Четыре фигуры (лимит 30 ходов). Взятие оставляет три фигуры: лимит становится
    // равным пяти ходам и отсчитывается от взятия.
    let start = pos("W:WKc1,e3:Bf4,Kb8");
    let seasoned = GameState {
        pos: start,
        counters: DrawCounters {
            balance: 40,
            ..DrawCounters::default()
        },
    };
    let capture = mv(&start, "e3:g5");
    assert_eq!(seasoned.play(&capture).counters.balance, 0);
    assert_eq!(seasoned.play(&capture).outcome(1), None);

    let mut tree = GameTree::from_position(start);
    tree.play(&capture).unwrap();
    assert_draw_at(
        &mut tree,
        10,
        DrawReason::Balance { pieces: 3, moves: 5 },
        &any_quiet,
    );
    assert_eq!(tree.current_node().ply(), 11);
}

// ---------------------------------------------------------------------------
// Правило 5: три шашки против одинокой дамки на большой дороге — 5 ходов
// ---------------------------------------------------------------------------

#[test]
fn rule5_every_combination_of_three_pieces_draws_on_the_fifth_move() {
    assert_eq!(MAIN_ROAD_MOVES, 5);
    for fen in [
        "W:WKc1,Ke1,Kg1:BKh8", // три дамки
        "W:WKc1,Ke1,a3:BKh8",  // две дамки и простая
        "W:WKc1,a3,h2:BKh8",   // дамка и две простые
        "W:Wc1,e1,g1:BKh8",    // три простые
    ] {
        // Ход белых: их пятый ход — полуход 9.
        let mut tree = GameTree::from_position(pos(fen));
        let result = run(&mut tree, 14, &black_on_road);
        assert_eq!(result, Some((9, Outcome::Draw(DrawReason::MainRoad))), "{fen}");
        assert_eq!(tree.current_node().state().counters.main_road, 5);
    }
}

#[test]
fn rule5_counts_moves_of_the_stronger_side_when_the_weaker_moves_first() {
    let mut tree = GameTree::from_position(pos("B:WKc1,Ke1,Kg1:BKh8"));
    assert_draw_at(&mut tree, 10, DrawReason::MainRoad, &black_on_road);
}

#[test]
fn rule5_fires_before_the_slower_rules() {
    // Три дамки против дамки на большой дороге: правило 2 дало бы ничью на 15-м ходу,
    // правило 4 — на 30-м, но раньше всех срабатывает правило 5.
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,Kg1:BKh8"));
    let result = run(&mut tree, 40, &black_on_road).unwrap();
    assert_eq!(result, (9, Outcome::Draw(DrawReason::MainRoad)));
    let counters = tree.current_node().state().counters;
    assert_eq!(counters.kings_vs_king, 5, "правило 2 отсчитывалось параллельно");
    assert_eq!(counters.balance, 9, "правило 4 тоже");
}

#[test]
fn rule5_does_not_apply_off_the_main_road_or_with_other_piece_counts() {
    // Дамка не на большой дороге.
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,g1:BKb8"));
    assert_eq!(run(&mut tree, 12, &black_off_road), None);
    assert_eq!(tree.current_node().state().counters.main_road, 0);
    // Четыре шашки против дамки на большой дороге.
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,g1,a3:BKh8"));
    assert_eq!(run(&mut tree, 12, &black_on_road), None);
    assert_eq!(tree.current_node().state().counters.main_road, 0);
    // На большой дороге не дамка, а простая.
    let mut tree = GameTree::from_position(pos("W:Wc1,e1,g1:Bh8"));
    let no_promotion = |_: usize, _: &Position, m: &Move| !m.promotes();
    assert_eq!(run(&mut tree, 4, &no_promotion), None);
    assert_eq!(tree.current_node().state().counters.main_road, 0);
}

#[test]
fn rule5_counter_restarts_when_the_king_leaves_the_main_road_and_returns() {
    // Чёрная дамка на 4-м полуходе уходит с большой дороги, на 6-м возвращается.
    let detour = |ply: usize, p: &Position, m: &Move| {
        if m.promotes() {
            return false;
        }
        match (p.side, ply) {
            (Color::White, _) => true,
            (Color::Black, 4) => !m.to().on_main_road(),
            (Color::Black, _) => m.to().on_main_road(),
        }
    };
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,a3:BKh8"));
    // Без ухода ничья была бы на 9-м полуходе. Теперь пять ходов белых отсчитываются
    // от возвращения дамки на дорогу: ходы белых № 4–8, то есть полуход 15.
    assert_draw_at(&mut tree, 15, DrawReason::MainRoad, &detour);
    let path = tree.path_to(tree.current());
    let counter = |ply: usize| tree.get(path[ply]).unwrap().state().counters.main_road;
    assert_eq!(counter(3), 2, "два хода белых до ухода дамки");
    assert_eq!(counter(4), 0, "дамка ушла с дороги");
    assert_eq!(counter(5), 0);
    assert_eq!(counter(6), 0, "дамка вернулась: отсчёт начинается заново");
    assert_eq!(counter(7), 1);
    assert_eq!(counter(9), 2);
}

#[test]
fn rule5_counter_restarts_when_the_balance_changes() {
    // Две дамки и простая; третьим своим ходом белые проводят простую в дамки.
    let promote_on_fifth_ply = |ply: usize, p: &Position, m: &Move| match p.side {
        Color::White => (ply == 5) == m.promotes() && (ply == 5 || is_king_move(p, m)),
        Color::Black => m.to().on_main_road(),
    };
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,a7:BKh8"));
    // Пять ходов белых после превращения: ходы № 4–8, полуход 15.
    assert_draw_at(&mut tree, 15, DrawReason::MainRoad, &promote_on_fifth_ply);
    let path = tree.path_to(tree.current());
    let counter = |ply: usize| tree.get(path[ply]).unwrap().state().counters.main_road;
    assert_eq!(counter(3), 2);
    assert_eq!(counter(5), 0, "превращение изменило соотношение сил");
    assert_eq!(counter(7), 1);
}

#[test]
fn rule5_capturing_the_king_on_the_fifth_move_wins() {
    // Четыре хода сделано; пятым ходом белые бьют дамку — это выигрыш, а не ничья.
    let start = pos("W:WKa1,c1,e1:BKc3");
    let state = GameState {
        pos: start,
        counters: DrawCounters {
            main_road: 4,
            ..DrawCounters::default()
        },
    };
    let capture = mv(&start, "a1:d4");
    assert_eq!(
        state.play(&capture).outcome(1),
        Some(Outcome::Win {
            winner: Color::White,
            reason: WinReason::NoPieces
        })
    );
    // Тихий пятый ход — ничья, а четвёртый её ещё не даёт.
    let quiet_start = pos("W:WKe1,c1,g1:BKh8");
    let fifth = GameState {
        pos: quiet_start,
        counters: DrawCounters {
            main_road: 4,
            ..DrawCounters::default()
        },
    };
    assert_eq!(
        fifth.play(&mv(&quiet_start, "c1-d2")).outcome(1),
        Some(Outcome::Draw(DrawReason::MainRoad))
    );
    let fourth = GameState {
        pos: quiet_start,
        counters: DrawCounters {
            main_road: 3,
            ..DrawCounters::default()
        },
    };
    assert_eq!(fourth.play(&mv(&quiet_start, "c1-d2")).outcome(1), None);
}
