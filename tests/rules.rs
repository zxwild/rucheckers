//! Правила ходов русских шашек: каждое правило на конкретных позициях.

mod common;

use common::{move_names, mv, pos, sq};
use rucheckers::board::{Color, Kind, Piece, Position};
use rucheckers::moves::legal_moves;
use rucheckers::rules::{GameState, Outcome, WinReason};

// ---------------------------------------------------------------------------
// Доска и начальная расстановка
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
// Тихие ходы
// ---------------------------------------------------------------------------

#[test]
fn man_moves_one_square_forward_only() {
    assert_eq!(move_names(&pos("W:We5:Bh8")), ["e5-d6", "e5-f6"]);
    assert_eq!(
        move_names(&pos("B:Wa1:Be5")),
        ["e5-d4", "e5-f4"],
        "чёрные ходят вниз"
    );
    assert_eq!(
        move_names(&pos("W:Wa3:Bh8")),
        ["a3-b4"],
        "с края доски — один ход"
    );
    // Свои и чужие шашки преграждают путь, перепрыгнуть свою нельзя.
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
    // Своя шашка останавливает дамку; две шашки подряд перепрыгнуть нельзя.
    let names = move_names(&pos("W:WKa1,d4:Bh2"));
    assert!(names.contains(&"a1-b2".to_string()) && names.contains(&"a1-c3".to_string()));
    assert!(!names
        .iter()
        .any(|n| n.starts_with("a1-") && (n.ends_with("d4") || n.ends_with("e5"))));
}

// ---------------------------------------------------------------------------
// Взятие обязательно
// ---------------------------------------------------------------------------

#[test]
fn capture_is_mandatory() {
    // У белых есть тихие ходы шашкой a1 и взятие шашкой c3: допустимо только взятие.
    assert_eq!(move_names(&pos("W:Wa1,c3:Bd4")), ["c3:e5"]);
    // То же для дамки: тихих ходов множество, но бить обязательно.
    assert_eq!(
        move_names(&pos("W:WKa1,g1:Bc3,h8")),
        ["a1:d4", "a1:e5", "a1:f6", "a1:g7"]
    );
    // И для чёрных.
    assert_eq!(move_names(&pos("B:Wd4,a1:Be5,h8")), ["e5:c3"]);
}

#[test]
fn capture_must_be_completed() {
    // Остановиться после первой шашки нельзя: бой доводится до конца.
    assert_eq!(move_names(&pos("W:Wc3:Bd4,f6")), ["c3:e5:g7"]);
    assert_eq!(move_names(&pos("W:Wa1:Bb2,d4,f6")), ["a1:c3:e5:g7"]);
    let p = pos("W:Wc3:Bd4,f6");
    let after = p.after(&mv(&p, "c3:e5:g7"));
    assert_eq!(after.count(Color::Black), 0, "обе побитые шашки сняты");
    assert_eq!(
        after.piece_at(sq("g7")),
        Some(Piece::new(Color::White, Kind::Man))
    );
    assert_eq!(after.piece_at(sq("c3")), None);
}

#[test]
fn player_may_choose_any_capture_not_necessarily_the_longest() {
    // Шашка c3 может взять одну шашку (b4) или две (d4 и f6) — допустимы оба варианта.
    assert_eq!(move_names(&pos("W:Wc3:Bb4,d4,f6")), ["c3:a5", "c3:e5:g7"]);
    // Выбор между разными шашками: a3 берёт одну, e3 — две.
    assert_eq!(move_names(&pos("W:Wa3,e3:Bb4,f4,f6")), ["a3:c5", "e3:g5:e7"]);
    // Дамка не обязана брать большинство: через b2 и b4 она берёт две шашки,
    // через d2 — одну, и вправе выбрать любой вариант.
    let p = pos("W:WKc1:Bd2,b2,b4");
    assert_eq!(
        move_names(&p),
        ["c1:a3:c5", "c1:a3:d6", "c1:a3:e7", "c1:a3:f8", "c1:e3", "c1:f4", "c1:g5", "c1:h6"]
    );
    let counts: Vec<u32> = legal_moves(&p).iter().map(|m| m.captured_count()).collect();
    assert!(counts.contains(&1) && counts.contains(&2), "{counts:?}");
}

// ---------------------------------------------------------------------------
// Простая бьёт назад
// ---------------------------------------------------------------------------

#[test]
fn man_captures_backward() {
    assert_eq!(
        move_names(&pos("W:We5:Bd4")),
        ["e5:c3"],
        "белая простая бьёт назад"
    );
    assert_eq!(
        move_names(&pos("B:We5:Bd4")),
        ["d4:f6"],
        "чёрная простая бьёт назад"
    );
    // Серия со сменой направления: вперёд, затем назад.
    assert_eq!(move_names(&pos("W:Wa3:Bb4,d4")), ["a3:c5:e3"]);
    // Назад простая только бьёт, но не ходит.
    assert_eq!(move_names(&pos("W:We5:Bh8")), ["e5-d6", "e5-f6"]);
}

// ---------------------------------------------------------------------------
// Турецкий удар
// ---------------------------------------------------------------------------

#[test]
fn turkish_strike_captured_pieces_stay_until_the_move_ends() {
    // Дамка c1 бьёт d2, f6 и b6 и оказывается на a5. Оттуда шашка c3 выглядит незащищённой
    // (поле d2 «за ней» освободится), но побитая d2 снимается только после хода и остаётся
    // препятствием, поэтому взять c3 нельзя.
    let p = pos("W:WKc1:Bd2,f6,b6,c3");
    assert_eq!(move_names(&p), ["c1:e3:a7", "c1:g5:d8:a5"]);
    for m in legal_moves(&p) {
        assert_eq!(m.captured() & sq("c3").bit(), 0, "шашка c3 не может быть взята");
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
    // Единственная чёрная шашка: дамка бьёт её один раз и не может прыгать туда-обратно.
    let p = pos("W:WKa1:Bb2");
    assert_eq!(
        move_names(&p),
        ["a1:c3", "a1:d4", "a1:e5", "a1:f6", "a1:g7", "a1:h8"]
    );
    for m in legal_moves(&p) {
        assert_eq!(m.captured_count(), 1);
        assert_eq!(m.path().len(), 1);
    }
    // Ни в одном взятии шашка не бьётся дважды: число прыжков равно числу побитых шашек.
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
    // Дамка обходит ромб из четырёх шашек и возвращается на исходное поле:
    // во время боя оно считается свободным.
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
    // Простая тоже может вернуться на исходное поле.
    let p = pos("W:Wc3:Bd4,f4,f2,d2");
    let names = move_names(&p);
    assert!(names.contains(&"c3:e5:g3:e1:c3".to_string()), "{names:?}");
}

// ---------------------------------------------------------------------------
// Дамка: взятие и выбор поля
// ---------------------------------------------------------------------------

#[test]
fn king_captures_from_a_distance_and_lands_on_any_free_square_behind() {
    assert_eq!(
        move_names(&pos("W:WKa1:Bc3")),
        ["a1:d4", "a1:e5", "a1:f6", "a1:g7", "a1:h8"]
    );
    assert_eq!(move_names(&pos("W:WKa1:Bf6")), ["a1:g7", "a1:h8"]);
    // За побитой шашкой сразу стоит другая — взятия нет.
    assert_eq!(move_names(&pos("W:WKa1,h2:Bc3,d4")), ["a1-b2", "h2-g3"]);
    // Шашка на краю доски недосягаема.
    let names = move_names(&pos("W:WKa1:Bh8"));
    assert!(names.iter().all(|n| n.contains('-')), "{names:?}");
    // Своя шашка на пути мешает взятию.
    let names = move_names(&pos("W:WKa1,b2:Bd4"));
    assert!(names.iter().all(|n| n.contains('-')), "{names:?}");
}

#[test]
fn king_must_land_where_the_capture_can_continue() {
    // Побив c3, дамка могла бы встать на d4, e5, f6, g7 или h8, но только с f6
    // возможно дальнейшее взятие (шашки e7) — значит, она обязана встать на f6.
    let p = pos("W:WKa1:Bc3,e7");
    assert_eq!(move_names(&p), ["a1:f6:d8"]);
    // Если продолжение возможно с нескольких полей, выбирает игрок: с d4 бьётся b6
    // (приземление на a7), с f6 — e7 (на d8), откуда бой продолжается взятием b6.
    let p = pos("W:WKa1:Bc3,e7,b6");
    assert_eq!(move_names(&p), ["a1:d4:a7", "a1:f6:d8:a5"]);
    // Продолжение по той же диагонали: вторая шашка стоит дальше по ходу.
    let p = pos("W:WKa1:Bc3,f6");
    assert_eq!(move_names(&p), ["a1:d4:g7", "a1:d4:h8", "a1:e5:g7", "a1:e5:h8"]);
}

#[test]
fn king_obligation_applies_to_every_step_of_the_capture() {
    // После второго взятия дамка тоже обязана выбрать поле с продолжением.
    // a1 бьёт c3 → обязана встать на f6 (оттуда бьётся e7) → после e7 встаёт на d8,
    // откуда бьётся b6 (через c7) с приземлением на a5.
    let p = pos("W:WKa1:Bc3,e7,b6,h2");
    let names = move_names(&p);
    assert!(names.contains(&"a1:f6:d8:a5".to_string()), "{names:?}");
    assert!(
        !names.contains(&"a1:f6:d8".to_string()),
        "бой нельзя прервать: {names:?}"
    );
}

// ---------------------------------------------------------------------------
// Превращение в дамку
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

    // Ход не на последнюю горизонталь превращением не является.
    let p = pos("W:Wa5:Bh2");
    assert!(!mv(&p, "a5-b6").promotes());
    // Дамка, пришедшая на последнюю горизонталь, «превращением» не считается.
    let p = pos("W:WKa7:Bh2");
    assert!(!mv(&p, "a7-b8").promotes());
}

#[test]
fn man_promotes_during_capture_and_continues_as_a_king() {
    // Простая b6 бьёт c7 и встаёт на d8 — становится дамкой — и тут же, уже как дамка,
    // бьёт издалека шашку g5. Простая так бить не может.
    let p = pos("W:Wb6:Bc7,g5");
    assert_eq!(move_names(&p), ["b6:d8:h4"]);
    let m = mv(&p, "b6:d8:h4");
    assert!(m.promotes());
    assert_eq!(m.promotion_step(), Some(0), "превращение на первом приземлении");
    let after = p.after(&m);
    assert_eq!(
        after.piece_at(sq("h4")),
        Some(Piece::new(Color::White, Kind::King))
    );
    assert_eq!(after.count(Color::Black), 0);

    // То же за чёрных.
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
    // Став дамкой на d8, шашка обязана выбрать такое поле за побитой g5, с которого
    // бой продолжается: с h4 бьётся g3 (на f2 или e1).
    let p = pos("W:Wb6:Bc7,g5,g3");
    assert_eq!(move_names(&p), ["b6:d8:h4:e1", "b6:d8:h4:f2"]);
    // Турецкий удар действует и после превращения: побитая c7 остаётся препятствием.
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
    // Простая f6 бьёт g7 и становится дамкой на h8. Шашка c3 стоит на той же большой
    // дороге, но побитая g7 ещё не снята и закрывает диагональ: бой на этом закончен.
    let p = pos("W:Wf6:Bg7,c3");
    assert_eq!(move_names(&p), ["f6:h8"]);
    // По другой диагонали новая дамка бьёт свободно: f6 берёт e7, встаёт на d8 и берёт b6.
    let p = pos("W:Wf6:Be7,b6");
    assert_eq!(move_names(&p), ["f6:d8:a5"]);
    for m in legal_moves(&p) {
        assert_eq!(m.promotion_step(), Some(0));
        assert_eq!(m.captured_count(), 2);
    }
}

// ---------------------------------------------------------------------------
// Окончание партии: нет шашек или нет ходов
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

    // Последняя шашка побита — партия закончена сразу после хода.
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
    // Белая шашка a1 заперта: поле b2 занято, а бить нельзя — за b2 стоит c3.
    let state = GameState::new(pos("W:Wa1:Bb2,c3"));
    assert!(legal_moves(&state.pos).is_empty());
    assert_eq!(
        state.outcome(1),
        Some(Outcome::Win {
            winner: Color::Black,
            reason: WinReason::NoMoves
        })
    );

    // Запертыми могут оказаться несколько шашек, в том числе дамка.
    let state = GameState::new(pos("B:Wg7,f6,e5:BKh8"));
    assert!(legal_moves(&state.pos).is_empty());
    assert_eq!(
        state.outcome(1),
        Some(Outcome::Win {
            winner: Color::White,
            reason: WinReason::NoMoves
        })
    );

    // Запирающий ход выигрывает партию: после a3-b4 шашка a5 не может ни пойти,
    // ни побить (за b4 стоит c3).
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
        "есть взятие a1:c3"
    );
}
