//! Automatic draws: every rule fires exactly on the right move, not earlier, and the
//! counters are reset when the balance of forces changes.
//!
//! Long sequences of quiet moves are built by a deterministic search
//! (`common::quiet_line`): they contain no captures and no repeated positions, so every
//! rule is checked separately from the others. Plies are numbered from one.

mod common;

use common::{is_king_move, mv, play, play_all, pos, quiet_line};
use rucheckers::board::{Color, Position};
use rucheckers::moves::Move;
use rucheckers::rules::{
    DrawCounters, DrawReason, GameState, Outcome, WinReason, BALANCE_MOVES_2_3, BALANCE_MOVES_4_5,
    BALANCE_MOVES_6_7, KINGS_ONLY_MOVES, KINGS_VS_KING_MOVES, MAIN_ROAD_MOVES, REPETITION_LIMIT,
};
use rucheckers::tree::GameTree;

/// A condition on a move: the ply number in the line being built (from 1), the position
/// before the move, the move.
type Allow<'a> = &'a dyn Fn(usize, &Position, &Move) -> bool;

/// Continues the game from the current node with a quiet line of up to `max_plies`
/// plies and returns the ply of the line at which the game ended, and how.
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
        tree.play(m).unwrap_or_else(|e| panic!("ply {}: {e}", i + 1));
        if let Some(result) = tree.current_node().outcome() {
            return Some((i + 1, result));
        }
    }
    None
}

/// Checks that the draw comes exactly at the ply `expected` of a quiet line.
fn assert_draw_at(tree: &mut GameTree, expected: usize, reason: DrawReason, allow: Allow) {
    let result = run(tree, expected + 6, allow);
    assert_eq!(
        result,
        Some((expected, Outcome::Draw(reason))),
        "a draw was expected at ply {expected}"
    );
}

const fn any_quiet(_: usize, _: &Position, m: &Move) -> bool {
    !m.promotes()
}

const fn kings_only(_: usize, p: &Position, m: &Move) -> bool {
    is_king_move(p, m)
}

/// Black's lone king stays off the main road (otherwise rule 5 would start to apply).
fn black_off_road(_: usize, p: &Position, m: &Move) -> bool {
    is_king_move(p, m) && (p.side == Color::White || !m.to().on_main_road())
}

/// Black's lone king stays on the main road; White's men do not reach promotion.
fn black_on_road(_: usize, p: &Position, m: &Move) -> bool {
    !m.promotes() && (p.side == Color::White || m.to().on_main_road())
}

// ---------------------------------------------------------------------------
// Rule 1: threefold repetition
// ---------------------------------------------------------------------------

const SHUFFLE_FEN: &str = "W:WKg1,a1,c1,e1:BKb8,d8,f8,h8";

#[test]
fn rule1_third_occurrence_is_a_draw_and_not_earlier() {
    assert_eq!(REPETITION_LIMIT, 3);
    let mut tree = GameTree::from_position(pos(SHUFFLE_FEN));
    let cycle = ["g1-h2", "b8-a7", "h2-g1", "a7-b8"];
    // The first round: the starting position has occurred for the second time.
    play_all(&mut tree, &cycle);
    assert_eq!(tree.current_node().occurrences(), 2);
    assert_eq!(
        tree.current_node().outcome(),
        None,
        "the second occurrence is not a draw yet"
    );
    // The second round: the game goes on until the last move.
    for text in &cycle[..3] {
        play(&mut tree, text);
        assert_eq!(tree.current_node().outcome(), None, "after {text}");
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
    // Between the repetitions the game visits other positions.
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
    // The white king returns to its place in three moves (c1-e3-d2-c1), the black one in
    // two: the placement is the same, but the side to move is different.
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
    // Branch A: the position Q (a king on h2, Black to move).
    let a = play(&mut tree, "g1-h2");
    assert_eq!(tree.current_node().occurrences(), 1);
    // Branch B from the start: one round by another route, then the same position Q.
    tree.go_to(GameTree::ROOT);
    play_all(&mut tree, &["g1-f2", "b8-a7", "f2-g1", "a7-b8"]);
    assert_eq!(
        tree.current_node().occurrences(),
        2,
        "the starting position for the second time on branch B"
    );
    let q = play(&mut tree, "g1-h2");
    assert_ne!(q, a);
    assert_eq!(tree.get(q).unwrap().position(), tree.get(a).unwrap().position());
    assert_eq!(
        tree.current_node().occurrences(),
        1,
        "the occurrence of Q on branch A does not count"
    );
    // On branch B the starting position occurs for the third time: the draw is right here.
    play_all(&mut tree, &["b8-a7", "h2-g1"]);
    assert_eq!(tree.current_node().outcome(), None);
    play(&mut tree, "a7-b8");
    assert_eq!(
        tree.current_node().outcome(),
        Some(Outcome::Draw(DrawReason::Repetition))
    );
    // Branch A is still unfinished.
    assert_eq!(tree.get(a).unwrap().outcome(), None);
}

// ---------------------------------------------------------------------------
// Rule 2: three or more kings against one king, 15 moves of the stronger side
// ---------------------------------------------------------------------------

#[test]
fn rule2_draw_on_the_fifteenth_move_of_the_stronger_side() {
    assert_eq!(KINGS_VS_KING_MOVES, 15);
    // White (the stronger side) to move: its 15th move is ply 29.
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,Kg1:BKb8"));
    assert_draw_at(&mut tree, 29, DrawReason::KingsVsKing, &black_off_road);
    assert_eq!(tree.current_node().state().counters.kings_vs_king, 15);
}

#[test]
fn rule2_counts_moves_of_the_stronger_side_when_the_weaker_moves_first() {
    // Black to move: White's 15th move is ply 30.
    let mut tree = GameTree::from_position(pos("B:WKc1,Ke1,Kg1:BKb8"));
    assert_draw_at(&mut tree, 30, DrawReason::KingsVsKing, &black_off_road);
}

#[test]
fn rule2_applies_to_more_than_three_kings_and_to_black_as_the_stronger_side() {
    let mut tree = GameTree::from_position(pos("W:WKa3,Kc1,Ke1,Kg1:BKb8"));
    assert_draw_at(&mut tree, 29, DrawReason::KingsVsKing, &black_off_road);

    // The stronger side is Black; the lone white king keeps off the main road.
    let white_off_road = |_: usize, p: &Position, m: &Move| {
        is_king_move(p, m) && (p.side == Color::Black || !m.to().on_main_road())
    };
    let mut tree = GameTree::from_position(pos("B:WKg1:BKb8,Kd8,Kf8"));
    assert_draw_at(&mut tree, 29, DrawReason::KingsVsKing, &white_off_road);
}

#[test]
fn rule2_does_not_apply_to_two_kings_or_to_a_lone_man() {
    // Two kings and a man against a king: rule 2 does not count.
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,g1:BKb8"));
    let result = run(&mut tree, 20, &black_off_road);
    assert_eq!(result, None);
    assert_eq!(tree.current_node().state().counters.kings_vs_king, 0);
}

#[test]
fn rule2_counter_restarts_when_the_balance_changes_by_capture() {
    // Four kings against one; Black captures one of them, the balance becomes "three
    // against one", and White's 15 moves are counted anew from that moment.
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
    // The black king leaves the main road at once and never returns to it. White's 15th
    // move after the capture is ply 29 of this line (the 30th from the start).
    assert_draw_at(&mut tree, 29, DrawReason::KingsVsKing, &black_off_road);
    assert_eq!(tree.current_node().ply(), 30);
}

#[test]
fn rule2_counter_restarts_when_the_balance_changes_by_promotion() {
    // Three kings and a man against a king. Five moves by White's kings, then (ply 11)
    // the man is promoted: the balance has changed, and the count starts anew.
    let schedule = |ply: usize, p: &Position, m: &Move| {
        if ply == 11 {
            return m.promotes();
        }
        is_king_move(p, m) && (p.side == Color::White || !m.to().on_main_road())
    };
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,Kg1,a7:BKh2"));
    // Without the reset the draw would come at ply 29; with it, White's 15th move after
    // the promotion is ply 41.
    assert_draw_at(&mut tree, 41, DrawReason::KingsVsKing, &schedule);
    let path = tree.path_to(tree.current());
    let counter = |ply: usize| tree.get(path[ply]).unwrap().state().counters.kings_vs_king;
    assert_eq!(counter(10), 5);
    assert_eq!(counter(11), 0, "the promotion changed the balance of forces");
    assert_eq!(counter(13), 1);
    assert_eq!(counter(29), 9, "at ply 29 the limit is still far away");
}

#[test]
fn rule2_winning_on_the_fifteenth_move_is_a_win_not_a_draw() {
    // 14 moves are made. With the fifteenth White blocks the king: that is a win.
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
    // Any other fifteenth move is a draw.
    let idle = state.play(&mv(&start, "g1-h2"));
    assert_eq!(idle.outcome(1), Some(Outcome::Draw(DrawReason::KingsVsKing)));
    // And the fourteenth move does not give a draw yet.
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
// Rule 3: 15 moves by kings only
// ---------------------------------------------------------------------------

#[test]
fn rule3_draw_after_fifteen_king_only_moves_by_each_side() {
    assert_eq!(KINGS_ONLY_MOVES, 15);
    // Eight pieces on the board: rule 4 does not apply, only rule 3 is checked.
    let mut tree = GameTree::from_position(pos(SHUFFLE_FEN));
    assert_draw_at(&mut tree, 30, DrawReason::KingsOnly, &kings_only);
    assert_eq!(tree.current_node().state().counters.kings_only, 30);
}

#[test]
fn rule3_counter_restarts_after_a_man_move() {
    // Twenty plies by kings; at the 21st a man moves and the series is broken.
    let schedule = |ply: usize, p: &Position, m: &Move| !m.promotes() && ((ply == 21) != is_king_move(p, m));
    let mut tree = GameTree::from_position(pos(SHUFFLE_FEN));
    // Without the man's move the draw would come at ply 30; now it comes at ply 51.
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
    // A capture by a king breaks the series…
    let capture = state.play(&mv(&start, "a1:d4"));
    assert_eq!(capture.counters.kings_only, 0);
    assert_eq!(capture.outcome(1), None);
    // …while a quiet king move in the same situation would be the thirtieth.
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
        "a move of a man"
    );
}

// ---------------------------------------------------------------------------
// Rule 4: the balance of forces does not change (both sides have kings)
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

    // Moves of a man do not change the balance of forces and do not interrupt the count.
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

/// Kings move, but at the given plies men do: this keeps rule 3 from firing earlier.
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
    // Black has no king: nothing is counted.
    let mut tree = GameTree::from_position(pos("W:WKc1,a1:Bh8,f8"));
    let no_promotion = |_: usize, _: &Position, m: &Move| !m.promotes();
    assert_eq!(run(&mut tree, 8, &no_promotion), None);
    assert_eq!(tree.current_node().state().counters.balance, 0);
    // Eight pieces: nothing is counted.
    let mut tree = GameTree::from_position(pos(SHUFFLE_FEN));
    assert_eq!(run(&mut tree, 12, &any_quiet), None);
    assert_eq!(tree.current_node().state().counters.balance, 0);
}

#[test]
fn rule4_counter_restarts_after_promotion() {
    // A king and a man against a king. Six plies by kings; at the seventh the man becomes
    // a king: the balance has changed, and the five moves are counted anew.
    let schedule = |ply: usize, p: &Position, m: &Move| {
        if ply == 7 {
            m.promotes()
        } else {
            is_king_move(p, m)
        }
    };
    let mut tree = GameTree::from_position(pos("W:WKc1,a7:BKf8"));
    // Without the reset the draw would come at ply 10; with it, at ply 17.
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
    // Four pieces (a limit of 30 moves). A capture leaves three pieces: the limit becomes
    // five moves and is counted from the capture.
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
// Rule 5: three pieces against a lone king on the main road, 5 moves
// ---------------------------------------------------------------------------

#[test]
fn rule5_every_combination_of_three_pieces_draws_on_the_fifth_move() {
    assert_eq!(MAIN_ROAD_MOVES, 5);
    for fen in [
        "W:WKc1,Ke1,Kg1:BKh8", // three kings
        "W:WKc1,Ke1,a3:BKh8",  // two kings and a man
        "W:WKc1,a3,h2:BKh8",   // a king and two men
        "W:Wc1,e1,g1:BKh8",    // three men
    ] {
        // White to move: its fifth move is ply 9.
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
    // Three kings against a king on the main road: rule 2 would give a draw at the 15th
    // move and rule 4 at the 30th, but rule 5 fires before either.
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,Kg1:BKh8"));
    let result = run(&mut tree, 40, &black_on_road).unwrap();
    assert_eq!(result, (9, Outcome::Draw(DrawReason::MainRoad)));
    let counters = tree.current_node().state().counters;
    assert_eq!(counters.kings_vs_king, 5, "rule 2 was counting in parallel");
    assert_eq!(counters.balance, 9, "and so was rule 4");
}

#[test]
fn rule5_does_not_apply_off_the_main_road_or_with_other_piece_counts() {
    // The king is not on the main road.
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,g1:BKb8"));
    assert_eq!(run(&mut tree, 12, &black_off_road), None);
    assert_eq!(tree.current_node().state().counters.main_road, 0);
    // Four pieces against a king on the main road.
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,g1,a3:BKh8"));
    assert_eq!(run(&mut tree, 12, &black_on_road), None);
    assert_eq!(tree.current_node().state().counters.main_road, 0);
    // A man, not a king, is on the main road.
    let mut tree = GameTree::from_position(pos("W:Wc1,e1,g1:Bh8"));
    let no_promotion = |_: usize, _: &Position, m: &Move| !m.promotes();
    assert_eq!(run(&mut tree, 4, &no_promotion), None);
    assert_eq!(tree.current_node().state().counters.main_road, 0);
}

#[test]
fn rule5_counter_restarts_when_the_king_leaves_the_main_road_and_returns() {
    // The black king leaves the main road at ply 4 and returns at ply 6.
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
    // Without the detour the draw would come at ply 9. Now White's five moves are counted
    // from the king's return to the road: White's moves 4–8, that is, ply 15.
    assert_draw_at(&mut tree, 15, DrawReason::MainRoad, &detour);
    let path = tree.path_to(tree.current());
    let counter = |ply: usize| tree.get(path[ply]).unwrap().state().counters.main_road;
    assert_eq!(counter(3), 2, "two moves by White before the king leaves");
    assert_eq!(counter(4), 0, "the king has left the road");
    assert_eq!(counter(5), 0);
    assert_eq!(counter(6), 0, "the king is back: the count starts anew");
    assert_eq!(counter(7), 1);
    assert_eq!(counter(9), 2);
}

#[test]
fn rule5_counter_restarts_when_the_balance_changes() {
    // Two kings and a man; with its third move White promotes the man.
    let promote_on_fifth_ply = |ply: usize, p: &Position, m: &Move| match p.side {
        Color::White => (ply == 5) == m.promotes() && (ply == 5 || is_king_move(p, m)),
        Color::Black => m.to().on_main_road(),
    };
    let mut tree = GameTree::from_position(pos("W:WKc1,Ke1,a7:BKh8"));
    // Five moves by White after the promotion: moves 4–8, ply 15.
    assert_draw_at(&mut tree, 15, DrawReason::MainRoad, &promote_on_fifth_ply);
    let path = tree.path_to(tree.current());
    let counter = |ply: usize| tree.get(path[ply]).unwrap().state().counters.main_road;
    assert_eq!(counter(3), 2);
    assert_eq!(counter(5), 0, "the promotion changed the balance of forces");
    assert_eq!(counter(7), 1);
}

#[test]
fn rule5_capturing_the_king_on_the_fifth_move_wins() {
    // Four moves are made; with the fifth White captures the king: a win, not a draw.
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
    // A quiet fifth move is a draw, and the fourth does not give one yet.
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
