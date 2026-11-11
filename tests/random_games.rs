//! Случайные партии: тысячи партий из случайных допустимых ходов с фиксированным зерном,
//! доведённых до конца, с проверкой инвариантов после каждого хода.

mod common;

use common::outcome_with_full_history;
use rucheckers::board::{Color, Kind, Position, Square, RANK_1, RANK_8};
use rucheckers::moves::{generate_captures, has_capture, legal_moves};
use rucheckers::notation::{format_move, parse_move};
use rucheckers::rng::Rng;
use rucheckers::rules::{GameState, Outcome};
use rucheckers::tree::GameTree;

/// Верхняя граница длины партии в полуходах: правила ничьей гарантируют окончание
/// намного раньше, превышение означает ошибку.
const MAX_PLIES: usize = 2_000;

#[derive(Default)]
struct Stats {
    games: u32,
    plies: u64,
    white_wins: u32,
    black_wins: u32,
    draws: u32,
    captures: u64,
    promotions: u64,
    longest_capture: u32,
}

/// Проверка расстановки: шашки только на тёмных полях, нет простых на дамочной горизонтали.
fn assert_position_invariants(pos: &Position) {
    assert!(pos.is_valid(), "{pos}");
    assert_eq!(pos.white & pos.black, 0);
    assert_eq!(pos.kings & !pos.occupied(), 0);
    assert_eq!(
        pos.men(Color::White) & RANK_8,
        0,
        "белая простая на восьмой горизонтали\n{pos}"
    );
    assert_eq!(
        pos.men(Color::Black) & RANK_1,
        0,
        "чёрная простая на первой горизонтали\n{pos}"
    );
    let mut counted = 0;
    for rank in 0..8u8 {
        for file in 0..8u8 {
            match Square::from_file_rank(file, rank) {
                Some(sq) => {
                    assert_eq!((file + rank) % 2, 0, "игровое поле должно быть тёмным");
                    if let Some(piece) = pos.piece_at(sq) {
                        counted += 1;
                        if piece.kind == Kind::Man {
                            let last = if piece.color == Color::White { 7 } else { 0 };
                            assert_ne!(rank, last, "простая на дамочной горизонтали\n{pos}");
                        }
                    }
                }
                None => assert_eq!((file + rank) % 2, 1),
            }
        }
    }
    assert_eq!(counted, pos.total(), "все шашки стоят на тёмных полях");
}

/// Играет одну случайную партию. Параллельно ведётся дерево партии: его результат
/// сверяется с независимым подсчётом повторений по полной истории.
fn play_random_game(rng: &mut Rng, stats: &mut Stats) -> Outcome {
    let mut state = GameState::initial();
    let mut earlier: Vec<Position> = Vec::new();
    let mut tree = GameTree::new();
    for _ply in 0..MAX_PLIES {
        let pos = state.pos;
        assert_position_invariants(&pos);

        let expected = outcome_with_full_history(&state, &earlier);
        assert_eq!(
            tree.current_node().outcome(),
            expected,
            "дерево и прямой подсчёт расходятся\n{pos}"
        );
        assert_eq!(tree.current_node().state(), &state);
        if let Some(outcome) = expected {
            return outcome;
        }

        let moves = legal_moves(&pos);
        assert!(!moves.is_empty());
        // Взятие обязательно: если оно есть, все допустимые ходы — взятия.
        let mut captures = Vec::new();
        generate_captures(&pos, &mut captures);
        assert_eq!(has_capture(&pos, pos.side), !captures.is_empty());
        if captures.is_empty() {
            assert!(moves.iter().all(|m| !m.is_capture()));
        } else {
            assert_eq!(moves, captures);
        }
        // Запись каждого хода однозначно разбирается обратно.
        for m in &moves {
            assert_eq!(parse_move(&format_move(m), &moves), Ok(*m), "{pos}");
        }

        let mv = *rng.pick(&moves).unwrap();
        let before_total = pos.total();
        let before_own = pos.count(pos.side);

        // Ход и его отмена восстанавливают позицию.
        let mut scratch = pos;
        let undo = scratch.make(&mv);
        let after = scratch;
        scratch.unmake(&mv, undo);
        assert_eq!(scratch, pos, "ход {mv:?} и его отмена не восстановили позицию");
        assert_eq!(after, pos.after(&mv));

        // Очередь хода чередуется, число шашек не растёт.
        assert_eq!(after.side, pos.side.opponent());
        assert_eq!(after.total(), before_total - mv.captured_count());
        assert_eq!(after.count(pos.side), before_own, "своих шашек столько же");
        assert!(after.total() <= before_total);
        // Побиты только чужие шашки, и каждая — ровно один раз.
        assert_eq!(mv.captured() & pos.pieces(pos.side), 0);
        assert_eq!(mv.captured() & !pos.pieces(pos.side.opponent()), 0);
        if mv.is_capture() {
            assert_eq!(mv.captured_count() as usize, mv.path().len());
            assert_eq!(mv.captured_in_order().len(), mv.path().len());
            stats.captures += 1;
            stats.longest_capture = stats.longest_capture.max(mv.captured_count());
        } else {
            assert_eq!(mv.path().len(), 1);
        }
        // Превращение: простая пришла на последнюю горизонталь и стала дамкой.
        let was_king = pos.kings & mv.from().bit() != 0;
        let is_king = after.kings & mv.to().bit() != 0;
        assert_eq!(is_king, was_king || mv.promotes());
        if mv.promotes() {
            assert!(!was_king);
            stats.promotions += 1;
        }

        earlier.push(pos);
        state = state.play(&mv);
        assert_eq!(state.pos, after);
        tree.play(&mv).unwrap();
        stats.plies += 1;
    }
    panic!("партия не закончилась за {MAX_PLIES} полуходов");
}

fn run_games(seed: u64, games: u32) -> Stats {
    let mut rng = Rng::new(seed);
    let mut stats = Stats::default();
    for _ in 0..games {
        match play_random_game(&mut rng, &mut stats) {
            Outcome::Win {
                winner: Color::White, ..
            } => stats.white_wins += 1,
            Outcome::Win {
                winner: Color::Black, ..
            } => stats.black_wins += 1,
            Outcome::Draw(_) => stats.draws += 1,
        }
        stats.games += 1;
    }
    stats
}

#[test]
fn three_thousand_random_games_keep_all_invariants() {
    let stats = run_games(0x00C0_FFEE, 3_000);
    assert_eq!(stats.games, 3_000);
    assert_eq!(stats.white_wins + stats.black_wins + stats.draws, 3_000);
    // Случайная игра должна задействовать все части правил.
    assert!(
        stats.white_wins > 100 && stats.black_wins > 100,
        "победы обеих сторон"
    );
    assert!(stats.draws > 10, "ничьи встречаются: {}", stats.draws);
    assert!(stats.captures > 30_000 && stats.promotions > 3_000);
    assert!(
        stats.longest_capture >= 4,
        "встречаются длинные серии: {}",
        stats.longest_capture
    );
    assert!(stats.plies / u64::from(stats.games) > 30);
}

#[test]
fn random_games_are_reproducible_for_a_fixed_seed() {
    let a = run_games(42, 50);
    let b = run_games(42, 50);
    let c = run_games(43, 50);
    assert_eq!(
        (a.plies, a.white_wins, a.black_wins, a.draws),
        (b.plies, b.white_wins, b.black_wins, b.draws)
    );
    assert_ne!(
        (a.plies, a.captures),
        (c.plies, c.captures),
        "другое зерно — другие партии"
    );
}

/// Долгая проверка: запускается отдельно командой `cargo test -- --ignored`.
#[test]
#[ignore = "долгая проверка: сто тысяч случайных партий"]
fn hundred_thousand_random_games_keep_all_invariants() {
    let stats = run_games(0x0DEC_0DE5, 100_000);
    assert_eq!(stats.games, 100_000);
    assert!(stats.longest_capture >= 6, "{}", stats.longest_capture);
}
