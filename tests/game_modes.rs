use rucheckers::games::session::Session;
use rucheckers::games::tree::GameTree;
use rucheckers::games::ugolki::{self, State};
use rucheckers::games::{self, Color, FinishReason, GameKind, Kind, Move, Outcome, Position, Square};
use rucheckers::settings::Settings;
use std::sync::atomic::AtomicBool;

fn sq(name: &str) -> Square {
    Square::parse(name).unwrap()
}
fn state(fen: &str) -> State {
    State::new(Position::from_fen_for(GameKind::Ugolki, fen).unwrap())
}
fn has(state: State, from: &str, to: &str) -> bool {
    state
        .legal_moves()
        .iter()
        .any(|mv| mv.from() == sq(from) && mv.to() == sq(to))
}

#[test]
fn ugolki_uses_twelve_pieces_and_both_cell_colours() {
    let position = ugolki::initial();
    assert_eq!(position.count(Color::White), 12);
    assert_eq!(position.count(Color::Black), 12);
    assert_eq!(position.white & position.black, 0);
    assert_eq!(position.kings, 0);
    assert_eq!(position.side, Color::White);
    assert_eq!(position.piece_at(sq("b1")).unwrap().kind, Kind::Man);
    assert!(position.piece_at(sq("d3")).is_some());
    assert!(position.piece_at(sq("e6")).is_some());
    assert_eq!(State::new(position).legal_moves().len(), 14);
}

#[test]
fn steps_are_orthogonal_and_jump_chains_can_stop_at_any_endpoint() {
    let position = state("W:Wa1,b1:Bd1");
    assert!(has(position, "a1", "a2"));
    assert!(has(position, "a1", "c1"));
    assert!(has(position, "a1", "e1"));
    assert!(!has(position, "a1", "b2"));
    let mv = position
        .legal_moves()
        .into_iter()
        .find(|mv| mv.from() == sq("a1") && mv.to() == sq("e1"))
        .unwrap();
    assert_eq!(mv.path(), &[sq("c1"), sq("e1")]);
    assert!(!mv.is_capture());
    let after = position.after(&mv);
    assert_eq!(after.pos.count(Color::White), 2);
    assert_eq!(after.pos.count(Color::Black), 1);
    assert_eq!(after.pos.piece_at(sq("d1")).unwrap().color, Color::Black);
    assert_eq!(after.pos.side, Color::Black);
}

#[test]
fn jump_paths_never_revisit_a_cell_or_mix_in_an_ordinary_step() {
    let position = state("W:Wa1,b1,c2:Bb3,a2,d1");
    for mv in position.legal_moves() {
        let mut visited = mv.from().bit();
        let mut previous = mv.from();
        for &next in mv.path() {
            assert_eq!(visited & next.bit(), 0);
            visited |= next.bit();
            let dx = previous.file().abs_diff(next.file());
            let dy = previous.rank().abs_diff(next.rank());
            assert!(dx == 0 || dy == 0);
            assert!((dx + dy) == 2 || (mv.path().len() == 1 && (dx + dy) == 1));
            if dx + dy == 2 {
                let over = Square::from_file_rank(
                    (previous.file() + next.file()) / 2,
                    (previous.rank() + next.rank()) / 2,
                )
                .unwrap();
                assert_ne!(position.pos.occupied() & over.bit(), 0);
            }
            previous = next;
        }
    }
}

#[test]
fn an_enemy_cannot_be_locked_by_a_two_deep_cross() {
    let position = state("W:Wc4,b4,e4,f4,d3,d2,d5,e6:Bd4");
    assert!(!has(position, "e6", "d6"));
    assert!(has(position, "e6", "f6"));
}

#[test]
fn leaving_home_deadline_is_checked_after_blacks_equal_reply() {
    let mut position = state("W:Wa1:Be4");
    position.ply = 78;
    assert_eq!(position.outcome(1), None);
    position.ply = 80;
    assert_eq!(
        position.outcome(1),
        Some(Outcome::Win {
            winner: Color::Black,
            reason: FinishReason::HomeDeadline
        })
    );
    let mut both = state("W:Wa1:Bh8");
    both.ply = 80;
    assert_eq!(both.outcome(1), Some(Outcome::Draw(FinishReason::HomeDeadline)));
    both.pos.side = Color::Black;
    both.ply = 79;
    assert_eq!(both.outcome(1), None);
}

#[test]
fn late_home_returns_are_forbidden_but_jumping_through_is_allowed() {
    let mut position = state("W:Wa4:Bh8");
    position.ply = 78;
    assert!(has(position, "a4", "a3"));
    position.ply = 80;
    assert!(!has(position, "a4", "a3"));
    let mut transit = state("W:Wa4:Ba3,b2,c3,h4");
    transit.ply = 80;
    assert!(has(transit, "a4", "c4"));
}

#[test]
fn black_can_equalise_a_white_finish_and_a_black_finish_wins_immediately() {
    let mut position = ugolki::initial();
    position.white = ugolki::home(Color::Black);
    position.black = (ugolki::home(Color::White) & !sq("d3").bit()) | sq("e3").bit();
    position.side = Color::Black;
    let pending = State {
        pos: position,
        ply: 49,
    };
    assert_eq!(pending.outcome(1), None);
    let equal = Move::route(sq("e3"), &[sq("d3")]);
    assert!(pending.legal_moves().contains(&equal));
    assert_eq!(
        pending.after(&equal).outcome(1),
        Some(Outcome::Draw(FinishReason::Simultaneous))
    );
    let miss = Move::route(sq("e3"), &[sq("e4")]);
    assert_eq!(
        pending.after(&miss).outcome(1),
        Some(Outcome::Win {
            winner: Color::White,
            reason: FinishReason::HomeFilled
        })
    );
    let mut black_first = pending.after(&equal);
    black_first.pos.white &= !sq("h8").bit();
    black_first.pos.white |= sq("d5").bit();
    assert_eq!(
        black_first.outcome(1),
        Some(Outcome::Win {
            winner: Color::Black,
            reason: FinishReason::HomeFilled
        })
    );
}

#[test]
fn repetitions_and_move_limit_end_a_race() {
    let position = state("W:We4:Bc5");
    assert_eq!(position.outcome(3), Some(Outcome::Draw(FinishReason::Repetition)));
    let mut limit = state("W:Wh8,g5:Be4");
    limit.ply = 160;
    assert_eq!(
        limit.outcome(1),
        Some(Outcome::Win {
            winner: Color::White,
            reason: FinishReason::MoveLimit
        })
    );
    limit.pos.black = sq("a1").bit();
    assert_eq!(limit.outcome(1), Some(Outcome::Draw(FinishReason::MoveLimit)));
}

#[test]
fn ugolki_ai_has_ten_legal_cancellable_levels_and_finishes_the_home() {
    let initial = State::new(ugolki::initial());
    let stop = AtomicBool::new(false);
    for level in 1..=10 {
        let mv = ugolki::ai::choose_move(initial, &[], level, 100 + u64::from(level), &stop).unwrap();
        assert!(initial.legal_moves().contains(&mv));
        let settings = ugolki::ai::level_settings(level);
        assert!(settings.budget_ms < rucheckers::ai::MOVE_TIME_LIMIT_MS);
    }
    let cancelled = AtomicBool::new(true);
    assert!(ugolki::ai::choose_move(initial, &[], 10, 1, &cancelled).is_none());
    let mut finish = ugolki::initial();
    finish.white = (ugolki::home(Color::Black) & !sq("e6").bit()) | sq("d6").bit();
    finish.black = sq("a4").bit();
    let ready = State { pos: finish, ply: 20 };
    let mv = ugolki::ai::choose_move(ready, &[], 10, 9, &stop).unwrap();
    assert_eq!(ready.after(&mv).arrived(Color::White), 12);
}

#[test]
fn mode_aware_records_preserve_variations_and_human_colour() {
    for game in GameKind::ALL {
        let mut tree = GameTree::for_game(game);
        let legal = tree.current_node().state().legal_moves();
        tree.play(&legal[0]).unwrap();
        let next = tree.current_node().state().legal_moves()[0];
        tree.play(&next).unwrap();
        tree.go_start();
        tree.play(&legal[1]).unwrap();
        let tags = games::record::standard_tags(Color::Black, "Human", "Computer");
        let text = games::record::to_pdn(&tree, &tags);
        let loaded = games::record::from_bytes(text.as_bytes()).unwrap();
        assert_eq!(loaded.tree.root().position().game, game);
        assert_eq!(loaded.human_color(), Some(Color::Black));
        assert!(loaded.tree.same_structure(&tree));
    }
}

#[test]
fn shared_session_accepts_light_cells_and_cancels_the_old_games_search() {
    let mut session = Session::for_game(GameKind::Ugolki, Color::White, 4, 123);
    assert!(session.grab(sq("c3")));
    session.drop_on(sq("c4"));
    assert!(session
        .tree()
        .current_node()
        .position()
        .piece_at(sq("c4"))
        .is_some());
    assert_eq!(session.tree().len(), 2);
    session.update(0.0);
    session.new_game(GameKind::RussianDraughts, Color::White, 4);
    session.update(100.0);
    assert_eq!(session.game(), GameKind::RussianDraughts);
    assert_eq!(session.tree().len(), 1);
    assert!(session
        .tree()
        .current_node()
        .position()
        .piece_at(sq("c3"))
        .is_some());
}

#[test]
fn saved_mode_and_all_other_preferences_survive_restarting() {
    let settings = Settings {
        game: GameKind::Ugolki,
        level: 8,
        human: Color::Black,
        sound: false,
        panel: false,
        ..Settings::default()
    };
    let restored = Settings::from_text(&settings.to_text(), Settings::default());
    assert_eq!(restored, settings);
    assert_eq!(
        Settings::from_text("level=6\ncolor=black", Settings::default()).game,
        GameKind::RussianDraughts
    );
    assert_eq!(
        Settings::from_text("game=unknown", settings).game,
        GameKind::Ugolki
    );
}

#[test]
fn every_cell_is_clickable_in_both_board_orientations() {
    use rucheckers::ui::geometry::BoardLayout;
    for flipped in [false, true] {
        for density in [1.0, 1.5, 2.0] {
            let layout = BoardLayout::fit(13.0, 19.0, 601.0, 527.0, flipped, density);
            for square in Square::all() {
                let (x, y) = layout.square_center(square);
                assert_eq!(layout.game_square_at(x, y), Some(square));
            }
            assert_eq!(layout.game_square_at(-1.0, -1.0), None);
        }
    }
}

#[test]
fn shared_draughts_facade_preserves_moves_results_and_animations() {
    use games::anim::MoveAnimation;
    use rucheckers::rng::Rng;
    let mut rng = Rng::new(987);
    for _ in 0..12 {
        let mut native = rucheckers::rules::GameState::initial();
        let mut shared = games::GameState::new(Position::initial());
        for _ in 0..180 {
            assert_eq!(shared.pos.draughts(), native.pos);
            let outcome = native.outcome(1);
            assert_eq!(shared.outcome(1), outcome.map(Outcome::Russian));
            if outcome.is_some() {
                break;
            }
            let native_moves = rucheckers::moves::legal_moves(&native.pos);
            let shared_moves = shared.legal_moves();
            assert_eq!(
                shared_moves,
                native_moves
                    .iter()
                    .copied()
                    .map(Move::from_russian)
                    .collect::<Vec<_>>()
            );
            let mv = *rng.pick(&native_moves).unwrap();
            let presented = Move::from_russian(mv);
            let piece = native.pos.piece_at(mv.from()).unwrap();
            let original_anim = rucheckers::ui::anim::MoveAnimation::new(mv, piece, 0, 1.0);
            let shared_anim = MoveAnimation::new(presented, piece, 0, 1.0);
            assert_eq!(original_anim.travel_seconds(), shared_anim.travel_seconds());
            assert_eq!(original_anim.promotion(), shared_anim.promotion());
            for time in [1.0, 1.1, 1.3, 1.9, 3.0] {
                let before = original_anim.frame(time);
                let after = shared_anim.frame(time);
                assert_eq!(
                    (before.file, before.rank, before.is_king, before.finished),
                    (after.file, after.rank, after.is_king, after.finished)
                );
                let expanded = rucheckers::board::squares_of(before.jumped)
                    .fold(0, |bits, sq| bits | Square::from(sq).bit());
                assert_eq!(expanded, after.jumped);
            }
            native = native.play(&mv);
            shared = shared.after(&presented);
        }
    }
}

#[test]
fn random_ugolki_games_keep_all_pieces_and_restore_counters_from_records() {
    use rucheckers::rng::Rng;
    let mut rng = Rng::new(739);
    for game in 0..40 {
        let mut tree = GameTree::for_game(GameKind::Ugolki);
        let mut finished = false;
        for _ in 0..=160 {
            let node = tree.current_node();
            let pos = node.position();
            assert_eq!((pos.count(Color::White), pos.count(Color::Black)), (12, 12));
            assert_eq!(pos.white & pos.black, 0);
            assert_eq!(pos.kings, 0);
            if node.outcome().is_some() {
                finished = true;
                break;
            }
            let legal = node.state().legal_moves();
            let mv = *rng.pick(&legal).unwrap();
            assert_eq!(pos.occupied() & mv.to().bit(), 0);
            tree.play(&mv).unwrap();
        }
        assert!(finished, "game {game} exceeded the 80-move limit");
        let text = games::record::to_pdn(&tree, &[]);
        let loaded = games::record::from_bytes(text.as_bytes()).unwrap();
        assert!(loaded.tree.same_structure(&tree));
        let last = tree.main_line_end();
        assert_eq!(
            tree.get(last).unwrap().state(),
            loaded.tree.get(last).unwrap().state()
        );
        assert_eq!(
            tree.get(last).unwrap().outcome(),
            loaded.tree.get(last).unwrap().outcome()
        );
    }
}
