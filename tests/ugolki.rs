//! Ugolki rules checked with an independent array-board reference, exhaustive local
//! configurations, mirrored homes, both colours, and real input/record round trips.

use rucheckers::games::input::{InputEvent, MoveInput};
use rucheckers::games::session::Session;
use rucheckers::games::tree::GameTree;
use rucheckers::games::ugolki::{self, HomeSide, State};
use rucheckers::games::{self, Color, FinishReason, GameKind, Kind, Move, Outcome, Piece, Position, Square};
use rucheckers::rng::Rng;
use rucheckers::settings::Settings;
use rucheckers::ui::geometry::BoardLayout;
use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

fn sq(name: &str) -> Square {
    Square::parse(name).unwrap()
}
fn empty(side: Color, home_side: HomeSide) -> Position {
    let mut pos = Position::empty(side, GameKind::Ugolki);
    pos.home_side = home_side;
    pos
}
fn put(pos: &mut Position, square: Square, color: Color) {
    pos.put(square, Piece::new(color, Kind::Man));
}
fn fen(text: &str) -> State {
    State::new(Position::from_fen_for(GameKind::Ugolki, text).unwrap())
}
fn find(state: State, from: Square, to: Square) -> Option<Move> {
    state
        .legal_moves()
        .into_iter()
        .find(|mv| mv.from() == from && mv.to() == to)
}
fn mask(names: &[&str]) -> u64 {
    names.iter().fold(0, |bits, name| bits | sq(name).bit())
}
fn mirror_square(square: Square) -> Square {
    Square::from_file_rank(7 - square.file(), square.rank()).unwrap()
}
fn mirror(mut state: State) -> State {
    let reflect = |bits| games::squares_of(bits).fold(0, |mask, sq| mask | mirror_square(sq).bit());
    state.pos.white = reflect(state.pos.white);
    state.pos.black = reflect(state.pos.black);
    state.pos.home_side = match state.pos.home_side {
        HomeSide::Left => HomeSide::Right,
        HomeSide::Right => HomeSide::Left,
    };
    state
}
fn rotate_square(square: Square) -> Square {
    Square::from_file_rank(7 - square.file(), 7 - square.rank()).unwrap()
}
fn rotate_and_swap(mut state: State) -> State {
    let rotate = |bits| games::squares_of(bits).fold(0, |mask, sq| mask | rotate_square(sq).bit());
    (state.pos.white, state.pos.black) = (rotate(state.pos.black), rotate(state.pos.white));
    state.pos.side = state.pos.side.opponent();
    state.ply ^= 1;
    state
}
fn for_home(state: State, side: HomeSide) -> State {
    if side == HomeSide::Left {
        state
    } else {
        mirror(state)
    }
}
fn endpoints(state: State) -> BTreeSet<(Square, Square)> {
    state
        .legal_moves()
        .iter()
        .map(|mv| (mv.from(), mv.to()))
        .collect()
}

// Reference uses an array and scans possible destinations, not the engine's masks,
// directional neighbour helper or breadth-first route reconstruction.
type Board = [Option<Color>; 64];
fn board(pos: Position) -> Board {
    std::array::from_fn(|i| {
        pos.piece_at(Square::from_index(i).unwrap())
            .map(|piece| piece.color)
    })
}
fn leg(board: &Board, from: usize, to: usize) -> u8 {
    if board[to].is_some() {
        return 0;
    }
    let (x1, y1) = (from % 8, from / 8);
    let (x2, y2) = (to % 8, to / 8);
    if x1 != x2 && y1 != y2 {
        return 0;
    }
    let distance = x1.abs_diff(x2) + y1.abs_diff(y2);
    match distance {
        1 => 1,
        2 if board[((y1 + y2) / 2) * 8 + (x1 + x2) / 2].is_some() => 2,
        _ => 0,
    }
}
fn ref_locked(board: &Board, square: usize, mover: Color) -> bool {
    // Only the moving player's pieces may form the forbidden wall. Temporarily
    // remove the victim's other pieces and test whether a step or jump is possible.
    let blockers = std::array::from_fn(|i| board[i].filter(|&color| color == mover));
    (0..64).all(|to| leg(&blockers, square, to) == 0)
}
fn ref_home(square: usize, color: Color, side: HomeSide) -> bool {
    let on_left = if color == Color::White {
        side == HomeSide::Left
    } else {
        side == HomeSide::Right
    };
    (if on_left { square % 8 < 4 } else { square % 8 >= 4 })
        && (if color == Color::White {
            square / 8 < 3
        } else {
            square / 8 >= 5
        })
}
fn ref_from(state: State, origin: usize) -> BTreeSet<(Square, Square)> {
    fn visit(board: &Board, square: usize, seen: &mut [bool; 64]) {
        for to in 0..64 {
            if !seen[to] && leg(board, square, to) == 2 {
                seen[to] = true;
                visit(board, to, seen);
            }
        }
    }
    let before = board(state.pos);
    let mut vacant = before;
    vacant[origin] = None;
    let mut seen = [false; 64];
    seen[origin] = true;
    visit(&vacant, origin, &mut seen);
    (0..64)
        .filter(|&to| to != origin && (seen[to] || leg(&vacant, origin, to) == 1))
        .filter(|&to| {
            if state.ply >= 80 && ref_home(to, state.pos.side, state.pos.home_side) {
                return false;
            }
            let mut after = vacant;
            after[to] = Some(state.pos.side);
            (0..64)
                .filter(|&s| before[s] == Some(state.pos.side.opponent()))
                .all(|s| !ref_locked(&after, s, state.pos.side) || ref_locked(&before, s, state.pos.side))
        })
        .map(|to| {
            (
                Square::from_index(origin).unwrap(),
                Square::from_index(to).unwrap(),
            )
        })
        .collect()
}
fn reference(state: State) -> BTreeSet<(Square, Square)> {
    board(state.pos)
        .iter()
        .enumerate()
        .filter(|(_, piece)| **piece == Some(state.pos.side))
        .flat_map(|(origin, _)| ref_from(state, origin))
        .collect()
}
fn assert_routes(state: State) {
    let legal = state.legal_moves();
    let mut ends = BTreeSet::new();
    for mv in legal {
        assert!(ends.insert((mv.from(), mv.to())), "duplicate endpoint");
        assert_eq!(state.pos.piece_at(mv.from()).unwrap().color, state.pos.side);
        assert!(!mv.is_capture() && !mv.promotes());
        let mut occupied = board(state.pos);
        occupied[mv.from().index()] = None;
        let mut seen = BTreeSet::from([mv.from()]);
        let mut previous = mv.from();
        for &to in mv.path() {
            assert!(seen.insert(to), "repeated cell: {mv:?}");
            let distance = leg(&occupied, previous.index(), to.index());
            assert!(
                distance == 2 || (distance == 1 && mv.path().len() == 1),
                "invalid route: {mv:?}"
            );
            previous = to;
        }
        let after = state.after(&mv);
        for color in [Color::White, Color::Black] {
            assert_eq!(after.pos.count(color), state.pos.count(color));
        }
        assert_eq!(
            after.pos.pieces(state.pos.side.opponent()),
            state.pos.pieces(state.pos.side.opponent())
        );
        assert_eq!(after.pos.side, state.pos.side.opponent());
        assert_eq!(after.pos.home_side, state.pos.home_side);
        assert_eq!(after.ply, state.ply + 1);
    }
}
fn assert_input(state: State) {
    let legal = state.legal_moves();
    for origin in games::squares_of(state.pos.pieces(state.pos.side)) {
        let expected: BTreeSet<_> = legal
            .iter()
            .filter(|mv| mv.from() == origin)
            .map(Move::to)
            .collect();
        if expected.is_empty() {
            continue;
        }
        let mut input = MoveInput::new();
        input.set_moves(legal.clone());
        input.click(origin);
        let visible: BTreeSet<_> = input.targets().into_iter().chain(input.final_targets()).collect();
        assert_eq!(
            visible, expected,
            "input hints must only offer legal endpoints: {state:?}"
        );
        for to in expected {
            for drag in [false, true] {
                let mut probe = input.clone();
                let event = if drag { probe.drop_on(to) } else { probe.click(to) };
                let InputEvent::Move(mv) = event else {
                    panic!("endpoint did not finish the turn: {event:?}");
                };
                assert!(legal.contains(&mv));
                assert!(!probe.in_progress());
            }
        }
    }
}

#[test]
fn homes_are_twelve_cells_on_the_selected_side_for_either_human_colour() {
    for side in HomeSide::ALL {
        let position = ugolki::initial_with_home(side);
        for human in [Color::White, Color::Black] {
            let layout = BoardLayout::fit(0.0, 0.0, 600.0, 600.0, human == Color::Black, 1.0);
            for square in games::squares_of(position.pieces(human)) {
                let (x, y) = layout.square_center(square);
                assert_eq!(
                    x < layout.field_x + layout.field_size() / 2.0,
                    side == HomeSide::Left
                );
                assert!(y > layout.field_y + layout.field_size() / 2.0);
            }
            assert_eq!(position.count(human), 12);
        }
        assert_eq!(State::new(position).legal_moves().len(), 14);
        assert_eq!(reference(State::new(position)), endpoints(State::new(position)));
    }
    let right = ugolki::initial_with_home(HomeSide::Right);
    assert_ne!(right.white & sq("h1").bit(), 0);
    assert_ne!(right.black & sq("a8").bit(), 0);
    assert_eq!(right.white & sq("a1").bit(), 0);
}

#[test]
fn jumping_over_either_colour_works_in_every_direction_and_at_every_edge() {
    for origin in Square::all() {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let offset = |n| {
                let x = i32::from(origin.file()) + n * dx;
                let y = i32::from(origin.rank()) + n * dy;
                if (0..8).contains(&x) && (0..8).contains(&y) {
                    Square::from_file_rank(x as u8, y as u8)
                } else {
                    None
                }
            };
            let (Some(over), Some(to)) = (offset(1), offset(2)) else {
                continue;
            };
            for home in HomeSide::ALL {
                for mover in [Color::White, Color::Black] {
                    for jumped in [Color::White, Color::Black] {
                        let mut pos = empty(mover, home);
                        put(&mut pos, origin, mover);
                        put(&mut pos, over, jumped);
                        let state = State::new(pos);
                        let mv = find(state, origin, to).expect("jump over either colour must exist");
                        assert_eq!(mv.path(), &[to]);
                        assert_eq!(endpoints(state), reference(state));
                        assert_routes(state);
                        assert_input(state);
                        for occupant in [Color::White, Color::Black] {
                            let mut blocked = pos;
                            put(&mut blocked, to, occupant);
                            assert!(find(State::new(blocked), origin, to).is_none());
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn all_6561_local_occupancies_match_the_reference_for_both_colours() {
    let neighbours = ["d5", "d6", "e4", "f4", "d3", "d2", "c4", "b4"].map(sq);
    for mover in [Color::White, Color::Black] {
        for encoded in 0..6561 {
            let mut pos = empty(mover, HomeSide::Left);
            put(&mut pos, sq("d4"), mover);
            let mut digits = encoded;
            for square in neighbours {
                match digits % 3 {
                    1 => put(&mut pos, square, Color::White),
                    2 => put(&mut pos, square, Color::Black),
                    _ => {}
                }
                digits /= 3;
            }
            let state = State::new(pos);
            let actual: BTreeSet<_> = endpoints(state)
                .into_iter()
                .filter(|&(from, _)| from == sq("d4"))
                .collect();
            assert_eq!(
                actual,
                ref_from(state, sq("d4").index()),
                "configuration {encoded}, {mover:?}"
            );
        }
    }
}

#[test]
fn mixed_jump_chains_can_turn_and_end_at_each_prefix() {
    let origin = sq("a1");
    let path = ["c1", "e1", "e3", "c3"].map(sq);
    let bridges = ["b1", "d1", "e2", "d3"].map(sq);
    for home in HomeSide::ALL {
        for mover in [Color::White, Color::Black] {
            for colors in 0..16 {
                let mut pos = empty(mover, HomeSide::Left);
                put(&mut pos, origin, mover);
                for (i, bridge) in bridges.iter().enumerate() {
                    put(
                        &mut pos,
                        *bridge,
                        if colors & (1 << i) == 0 {
                            Color::White
                        } else {
                            Color::Black
                        },
                    );
                }
                let state = for_home(State::new(pos), home);
                let map = |s| {
                    if home == HomeSide::Left {
                        s
                    } else {
                        mirror_square(s)
                    }
                };
                for length in 1..=4 {
                    let mv = find(state, map(origin), map(path[length - 1])).unwrap();
                    assert_eq!(
                        mv.path(),
                        &path[..length].iter().copied().map(map).collect::<Vec<_>>()
                    );
                }
                assert_routes(state);
                assert_input(state);
                assert_eq!(endpoints(state), reference(state));
            }
        }
    }
}

#[test]
fn loops_gaps_diagonals_and_multiple_adjacent_pieces_do_not_create_false_jumps() {
    for home in HomeSide::ALL {
        for position in [
            fen("W:Wc3,d3,e4:Bd5,c4"),
            fen("W:Wa1:Ba3"),
            fen("W:Wa1:Ba2,a3"),
            fen("W:Wa1:Bb2"),
        ] {
            let state = for_home(position, home);
            assert_eq!(endpoints(state), reference(state));
            assert_routes(state);
            assert_input(state);
            assert!(state.legal_moves().iter().all(|mv| mv.from() != mv.to()));
        }
    }
    let position = fen("W:Wa1:Bb1");
    assert!(
        find(position, sq("a1"), sq("a2")).is_some(),
        "a jump does not make ordinary steps compulsory"
    );
    assert!(find(position, sq("a1"), sq("c1")).is_some());
}

#[test]
fn self_blocked_enemy_does_not_forbid_unrelated_jumps_over_either_colour() {
    for bridge in [Color::White, Color::Black] {
        for home in HomeSide::ALL {
            let mut before = fen("B:Wb1,a2,a3,e4:Ba1,d1,f4");
            put(&mut before.pos, sq("f4"), bridge);
            let closing = find(before, sq("d1"), sq("c1")).unwrap();
            let next = for_home(before.after(&closing), home);
            let map = |s| {
                if home == HomeSide::Left {
                    s
                } else {
                    mirror_square(s)
                }
            };
            assert!(find(next, map(sq("e4")), map(sq("g4"))).is_some());
            assert_eq!(endpoints(next), reference(next));
            assert_input(next);
        }
    }
}

#[test]
fn new_enemy_locks_are_forbidden_in_the_middle_and_at_a_corner() {
    for home in HomeSide::ALL {
        for (position, from, to) in [
            (fen("W:Wc4,b4,e4,f4,d3,d2,d5,e6:Bd4"), sq("e6"), sq("d6")),
            (fen("W:Wb1,c1,a2,b3:Ba1"), sq("b3"), sq("a3")),
        ] {
            let state = for_home(position, home);
            let map = |s| {
                if home == HomeSide::Left {
                    s
                } else {
                    mirror_square(s)
                }
            };
            assert!(find(state, map(from), map(to)).is_none());
            assert_eq!(endpoints(state), reference(state));
        }
    }
}

#[test]
fn reported_screenshot_positions_allow_f4_c6_and_e4() {
    // Photos 1–2: after 13...a5-a3. Photo 3: after 24...a3-c3.
    // Both games have the white home on the right; every piece is transcribed.
    let early = "W:Wc2,d2,e2,f2,g2,c3,d3,f3,g3,e4,e5,d6:Ba3,a4,b4,c4,d4,b5,c5,d5,e6,e7,f7,d8";
    let later = "W:Wc2,d2,e2,f2,d3,e3,c5,f5,b6,c6,e6,e7:Bf1,g2,b3,c3,g3,a4,b4,c4,d4,f4,d5,f7";
    for (text, ply, from, expected) in [
        (early, 26, "f2", "f1 f4 h2"),
        (early, 26, "d6", "c6 d7 f6 f8"),
        (later, 48, "e2", "e1 e4 g4"),
    ] {
        let mut original = fen(text);
        original.pos.home_side = HomeSide::Right;
        original.ply = ply;
        for swap in [false, true] {
            for reflect in [false, true] {
                let mut state = if swap { rotate_and_swap(original) } else { original };
                if reflect {
                    state = mirror(state);
                }
                let map = |square| {
                    let square = if swap { rotate_square(square) } else { square };
                    if reflect {
                        mirror_square(square)
                    } else {
                        square
                    }
                };
                let from = map(sq(from));
                let expected: BTreeSet<_> = expected.split_whitespace().map(|name| map(sq(name))).collect();
                let actual: BTreeSet<_> = state
                    .legal_moves()
                    .iter()
                    .filter(|mv| mv.from() == from)
                    .map(Move::to)
                    .collect();
                assert_eq!(actual, expected, "{from}, swap={swap}, reflect={reflect}");
                assert_eq!(endpoints(state), reference(state));
                assert_routes(state);
                assert_input(state);

                for to in expected.iter().copied() {
                    for drag in [false, true] {
                        let mut session = Session::from_position(state.pos, state.pos.side, 7, 907);
                        if drag {
                            assert!(session.grab(from));
                        } else {
                            assert_eq!(session.click(from), InputEvent::Selected(from));
                        }
                        let hints: BTreeSet<_> = session
                            .input()
                            .targets()
                            .into_iter()
                            .chain(session.input().final_targets())
                            .collect();
                        assert_eq!(hints, expected);
                        let event = if drag {
                            session.drop_on(to)
                        } else {
                            session.click(to)
                        };
                        let InputEvent::Move(mv) = event else {
                            panic!("reported destination was rejected: {from}-{to}: {event:?}");
                        };
                        assert_eq!(*session.tree().current_node().position(), state.after(&mv).pos);
                        let record = games::record::to_pdn(session.tree(), &[]);
                        let loaded = games::record::from_pdn(&record).unwrap();
                        assert!(loaded.tree.same_structure(session.tree()));
                    }
                }
            }
        }
    }
}

#[test]
fn every_blocking_wall_colour_and_closing_square_is_checked_across_the_board() {
    for victim in Square::all() {
        let wall: Vec<_> = Square::all()
            .filter(|sq| {
                let dx = sq.file().abs_diff(victim.file());
                let dy = sq.rank().abs_diff(victim.rank());
                (dx == 0 || dy == 0) && (1..=2).contains(&(dx + dy))
            })
            .collect();
        for &closing in &wall {
            // Approach the last wall cell by a step from outside the cross.
            let from = Square::all()
                .find(|sq| {
                    *sq != victim
                        && !wall.contains(sq)
                        && sq.file().abs_diff(closing.file()) + sq.rank().abs_diff(closing.rank()) == 1
                })
                .unwrap();
            let remaining: Vec<_> = wall.iter().copied().filter(|&sq| sq != closing).collect();
            for mover in [Color::White, Color::Black] {
                for home in HomeSide::ALL {
                    for colors in 0..(1 << remaining.len()) {
                        let mut pos = empty(mover, home);
                        put(&mut pos, victim, mover.opponent());
                        put(&mut pos, from, mover);
                        for (i, &square) in remaining.iter().enumerate() {
                            put(
                                &mut pos,
                                square,
                                if colors & (1 << i) == 0 {
                                    mover
                                } else {
                                    mover.opponent()
                                },
                            );
                        }
                        // Only an entirely one-colour wall is forbidden. A victim's
                        // own neighbour can be moved away on a later turn.
                        assert_eq!(find(State::new(pos), from, closing).is_some(), colors != 0,
                            "victim={victim}, closing={closing}, colours={colors}, mover={mover:?}, home={home:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn a_forbidden_lock_can_be_crossed_but_not_used_as_a_jump_endpoint() {
    let original = fen("W:Wc4,b4,e4,f4,d3,d2,d5,b6,c6,e6:Bd4");
    for swap in [false, true] {
        let state = if swap { rotate_and_swap(original) } else { original };
        for home in HomeSide::ALL {
            let state = for_home(state, home);
            let map = |square| {
                let square = if swap { rotate_square(square) } else { square };
                if home == HomeSide::Right {
                    mirror_square(square)
                } else {
                    square
                }
            };
            let from = map(sq("b6"));
            let transit = map(sq("d6"));
            let to = map(sq("f6"));
            assert!(find(state, from, transit).is_none());
            assert_eq!(find(state, from, to).unwrap().path(), &[transit, to]);
            assert_eq!(endpoints(state), reference(state));
            assert_routes(state);
            assert_input(state);
        }
    }
}

#[test]
fn a_preexisting_full_lock_does_not_forbid_other_moves_in_a_loaded_position() {
    let original = fen("W:Wb1,c1,a2,a3,e4:Ba1,f4");
    for swap in [false, true] {
        let state = if swap { rotate_and_swap(original) } else { original };
        for home in HomeSide::ALL {
            let state = for_home(state, home);
            let map = |square| {
                let square = if swap { rotate_square(square) } else { square };
                if home == HomeSide::Right {
                    mirror_square(square)
                } else {
                    square
                }
            };
            assert!(find(state, map(sq("e4")), map(sq("g4"))).is_some());
            assert_eq!(endpoints(state), reference(state));
        }
    }
}

#[test]
fn home_deadline_changes_endpoints_but_not_transit_jump_routes() {
    for home in HomeSide::ALL {
        let before = for_home(
            State {
                ply: 78,
                ..fen("W:Wa4:Ba3,b2,c3,h4")
            },
            home,
        );
        let after = State { ply: 80, ..before };
        let map = |s| {
            if home == HomeSide::Left {
                s
            } else {
                mirror_square(s)
            }
        };
        assert!(find(before, map(sq("a4")), map(sq("a2"))).is_some());
        assert!(find(after, map(sq("a4")), map(sq("a2"))).is_none());
        let chain = find(after, map(sq("a4")), map(sq("c4"))).unwrap();
        assert_eq!(chain.path(), &[map(sq("a2")), map(sq("c2")), map(sq("c4"))]);
        assert_input(after);
        assert_routes(after);
        assert_eq!(endpoints(after), reference(after));
    }
}

#[test]
fn arbitrary_positions_and_both_homes_match_reference_and_clickable_endpoints() {
    let mut rng = Rng::new(0xAA27);
    for iteration in 0..500 {
        let mut squares: Vec<_> = Square::all().collect();
        rng.shuffle(&mut squares);
        let mut pos = empty(
            if iteration % 2 == 0 {
                Color::White
            } else {
                Color::Black
            },
            HomeSide::Left,
        );
        let white = 1 + rng.below(12);
        let black = 1 + rng.below(12);
        for &s in &squares[..white] {
            put(&mut pos, s, Color::White);
        }
        for &s in &squares[white..white + black] {
            put(&mut pos, s, Color::Black);
        }
        let state = State {
            pos,
            ply: if iteration % 3 == 0 { 100 } else { 20 },
        };
        let mirrored = mirror(state);
        for state in [state, mirrored] {
            assert_eq!(endpoints(state), reference(state), "{state:?}");
            assert_routes(state);
            assert_input(state);
        }
        let reflected: BTreeSet<_> = endpoints(state)
            .iter()
            .map(|&(from, to)| (mirror_square(from), mirror_square(to)))
            .collect();
        assert_eq!(endpoints(mirrored), reflected);
        assert_eq!(state.outcome(1), mirrored.outcome(1));
        assert_eq!(
            ugolki::ai::evaluate(state, pos.side),
            ugolki::ai::evaluate(mirrored, pos.side)
        );
    }
}

#[test]
fn both_homes_use_the_correct_goals_deadlines_and_equalising_reply() {
    for home in HomeSide::ALL {
        let mut white_done = ugolki::initial();
        white_done.white = ugolki::home(Color::Black);
        white_done.black = (ugolki::home(Color::White) & !sq("d3").bit()) | sq("e3").bit();
        white_done.side = Color::Black;
        let pending = for_home(
            State {
                pos: white_done,
                ply: 79,
            },
            home,
        );
        assert_eq!(pending.outcome(1), None);
        let map = |s| {
            if home == HomeSide::Left {
                s
            } else {
                mirror_square(s)
            }
        };
        let equal = find(pending, map(sq("e3")), map(sq("d3"))).unwrap();
        assert_eq!(
            pending.after(&equal).outcome(3),
            Some(Outcome::Draw(FinishReason::Simultaneous))
        );
        let miss = find(pending, map(sq("e3")), map(sq("e4"))).unwrap();
        assert_eq!(
            pending.after(&miss).outcome(1),
            Some(Outcome::Win {
                winner: Color::White,
                reason: FinishReason::HomeFilled
            })
        );
        let mut black_first = pending.after(&equal);
        black_first.pos.white &= !map(sq("h8")).bit();
        put(&mut black_first.pos, map(sq("d5")), Color::White);
        assert_eq!(
            black_first.outcome(1),
            Some(Outcome::Win {
                winner: Color::Black,
                reason: FinishReason::HomeFilled
            })
        );
    }
}

#[test]
fn deadlines_repetition_and_blocked_endings_work_for_both_homes() {
    for home in HomeSide::ALL {
        for (position, result) in [
            (
                "W:Wa1:Be4",
                Outcome::Win {
                    winner: Color::Black,
                    reason: FinishReason::HomeDeadline,
                },
            ),
            (
                "W:We4:Bh8",
                Outcome::Win {
                    winner: Color::White,
                    reason: FinishReason::HomeDeadline,
                },
            ),
            ("W:Wa1:Bh8", Outcome::Draw(FinishReason::HomeDeadline)),
        ] {
            let state = for_home(
                State {
                    ply: 78,
                    ..fen(position)
                },
                home,
            );
            assert_eq!(state.outcome(1), None);
            assert_eq!(State { ply: 80, ..state }.outcome(1), Some(result));
        }
        for (position, result) in [
            (
                "W:Wh8,g5:Be4",
                Outcome::Win {
                    winner: Color::White,
                    reason: FinishReason::MoveLimit,
                },
            ),
            (
                "W:We4:Ba1,b4",
                Outcome::Win {
                    winner: Color::Black,
                    reason: FinishReason::MoveLimit,
                },
            ),
            ("W:Wh8:Ba1", Outcome::Draw(FinishReason::MoveLimit)),
        ] {
            let state = for_home(
                State {
                    ply: 158,
                    ..fen(position)
                },
                home,
            );
            assert_eq!(state.outcome(1), None);
            assert_eq!(State { ply: 160, ..state }.outcome(1), Some(result));
        }
        assert_eq!(
            for_home(fen("W:We4:Bc5"), home).outcome(3),
            Some(Outcome::Draw(FinishReason::Repetition))
        );
        let mut stuck = empty(Color::White, home);
        stuck.white = mask(&["a1", "b1", "c1", "a2", "b2", "c2", "a3", "b3", "c3"]);
        stuck.side = Color::Black; // An empty side cannot pass or create a move.
        assert_eq!(
            State::new(stuck).outcome(1),
            Some(Outcome::Draw(FinishReason::Blocked))
        );
    }
}

#[test]
fn settings_records_and_sessions_remember_layout_independently_of_colour() {
    for home in HomeSide::ALL {
        for color in [Color::White, Color::Black] {
            let settings = Settings {
                game: GameKind::Ugolki,
                ugolki_home_side: home,
                human: color,
                level: 9,
                ..Settings::default()
            };
            assert_eq!(
                Settings::from_text(&settings.to_text(), Settings::default()),
                settings
            );
            assert_eq!(Settings::from_text("ugolki_home=invalid", settings), settings);
            let mut tree = GameTree::from_position(ugolki::initial_with_home(home));
            let moves = tree.current_node().state().legal_moves();
            tree.play(&moves[0]).unwrap();
            tree.go_start();
            tree.play(&moves[1]).unwrap();
            let text =
                games::record::to_pdn(&tree, &games::record::standard_tags(color, "Human", "Computer"));
            assert!(text.contains(&format!("[UgolkiHome \"{}\"]", home.code())));
            let loaded = games::record::from_pdn(&text).unwrap();
            assert!(loaded.tree.same_structure(&tree));
            assert_eq!(loaded.human_color(), Some(color));
            let mut session = Session::for_game(GameKind::RussianDraughts, color, 9, 4);
            let human = loaded.human_color();
            session.load(loaded.tree, human);
            assert_eq!(session.tree().root().position().home_side, home);
            assert_eq!(session.level(), 9);
        }
    }
    let old = games::record::from_pdn("[GameType \"Ugolki\"]\n1. c3-c4 *").unwrap();
    assert_eq!(old.tree.root().position().home_side, HomeSide::Left);
    assert!(games::record::from_pdn("[GameType \"Ugolki\"]\n[UgolkiHome \"invalid\"]\n*").is_err());
    let custom = games::record::from_pdn(
        "[GameType \"Ugolki\"]\n[UgolkiHome \"right\"]\n[FEN \"W:Wa1,b1:Bd1\"]\n1. a1-c1-e1 *",
    )
    .unwrap();
    assert_eq!(custom.tree.root().position().home_side, HomeSide::Right);
    let text = games::record::to_pdn(&custom.tree, &[]);
    assert!(games::record::from_pdn(&text)
        .unwrap()
        .tree
        .same_structure(&custom.tree));
    assert_eq!(
        Settings::from_text("game=ugolki", Settings::default()).ugolki_home_side,
        HomeSide::Left
    );
}

#[test]
fn all_ten_ai_levels_play_legal_moves_for_either_colour_and_home() {
    let stop = AtomicBool::new(false);
    for home in HomeSide::ALL {
        for color in [Color::White, Color::Black] {
            let mut state = State::new(ugolki::initial_with_home(home));
            if color == Color::Black {
                state = state.after(&state.legal_moves()[0]);
            }
            for level in 1..=10 {
                let started = Instant::now();
                let mv = ugolki::ai::choose_move(state, &[], level, 907 + u64::from(level), &stop).unwrap();
                assert!(state.legal_moves().contains(&mv));
                assert!(reference(state).contains(&(mv.from(), mv.to())));
                assert!(started.elapsed() < Duration::from_secs(5));
            }
            let cancelled = AtomicBool::new(true);
            assert!(ugolki::ai::choose_move(state, &[], 10, 1, &cancelled).is_none());
            let mut pos = empty(color, home);
            let goal = ugolki::home_for(color.opponent(), home);
            let (outside, inside) = match (home, color) {
                (HomeSide::Left, Color::White) => ("d6", "e6"),
                (HomeSide::Right, Color::White) => ("e6", "d6"),
                (HomeSide::Left, Color::Black) => ("e3", "d3"),
                (HomeSide::Right, Color::Black) => ("d3", "e3"),
            };
            for square in games::squares_of(goal & !sq(inside).bit()) {
                put(&mut pos, square, color);
            }
            put(&mut pos, sq(outside), color);
            put(&mut pos, sq("a4"), color.opponent());
            let ready = State { pos, ply: 20 };
            let mv = ugolki::ai::choose_move(ready, &[], 8, 907, &stop).unwrap();
            assert_eq!(mv.to(), sq(inside));
            assert_eq!(ready.after(&mv).arrived(color), 12);
        }
    }
}

#[test]
fn repetitions_are_branch_local_and_survive_undo_and_reload() {
    for home in HomeSide::ALL {
        let state = for_home(fen("W:We4:Bc5"), home);
        let map = |s| {
            if home == HomeSide::Left {
                s
            } else {
                mirror_square(s)
            }
        };
        let cycle = [("e4", "f4"), ("c5", "b5"), ("f4", "e4"), ("b5", "c5")];
        let mut tree = GameTree::from_position(state.pos);
        for (from, to) in cycle {
            tree.play(&Move::route(map(sq(from)), &[map(sq(to))])).unwrap();
        }
        assert_eq!(tree.current_node().occurrences(), 2);
        assert!(tree.current_node().outcome().is_none());
        tree.go_start();
        // The same root position in a neighbouring variation must not count here.
        for (from, to) in [("e4", "e3"), ("c5", "c6"), ("e3", "e4"), ("c6", "c5")] {
            tree.play(&Move::route(map(sq(from)), &[map(sq(to))])).unwrap();
        }
        assert_eq!(tree.current_node().occurrences(), 2);
        for (from, to) in cycle {
            tree.play(&Move::route(map(sq(from)), &[map(sq(to))])).unwrap();
        }
        assert_eq!(tree.current_node().occurrences(), 3);
        assert_eq!(
            tree.current_node().outcome(),
            Some(Outcome::Draw(FinishReason::Repetition))
        );
        assert!(tree.play(&Move::route(map(sq("e4")), &[map(sq("f4"))])).is_err());
        let text = games::record::to_pdn(&tree, &[]);
        assert!(games::record::from_pdn(&text).unwrap().tree.same_structure(&tree));
        assert!(tree.back());
        assert!(tree.current_node().outcome().is_none());
        assert!(tree.forward());
        assert_eq!(tree.current_node().occurrences(), 3);
    }
}
