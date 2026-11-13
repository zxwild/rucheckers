//! The move rules of Russian draughts: every rule on concrete positions.

mod common;

use common::{move_names, mv, pos, sq};
use rucheckers::board::{Color, Kind, Piece, Position};
use rucheckers::moves::{legal_moves, Move};
use rucheckers::rules::{GameState, Outcome, WinReason};

// ---------------------------------------------------------------------------
// The board and the starting position
// ---------------------------------------------------------------------------

#[test]
fn initial_setup_twelve_men_each_on_dark_squares_white_moves_first() {
    let p = Position::initial();
    assert_eq!(p.side, Color::White);
    let white = [
        "a1", "c1", "e1", "g1", "b2", "d2", "f2", "h2", "a3", "c3", "e3", "g3",
    ];
    let black = [
        "b6", "d6", "f6", "h6", "a7", "c7", "e7", "g7", "b8", "d8", "f8", "h8",
    ];
    for name in white {
        assert_eq!(
            p.piece_at(sq(name)),
            Some(Piece::new(Color::White, Kind::Man)),
            "{name}"
        );
    }
    for name in black {
        assert_eq!(
            p.piece_at(sq(name)),
            Some(Piece::new(Color::Black, Kind::Man)),
            "{name}"
        );
    }
    assert_eq!(p.total(), 24);
    for name in ["b4", "d4", "f4", "h4", "a5", "c5", "e5", "g5"] {
        assert_eq!(p.piece_at(sq(name)), None, "{name}");
    }
}

// ---------------------------------------------------------------------------
// Quiet moves
// ---------------------------------------------------------------------------

#[test]
fn man_moves_one_square_forward_only() {
    assert_eq!(move_names(&pos("W:We5:Bh8")), ["e5-d6", "e5-f6"]);
    assert_eq!(
        move_names(&pos("B:Wa1:Be5")),
        ["e5-d4", "e5-f4"],
        "Black moves down the board"
    );
    assert_eq!(
        move_names(&pos("W:Wa3:Bh8")),
        ["a3-b4"],
        "one move from the edge of the board"
    );
    // Own and enemy pieces block the way; an own piece cannot be jumped.
    assert_eq!(
        move_names(&pos("W:Wc3,b4,d4:Bh8")),
        ["b4-a5", "b4-c5", "d4-c5", "d4-e5"]
    );
}

#[test]
fn king_moves_any_distance_along_diagonals() {
    let names = move_names(&pos("W:WKd4:Bh2"));
    let expected = [
        "d4-a1", "d4-a7", "d4-b2", "d4-b6", "d4-c3", "d4-c5", "d4-e3", "d4-e5", "d4-f2", "d4-f6", "d4-g1",
        "d4-g7", "d4-h8",
    ];
    assert_eq!(names, expected);
    // An own piece stops a king; two pieces in a row cannot be jumped.
    let names = move_names(&pos("W:WKa1,d4:Bh2"));
    assert!(names.contains(&"a1-b2".to_string()) && names.contains(&"a1-c3".to_string()));
    assert!(!names
        .iter()
        .any(|n| n.starts_with("a1-") && (n.ends_with("d4") || n.ends_with("e5"))));
}

// ---------------------------------------------------------------------------
// Capturing is mandatory
// ---------------------------------------------------------------------------

#[test]
fn capture_is_mandatory() {
    // White has quiet moves with a1 and a capture with c3: only the capture is legal.
    assert_eq!(move_names(&pos("W:Wa1,c3:Bd4")), ["c3:e5"]);
    // The same for a king: there are many quiet moves, but it must capture.
    assert_eq!(
        move_names(&pos("W:WKa1,g1:Bc3,h8")),
        ["a1:d4", "a1:e5", "a1:f6", "a1:g7"]
    );
    // And for Black.
    assert_eq!(move_names(&pos("B:Wd4,a1:Be5,h8")), ["e5:c3"]);
}

#[test]
fn capture_must_be_completed() {
    // Stopping after the first piece is not allowed: the capture is carried to its end.
    assert_eq!(move_names(&pos("W:Wc3:Bd4,f6")), ["c3:e5:g7"]);
    assert_eq!(move_names(&pos("W:Wa1:Bb2,d4,f6")), ["a1:c3:e5:g7"]);
    let p = pos("W:Wc3:Bd4,f6");
    let after = p.after(&mv(&p, "c3:e5:g7"));
    assert_eq!(after.count(Color::Black), 0, "both captured pieces are removed");
    assert_eq!(
        after.piece_at(sq("g7")),
        Some(Piece::new(Color::White, Kind::Man))
    );
    assert_eq!(after.piece_at(sq("c3")), None);
}

#[test]
fn player_may_choose_any_capture_not_necessarily_the_longest() {
    // The piece on c3 can capture one piece (b4) or two (d4 and f6): both are legal.
    assert_eq!(move_names(&pos("W:Wc3:Bb4,d4,f6")), ["c3:a5", "c3:e5:g7"]);
    // A choice between pieces: a3 captures one, e3 captures two.
    assert_eq!(move_names(&pos("W:Wa3,e3:Bb4,f4,f6")), ["a3:c5", "e3:g5:e7"]);
    // A king need not capture the most: through b2 and b4 it captures two pieces,
    // through d2 one, and it is free to choose either.
    let p = pos("W:WKc1:Bd2,b2,b4");
    assert_eq!(
        move_names(&p),
        ["c1:a3:c5", "c1:a3:d6", "c1:a3:e7", "c1:a3:f8", "c1:e3", "c1:f4", "c1:g5", "c1:h6"]
    );
    let counts: Vec<u32> = legal_moves(&p).iter().map(Move::captured_count).collect();
    assert!(counts.contains(&1) && counts.contains(&2), "{counts:?}");
}

// ---------------------------------------------------------------------------
// A man captures backward
// ---------------------------------------------------------------------------

#[test]
fn man_captures_backward() {
    assert_eq!(
        move_names(&pos("W:We5:Bd4")),
        ["e5:c3"],
        "a white man captures backward"
    );
    assert_eq!(
        move_names(&pos("B:We5:Bd4")),
        ["d4:f6"],
        "a black man captures backward"
    );
    // A series that changes direction: forward, then backward.
    assert_eq!(move_names(&pos("W:Wa3:Bb4,d4")), ["a3:c5:e3"]);
    // Backward a man only captures; it does not move there.
    assert_eq!(move_names(&pos("W:We5:Bh8")), ["e5-d6", "e5-f6"]);
}

// ---------------------------------------------------------------------------
// The Turkish strike
// ---------------------------------------------------------------------------

#[test]
fn turkish_strike_captured_pieces_stay_until_the_move_ends() {
    // The king on c1 captures d2, f6 and b6 and ends up on a5. From there c3 looks
    // unprotected (the square d2 "behind it" will be vacated), but the captured d2 is
    // removed only after the move and remains an obstacle, so c3 cannot be captured.
    let p = pos("W:WKc1:Bd2,f6,b6,c3");
    assert_eq!(move_names(&p), ["c1:e3:a7", "c1:g5:d8:a5"]);
    for m in legal_moves(&p) {
        assert_eq!(
            m.captured() & sq("c3").bit(),
            0,
            "the piece on c3 cannot be captured"
        );
    }
    let after = p.after(&mv(&p, "c1:g5:d8:a5"));
    assert_eq!(after.count(Color::Black), 1);
    assert_eq!(
        after.piece_at(sq("c3")),
        Some(Piece::new(Color::Black, Kind::Man))
    );
    assert_eq!(
        after.piece_at(sq("a5")),
        Some(Piece::new(Color::White, Kind::King))
    );
}

#[test]
fn turkish_strike_a_piece_cannot_be_jumped_twice() {
    // The only black piece: the king captures it once and cannot jump back and forth.
    let p = pos("W:WKa1:Bb2");
    assert_eq!(
        move_names(&p),
        ["a1:c3", "a1:d4", "a1:e5", "a1:f6", "a1:g7", "a1:h8"]
    );
    for m in legal_moves(&p) {
        assert_eq!(m.captured_count(), 1);
        assert_eq!(m.path().len(), 1);
    }
    // No capture takes a piece twice: the number of jumps equals the number of captured pieces.
    for fen in [
        "W:WKd2:Bc3,c5,e5,e3",
        "W:WKc1:Bd2,f6,b6,c3",
        "W:Wa1:Bb2,d4,f6,f4,d2",
        "B:Wb2,d2,d4,f4,f6:BKa1",
    ] {
        let p = pos(fen);
        let moves = legal_moves(&p);
        assert!(!moves.is_empty(), "{fen}");
        for m in moves {
            assert_eq!(m.captured_count() as usize, m.path().len(), "{fen}: {m:?}");
            let order = m.captured_in_order();
            let mut unique = order.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), order.len(), "{fen}: {m:?}");
        }
    }
}

#[test]
fn capturing_piece_may_pass_through_its_own_starting_square() {
    // A king goes round a rhombus of four pieces and returns to its starting square:
    // during the capture that square counts as empty.
    let p = pos("W:WKd2:Bc3,c5,e5,e3");
    let names = move_names(&p);
    assert!(names.contains(&"d2:f4:d6:b4:d2".to_string()), "{names:?}");
    assert!(names.contains(&"d2:b4:d6:f4:d2".to_string()), "{names:?}");
    let after = p.after(&mv(&p, "d2:f4:d6:b4:d2"));
    assert_eq!(after.count(Color::Black), 0);
    assert_eq!(
        after.piece_at(sq("d2")),
        Some(Piece::new(Color::White, Kind::King))
    );
    // A man can return to its starting square too.
    let p = pos("W:Wc3:Bd4,f4,f2,d2");
    let names = move_names(&p);
    assert!(names.contains(&"c3:e5:g3:e1:c3".to_string()), "{names:?}");
}

// ---------------------------------------------------------------------------
// The king: capturing and choosing a landing square
// ---------------------------------------------------------------------------

#[test]
fn king_captures_from_a_distance_and_lands_on_any_free_square_behind() {
    assert_eq!(
        move_names(&pos("W:WKa1:Bc3")),
        ["a1:d4", "a1:e5", "a1:f6", "a1:g7", "a1:h8"]
    );
    assert_eq!(move_names(&pos("W:WKa1:Bf6")), ["a1:g7", "a1:h8"]);
    // Another piece stands right behind the attacked one: no capture.
    assert_eq!(move_names(&pos("W:WKa1,h2:Bc3,d4")), ["a1-b2", "h2-g3"]);
    // A piece on the edge of the board is out of reach.
    let names = move_names(&pos("W:WKa1:Bh8"));
    assert!(names.iter().all(|n| n.contains('-')), "{names:?}");
    // An own piece on the way prevents the capture.
    let names = move_names(&pos("W:WKa1,b2:Bd4"));
    assert!(names.iter().all(|n| n.contains('-')), "{names:?}");
}

#[test]
fn king_must_land_where_the_capture_can_continue() {
    // Having captured c3, the king could land on d4, e5, f6, g7 or h8, but only from f6
    // is a further capture possible (of e7), so it must land on f6.
    let p = pos("W:WKa1:Bc3,e7");
    assert_eq!(move_names(&p), ["a1:f6:d8"]);
    // If the capture continues from several squares, the player chooses: from d4, b6 is
    // captured (landing on a7); from f6, e7 (landing on d8), and from there b6 as well.
    let p = pos("W:WKa1:Bc3,e7,b6");
    assert_eq!(move_names(&p), ["a1:d4:a7", "a1:f6:d8:a5"]);
    // A continuation along the same diagonal: the second piece stands further on.
    let p = pos("W:WKa1:Bc3,f6");
    assert_eq!(move_names(&p), ["a1:d4:g7", "a1:d4:h8", "a1:e5:g7", "a1:e5:h8"]);
}

#[test]
fn king_obligation_applies_to_every_step_of_the_capture() {
    // After the second capture the king must again choose a square with a continuation.
    // a1 captures c3 and must land on f6 (from where e7 is captured); after e7 it lands
    // on d8, from where b6 is captured (through c7), landing on a5.
    let p = pos("W:WKa1:Bc3,e7,b6,h2");
    let names = move_names(&p);
    assert!(names.contains(&"a1:f6:d8:a5".to_string()), "{names:?}");
    assert!(
        !names.contains(&"a1:f6:d8".to_string()),
        "the capture cannot be broken off: {names:?}"
    );
}

// ---------------------------------------------------------------------------
// Promotion to a king
// ---------------------------------------------------------------------------

#[test]
fn man_promotes_by_a_quiet_move() {
    let p = pos("W:Wa7:Bh2");
    let m = mv(&p, "a7-b8");
    assert!(m.promotes());
    assert_eq!(m.promotion_step(), Some(0));
    let after = p.after(&m);
    assert_eq!(
        after.piece_at(sq("b8")),
        Some(Piece::new(Color::White, Kind::King))
    );

    let p = pos("B:Wa7:Bh2");
    let m = mv(&p, "h2-g1");
    assert!(m.promotes());
    let after = p.after(&m);
    assert_eq!(
        after.piece_at(sq("g1")),
        Some(Piece::new(Color::Black, Kind::King))
    );

    // A move that does not reach the last rank is not a promotion.
    let p = pos("W:Wa5:Bh2");
    assert!(!mv(&p, "a5-b6").promotes());
    // A king arriving on the last rank does not count as a "promotion".
    let p = pos("W:WKa7:Bh2");
    assert!(!mv(&p, "a7-b8").promotes());
}

#[test]
fn man_promotes_during_capture_and_continues_as_a_king() {
    // The man on b6 captures c7 and lands on d8, becomes a king, and at once, already as
    // a king, captures g5 from afar. A man cannot capture like that.
    let p = pos("W:Wb6:Bc7,g5");
    assert_eq!(move_names(&p), ["b6:d8:h4"]);
    let m = mv(&p, "b6:d8:h4");
    assert!(m.promotes());
    assert_eq!(m.promotion_step(), Some(0), "promotion at the first landing");
    let after = p.after(&m);
    assert_eq!(
        after.piece_at(sq("h4")),
        Some(Piece::new(Color::White, Kind::King))
    );
    assert_eq!(after.count(Color::Black), 0);

    // The same for Black.
    let p = pos("B:Wf2,b4:Bg3");
    assert_eq!(move_names(&p), ["g3:e1:a5"]);
    let after = p.after(&mv(&p, "g3:e1:a5"));
    assert_eq!(
        after.piece_at(sq("a5")),
        Some(Piece::new(Color::Black, Kind::King))
    );
}

#[test]
fn promoted_piece_obeys_king_rules_for_the_rest_of_the_capture() {
    // Having become a king on d8, the piece must choose a square behind the captured g5
    // from which the capture continues: from h4, g3 is captured (landing on f2 or e1).
    let p = pos("W:Wb6:Bc7,g5,g3");
    assert_eq!(move_names(&p), ["b6:d8:h4:e1", "b6:d8:h4:f2"]);
    // The Turkish strike applies after a promotion too: the captured c7 remains an obstacle.
    let p = pos("W:Wb6:Bc7,b8");
    assert_eq!(move_names(&p), ["b6:d8"]);
}

#[test]
fn promotion_during_capture_without_continuation_ends_the_move() {
    let p = pos("W:Wb6:Bc7,h2");
    assert_eq!(move_names(&p), ["b6:d8"]);
    let m = mv(&p, "b6:d8");
    assert!(m.promotes());
    let after = p.after(&m);
    assert_eq!(
        after.piece_at(sq("d8")),
        Some(Piece::new(Color::White, Kind::King))
    );
    assert_eq!(after.side, Color::Black);
}

#[test]
fn piece_captured_on_the_way_to_promotion_still_blocks_the_new_king() {
    // The man on f6 captures g7 and becomes a king on h8. The piece on c3 is on the same
    // main road, but the captured g7 is not removed yet and blocks the diagonal: the
    // capture ends here.
    let p = pos("W:Wf6:Bg7,c3");
    assert_eq!(move_names(&p), ["f6:h8"]);
    // Along another diagonal the new king captures freely: f6 takes e7, lands on d8 and takes b6.
    let p = pos("W:Wf6:Be7,b6");
    assert_eq!(move_names(&p), ["f6:d8:a5"]);
    for m in legal_moves(&p) {
        assert_eq!(m.promotion_step(), Some(0));
        assert_eq!(m.captured_count(), 2);
    }
}

// ---------------------------------------------------------------------------
// The end of a game: no pieces or no moves
// ---------------------------------------------------------------------------

#[test]
fn side_without_pieces_loses() {
    let state = GameState::new(pos("W:W:Bb2"));
    assert!(legal_moves(&state.pos).is_empty());
    assert_eq!(
        state.outcome(1),
        Some(Outcome::Win {
            winner: Color::Black,
            reason: WinReason::NoPieces
        })
    );

    // The last piece is captured: the game is over right after the move.
    let start = GameState::new(pos("W:Wc3:Bd4"));
    assert_eq!(start.outcome(1), None);
    let end = start.play(&mv(&start.pos, "c3:e5"));
    assert_eq!(
        end.outcome(1),
        Some(Outcome::Win {
            winner: Color::White,
            reason: WinReason::NoPieces
        })
    );
}

#[test]
fn side_without_legal_moves_loses() {
    // The white piece on a1 is blocked: b2 is occupied, and it cannot capture because c3
    // stands behind b2.
    let state = GameState::new(pos("W:Wa1:Bb2,c3"));
    assert!(legal_moves(&state.pos).is_empty());
    assert_eq!(
        state.outcome(1),
        Some(Outcome::Win {
            winner: Color::Black,
            reason: WinReason::NoMoves
        })
    );

    // Several pieces can be blocked, a king among them.
    let state = GameState::new(pos("B:Wg7,f6,e5:BKh8"));
    assert!(legal_moves(&state.pos).is_empty());
    assert_eq!(
        state.outcome(1),
        Some(Outcome::Win {
            winner: Color::White,
            reason: WinReason::NoMoves
        })
    );

    // A blocking move wins the game: after a3-b4 the piece on a5 can neither move nor
    // capture (c3 stands behind b4).
    let start = GameState::new(pos("W:Wa3,c3:Ba5"));
    assert_eq!(start.outcome(1), None);
    let end = start.play(&mv(&start.pos, "a3-b4"));
    assert!(legal_moves(&end.pos).is_empty());
    assert_eq!(
        end.outcome(1),
        Some(Outcome::Win {
            winner: Color::White,
            reason: WinReason::NoMoves
        })
    );
}

#[test]
fn a_position_with_moves_is_not_over() {
    assert_eq!(GameState::initial().outcome(1), None);
    assert_eq!(
        GameState::new(pos("W:Wa1:Bb2")).outcome(1),
        None,
        "there is the capture a1:c3"
    );
}
