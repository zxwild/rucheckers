//! Random games: thousands of games of random legal moves with a fixed seed, played to
//! the end, with invariants checked after every move.

mod common;

use common::outcome_with_full_history;
use rucheckers::board::{Color, Kind, Position, Square, RANK_1, RANK_8};
use rucheckers::moves::{generate_captures, has_capture, legal_moves};
use rucheckers::notation::{format_move, parse_move};
use rucheckers::rng::Rng;
use rucheckers::rules::{GameState, Outcome};
use rucheckers::tree::GameTree;

/// Upper bound on the length of a game in plies: the draw rules guarantee an end much
/// earlier, so exceeding it means a bug.
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

/// Checks the placement: pieces on dark squares only, no men on their promotion rank.
fn assert_position_invariants(pos: &Position) {
    assert!(pos.is_valid(), "{pos}");
    assert_eq!(pos.white & pos.black, 0);
    assert_eq!(pos.kings & !pos.occupied(), 0);
    assert_eq!(
        pos.men(Color::White) & RANK_8,
        0,
        "a white man on the eighth rank\n{pos}"
    );
    assert_eq!(
        pos.men(Color::Black) & RANK_1,
        0,
        "a black man on the first rank\n{pos}"
    );
    let mut counted = 0;
    for rank in 0..8u8 {
        for file in 0..8u8 {
            match Square::from_file_rank(file, rank) {
                Some(sq) => {
                    assert_eq!((file + rank) % 2, 0, "a playable square must be dark");
                    if let Some(piece) = pos.piece_at(sq) {
                        counted += 1;
                        if piece.kind == Kind::Man {
                            let last = if piece.color == Color::White { 7 } else { 0 };
                            assert_ne!(rank, last, "a man on its promotion rank\n{pos}");
                        }
                    }
                }
                None => assert_eq!((file + rank) % 2, 1),
            }
        }
    }
    assert_eq!(counted, pos.total(), "all pieces stand on dark squares");
}

/// Plays one random game. A game tree is kept in parallel: its result is compared with
/// an independent count of repetitions over the full history.
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
            "the tree and the direct count disagree\n{pos}"
        );
        assert_eq!(tree.current_node().state(), &state);
        if let Some(outcome) = expected {
            return outcome;
        }

        let moves = legal_moves(&pos);
        assert_ne!(moves, []);
        // Capturing is mandatory: if there is a capture, all legal moves are captures.
        let mut captures = Vec::new();
        generate_captures(&pos, &mut captures);
        assert_eq!(has_capture(&pos, pos.side), !captures.is_empty());
        if captures.is_empty() {
            assert!(moves.iter().all(|m| !m.is_capture()));
        } else {
            assert_eq!(moves, captures);
        }
        // The notation of every move parses back unambiguously.
        for m in &moves {
            assert_eq!(parse_move(&format_move(m), &moves), Ok(*m), "{pos}");
        }

        let mv = *rng.pick(&moves).unwrap();
        let before_total = pos.total();
        let before_own = pos.count(pos.side);

        // Making a move and taking it back restores the position.
        let mut scratch = pos;
        let undo = scratch.make(&mv);
        let after = scratch;
        scratch.unmake(&mv, undo);
        assert_eq!(
            scratch, pos,
            "making {mv:?} and taking it back did not restore the position"
        );
        assert_eq!(after, pos.after(&mv));

        // The side to move alternates; the number of pieces does not grow.
        assert_eq!(after.side, pos.side.opponent());
        assert_eq!(after.total(), before_total - mv.captured_count());
        assert_eq!(
            after.count(pos.side),
            before_own,
            "the mover keeps all its pieces"
        );
        assert!(after.total() <= before_total);
        // Only enemy pieces are captured, and each exactly once.
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
        // Promotion: a man has reached the last rank and become a king.
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
    panic!("the game did not end within {MAX_PLIES} plies");
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
    // Random play must exercise every part of the rules.
    assert!(
        stats.white_wins > 100 && stats.black_wins > 100,
        "wins by both sides"
    );
    assert!(stats.draws > 10, "draws occur: {}", stats.draws);
    assert!(stats.captures > 30_000 && stats.promotions > 3_000);
    assert!(
        stats.longest_capture >= 4,
        "long multi-captures occur: {}",
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
        "another seed gives other games"
    );
}

/// A long check: run separately with `cargo test -- --ignored`.
#[test]
#[ignore = "long check: a hundred thousand random games"]
fn hundred_thousand_random_games_keep_all_invariants() {
    let stats = run_games(0x0DEC_0DE5, 100_000);
    assert_eq!(stats.games, 100_000);
    assert!(stats.longest_capture >= 6, "{}", stats.longest_capture);
}
