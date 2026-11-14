//! The computer opponent: legal moves and the time limit on every level, tactics on the
//! high levels, awareness of the draw rules, games against itself and the superiority
//! of strong levels over weak ones.
//!
//! Long checks are marked `#[ignore]` and are run with
//! `cargo test --release -- --ignored`.

mod common;

use common::{mv, play_all, pos, winning_moves, wins_within};
use rucheckers::ai::search::{search, SearchParams, WIN_BOUND};
use rucheckers::ai::worker::{JobStatus, SearchJob};
use rucheckers::ai::{choose_move, level_settings, MAX_LEVEL, MIN_LEVEL, MOVE_TIME_LIMIT_MS};
use rucheckers::board::{Color, Position};
use rucheckers::moves::{legal_moves, Move};
use rucheckers::notation::format_move;
use rucheckers::rng::Rng;
use rucheckers::rules::{DrawCounters, GameState, Outcome};
use rucheckers::tree::GameTree;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const INITIAL: &str = "W:Wa1,c1,e1,g1,b2,d2,f2,h2,a3,c3,e3,g3:Bb6,d6,f6,h6,a7,c7,e7,g7,b8,d8,f8,h8";

/// Positions of different kinds: opening, middle game, endgames with kings, a choice of captures.
const POSITIONS: [&str; 9] = [
    INITIAL,
    "B:Wa1,c1,e1,g1,b2,d2,f2,h2,a3,e3,g3,d4:Bb6,d6,f6,h6,a7,c7,e7,g7,b8,d8,f8,h8",
    "W:Wc1,e1,b2,f2,a3,c3,e3,d4:Bb6,f6,h6,a7,c7,e7,d8,f8",
    "B:Wa1,c1,g1,d2,h2,e3,g3,f4:Bb6,d6,h6,a7,e7,g7,b8,f8",
    "W:WKa1,Kc1,e3:BKh8,Kf8,b6",
    "B:WKa1,Kc1,Ke1:BKd4",
    "W:WKc1:Bd2,b2,b4",
    "W:Wc3:Bd4,f6",
    "B:Wb2,f2,Kc3:Bh2,d6,f6",
];

const fn never() -> AtomicBool {
    AtomicBool::new(false)
}

fn pick(state: &GameState, history: &[Position], level: u8, seed: u64) -> Move {
    choose_move(state, history, level, seed, &never())
        .expect("the move must exist")
        .mv
}

/// Chooses a move and checks that it is legal and how long the calculation took.
fn timed_pick(fen: &str, level: u8, seed: u64) -> (Move, Duration) {
    let state = GameState::new(pos(fen));
    let legal = legal_moves(&state.pos);
    let started = Instant::now();
    let chosen = pick(&state, &[], level, seed);
    let elapsed = started.elapsed();
    assert!(
        legal.contains(&chosen),
        "level {level}: illegal move {chosen:?} in {fen}"
    );
    assert!(
        elapsed < Duration::from_millis(MOVE_TIME_LIMIT_MS),
        "level {level}: the calculation took {elapsed:?} in {fen}"
    );
    (chosen, elapsed)
}

// ---------------------------------------------------------------------------
// Legal moves and the time limit
// ---------------------------------------------------------------------------

#[test]
fn levels_1_to_8_return_legal_moves_quickly() {
    for level in 1..=8 {
        for (i, fen) in POSITIONS.iter().enumerate() {
            for seed in 0..3 {
                timed_pick(fen, level, 1000 * u64::from(level) + 10 * i as u64 + seed);
            }
        }
    }
}

#[test]
fn level_9_returns_legal_moves_within_the_time_limit() {
    for (i, fen) in [INITIAL, POSITIONS[2], POSITIONS[4]].iter().enumerate() {
        timed_pick(fen, 9, 90 + i as u64);
    }
}

#[test]
fn level_10_returns_a_legal_move_within_five_seconds() {
    let (_, elapsed) = timed_pick(POSITIONS[2], 10, 10);
    // The tenth level uses its time instead of answering at once.
    assert!(elapsed > Duration::from_millis(500), "{elapsed:?}");
}

#[test]
fn weak_levels_answer_in_milliseconds() {
    for level in 1..=5 {
        let started = Instant::now();
        for fen in POSITIONS {
            timed_pick(fen, level, 5);
        }
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "level {level}: {:?}",
            started.elapsed()
        );
    }
}

#[test]
fn no_move_is_returned_only_when_there_are_none() {
    for level in MIN_LEVEL..=MAX_LEVEL {
        let lost = GameState::new(pos("W:Wa1:Bb2,c3"));
        assert!(choose_move(&lost, &[], level, 1, &never()).is_none());
        // The only move is returned at once, without a search.
        let forced = GameState::new(pos("W:Wc3:Bd4,f6"));
        let choice = choose_move(&forced, &[], level, 1, &never()).unwrap();
        assert_eq!(format_move(&choice.mv), "c3:e5:g7");
        assert_eq!(choice.nodes, 0);
    }
    // A level number out of range is brought into range.
    let state = GameState::initial();
    assert!(legal_moves(&state.pos).contains(&pick(&state, &[], 0, 1)));
    assert!(legal_moves(&state.pos).contains(&pick(&state, &[], 77, 1)));
    assert_eq!(level_settings(77), level_settings(MAX_LEVEL));
}

#[test]
fn search_can_be_cancelled_from_outside() {
    let state = GameState::new(pos(POSITIONS[2]));
    // The stop signal is set in advance: the search returns at once and still gives a move.
    let stop = AtomicBool::new(true);
    let started = Instant::now();
    let choice = choose_move(&state, &[], 10, 1, &stop).unwrap();
    assert!(started.elapsed() < Duration::from_millis(500));
    assert!(legal_moves(&state.pos).contains(&choice.mv));

    // Stopping during the calculation.
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let handle = std::thread::spawn(move || {
        let started = Instant::now();
        let choice = choose_move(&state, &[], 10, 2, &flag);
        (choice, started.elapsed())
    });
    std::thread::sleep(Duration::from_millis(150));
    stop.store(true, Ordering::Relaxed);
    let (choice, elapsed) = handle.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(1_500),
        "the calculation stopped after {elapsed:?}"
    );
    assert!(choice.is_some());
}

#[test]
fn background_job_reports_the_result_and_stops_when_dropped() {
    let state = GameState::initial();
    let job = SearchJob::start(state, Vec::new(), 3, 99);
    let started = Instant::now();
    let choice = loop {
        match job.poll() {
            JobStatus::Running => {
                assert!(started.elapsed() < Duration::from_secs(5));
                std::thread::sleep(Duration::from_millis(2));
            }
            JobStatus::Done(choice) => break choice,
            JobStatus::Failed => panic!("the calculation thread did not answer"),
        }
    };
    assert!(legal_moves(&state.pos).contains(&choice.unwrap().mv));
    // After the answer has been received the channel is closed.
    assert!(matches!(job.poll(), JobStatus::Failed));

    // A long calculation is interrupted when its job is dropped; new jobs still work.
    let long = SearchJob::start(GameState::new(pos(POSITIONS[2])), Vec::new(), 10, 1);
    assert!(matches!(long.poll(), JobStatus::Running));
    drop(long);
    let quick = SearchJob::start(state, Vec::new(), 1, 5);
    let started = Instant::now();
    while matches!(quick.poll(), JobStatus::Running) {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(2));
    }
}

// ---------------------------------------------------------------------------
// Weak levels make mistakes, strong ones do not
// ---------------------------------------------------------------------------

/// Fraction of the cases in which a level plays a move that at once leaves a piece to be
/// captured for free.
fn hanging_rate(level: u8, trials: u64) -> f64 {
    // The move c3-d4 gives a piece away for nothing (e5:c3); the other moves are safe.
    let state = GameState::new(pos("W:Wa1,c3,g1,h2:Be5,f6,g7,h8"));
    let blunder = mv(&state.pos, "c3-d4");
    let mut count = 0;
    for seed in 0..trials {
        if pick(&state, &[], level, seed) == blunder {
            count += 1;
        }
    }
    f64::from(count) / trials as f64
}

#[test]
fn level_1_makes_obvious_mistakes_and_strong_levels_do_not() {
    let novice = hanging_rate(1, 300);
    assert!(
        novice > 0.08,
        "the first level blunders a piece noticeably often: {novice}"
    );
    let middle = hanging_rate(4, 300);
    assert!(
        middle < novice,
        "the fourth level errs less often than the first: {middle} versus {novice}"
    );
    for level in 7..=9 {
        assert_eq!(
            hanging_rate(level, 12),
            0.0,
            "level {level} does not give a piece away for nothing"
        );
    }
}

#[test]
fn every_level_takes_the_last_piece_when_it_can() {
    // The capture is mandatory and the only one: even the first level finishes the game.
    for level in MIN_LEVEL..=MAX_LEVEL {
        let state = GameState::new(pos("W:Wc3,a1:Bd4"));
        assert_eq!(format_move(&pick(&state, &[], level, 3)), "c3:e5");
    }
}

// ---------------------------------------------------------------------------
// Tactics: a forced win in several moves
// ---------------------------------------------------------------------------

/// Positions with a single winning move. The depth of the win in plies and the move
/// itself are confirmed in the test by an independent full search (`common::wins_within`).
const WINS_IN_5: [(&str, &str); 6] = [
    ("B:Wh2,g5,Kf8:BKa3,b6", "b6-c5"),
    ("B:Wc1,b2,h4,a5:Ba3,e3,b6,c7", "e3-d2"),
    ("W:Wg1,f2,h2,a5:Bf4,h4,f6,c7", "f2-g3"),
    ("B:Wa1,d2,f2,d4,h4,c7:Ba3,f6,g7,h8", "f6-g5"),
    ("W:Wc1,b2,d2,c3,g3,a5:Ba3,c5,Kg5,a7", "c3-d4"),
    ("W:Wa3,e3,b4,e5,b6,a7:BKg1,e7,g7,b8", "e3-d4"),
];

const WINS_IN_7: [(&str, &str); 5] = [
    ("W:Wa1,g3,h4,a7:BKa3,h6,e7", "a1-b2"),
    ("W:Wc1,b2,d2,f2:Bd4,f4,b6,d6", "d2-e3"),
    ("B:Wb2,f2,Kc3:Bh2,d6,f6", "h2-g1"),
    ("W:Wd4,h6,Kh8:BKa1,g3,f8", "d4-e5"),
    ("W:Wc1,d2,Kg7:Bb4,Kf4,b6", "d2-c3"),
];

/// Checks the problem itself: there is a win in exactly `plies` plies and the key move is unique.
fn verified_key_move(fen: &str, key: &str, plies: u32) -> (GameState, Move) {
    let position = pos(fen);
    assert!(wins_within(&position, plies), "{fen}: no win in {plies} plies");
    assert!(
        !wins_within(&position, plies - 2),
        "{fen}: the win is shorter than {plies} plies"
    );
    let winning = winning_moves(&position, plies);
    assert_eq!(winning.len(), 1, "{fen}: the key move is not unique");
    assert_eq!(format_move(&winning[0]), key, "{fen}");
    (GameState::new(position), winning[0])
}

fn assert_levels_find(tasks: &[(&str, &str)], plies: u32, levels: std::ops::RangeInclusive<u8>) {
    for (fen, key) in tasks {
        let (state, key_move) = verified_key_move(fen, key, plies);
        for level in levels.clone() {
            let started = Instant::now();
            let choice = choose_move(&state, &[], level, 7, &never()).unwrap();
            assert_eq!(choice.mv, key_move, "level {level} did not find {key} in {fen}");
            let score = choice.score.unwrap();
            assert!(
                score >= WIN_BOUND,
                "level {level}: the win is not recognized, score {score}"
            );
            // Having found a forced win, the computer does not spend all its time.
            assert!(started.elapsed() < Duration::from_millis(MOVE_TIME_LIMIT_MS));
        }
    }
}

#[test]
fn strong_levels_find_a_forced_win_in_three_moves() {
    assert_levels_find(&WINS_IN_5, 5, 7..=10);
}

#[test]
fn strong_levels_find_a_forced_win_in_four_moves() {
    assert_levels_find(&WINS_IN_7, 7, 8..=10);
}

#[test]
fn search_reports_a_win_score_with_the_distance_to_it() {
    let (state, key_move) = verified_key_move(WINS_IN_5[1].0, WINS_IN_5[1].1, 5);
    let params = SearchParams {
        max_depth: 12,
        hard_time: None,
        soft_time: None,
        quiescence: true,
        jitter: 0,
        seed: 1,
        table_bits: 16,
        exact_root_scores: false,
    };
    let result = search(&state, &[], &params, &never());
    assert_eq!(result.best, Some(key_move));
    assert_eq!(
        result.score,
        rucheckers::ai::search::WIN - 5,
        "a win at the fifth ply"
    );
    assert!(!result.aborted);
    // From the losing side's point of view the score is symmetric.
    let after = state.play(&key_move);
    let reply = search(&after, &[], &params, &never());
    assert_eq!(reply.score, -(rucheckers::ai::search::WIN - 4));
}

// ---------------------------------------------------------------------------
// The draw rules in the search
// ---------------------------------------------------------------------------

#[test]
fn computer_avoids_a_draw_by_the_kings_only_rule_when_it_is_ahead() {
    // White has two kings and a man against a king and a man. The move h4-g5 gives the man away.
    let start = pos("W:WKc1,Ke1,h4:BKb8,f6");
    let sacrifice = mv(&start, "h4-g5");
    for level in 8..=9 {
        // Normally there is no reason to give the man away.
        let fresh = GameState::new(start);
        assert_ne!(pick(&fresh, &[], level, 1), sacrifice, "level {level}");
        // But if 29 plies have been made by kings, any king move is a draw by rule 3, and
        // the advantage can be kept only by moving the man.
        let stale = GameState {
            pos: start,
            counters: DrawCounters {
                kings_only: 29,
                ..DrawCounters::default()
            },
        };
        let choice = choose_move(&stale, &[], level, 1, &never()).unwrap();
        assert_eq!(choice.mv, sacrifice, "level {level}");
        assert!(choice.score.unwrap() > 0, "level {level}: {:?}", choice.score);
    }
}

/// A game in which the kings shuttle back and forth: in the current position Black's
/// move `a7-b8` leads to the third repetition.
fn shuffle_tree(fen: &str) -> GameTree {
    let mut tree = GameTree::from_position(pos(fen));
    play_all(
        &mut tree,
        &["g1-h2", "b8-a7", "h2-g1", "a7-b8", "g1-h2", "b8-a7", "h2-g1"],
    );
    assert_eq!(tree.current_node().outcome(), None);
    tree
}

#[test]
fn computer_takes_a_draw_by_repetition_when_it_is_behind() {
    // Black has one king against a king and three men.
    let tree = shuffle_tree("W:WKg1,a1,c1,e1:BKb8");
    let node = tree.current();
    let state = *tree.current_node().state();
    let history = tree.history_before(node);
    let repeat = mv(&state.pos, "a7-b8");
    assert_eq!(history.len(), 7);
    for level in 7..=9 {
        let choice = choose_move(&state, &history, level, 3, &never()).unwrap();
        assert_eq!(choice.mv, repeat, "level {level} must repeat the position");
        assert_eq!(
            choice.score,
            Some(0),
            "level {level}: the repetition is scored as a draw"
        );
        // Without the history of the game the same move gives no draw, and the position is
        // scored as bad.
        let blind = choose_move(&state, &[], level, 3, &never()).unwrap();
        assert!(blind.score.unwrap() < 0, "level {level}: {:?}", blind.score);
    }
}

#[test]
fn computer_avoids_a_draw_by_repetition_when_it_is_ahead() {
    // Now Black has a king and three men against a lone king.
    let tree = shuffle_tree("W:WKg1:BKb8,d8,f8,h8");
    let node = tree.current();
    let state = *tree.current_node().state();
    let history = tree.history_before(node);
    let repeat = mv(&state.pos, "a7-b8");
    for level in 7..=9 {
        let choice = choose_move(&state, &history, level, 3, &never()).unwrap();
        assert_ne!(choice.mv, repeat, "level {level} must not agree to a repetition");
        assert!(choice.score.unwrap() > 0, "level {level}: {:?}", choice.score);
    }
}

// ---------------------------------------------------------------------------
// Games of the computer against itself
// ---------------------------------------------------------------------------

struct GameRecord {
    outcome: Outcome,
    moves: Vec<String>,
}

/// A game between two levels. Every move is checked for legality, and the history is
/// kept in a game tree, the same way as in the application.
fn self_play(white: u8, black: u8, seed: u64) -> GameRecord {
    let mut rng = Rng::new(seed);
    let mut tree = GameTree::new();
    let mut moves = Vec::new();
    loop {
        let node = tree.current_node();
        if let Some(outcome) = node.outcome() {
            return GameRecord { outcome, moves };
        }
        assert!(moves.len() < 1_000, "the game does not end");
        let state = *node.state();
        let level = if state.pos.side == Color::White {
            white
        } else {
            black
        };
        let history = tree.history_before(tree.current());
        let chosen = pick(&state, &history, level, rng.next_u64());
        assert!(
            legal_moves(&state.pos).contains(&chosen),
            "illegal move {chosen:?}\n{}",
            state.pos
        );
        moves.push(format_move(&chosen));
        tree.play(&chosen).unwrap();
    }
}

/// Points of the first level in a match of `games` games with alternating colours (a win is
/// 2, a draw is 1).
fn match_points(first: u8, second: u8, games: u64, seed: u64) -> (u64, u64) {
    let (mut a, mut b) = (0, 0);
    for game in 0..games {
        let first_is_white = game % 2 == 0;
        let (white, black) = if first_is_white {
            (first, second)
        } else {
            (second, first)
        };
        match self_play(white, black, seed + game).outcome.winner() {
            Some(winner) if (winner == Color::White) == first_is_white => a += 2,
            Some(_) => b += 2,
            None => {
                a += 1;
                b += 1;
            }
        }
    }
    (a, b)
}

#[test]
fn self_play_games_finish_without_failures() {
    for (i, &(white, black)) in [(1, 1), (2, 3), (3, 2), (4, 5), (5, 6), (6, 4), (7, 7)]
        .iter()
        .enumerate()
    {
        let record = self_play(white, black, 500 + i as u64);
        assert!(
            record.moves.len() >= 10,
            "levels {white}–{black}: the game is too short"
        );
    }
}

#[test]
fn games_differ_from_one_another() {
    // The same seed gives the same game; different seeds give different games.
    for level in [2, 5, 7] {
        let a = self_play(level, level, 1);
        let b = self_play(level, level, 1);
        let c = self_play(level, level, 2);
        assert_eq!(
            a.moves, b.moves,
            "level {level}: the game is reproducible with the same seed"
        );
        assert_ne!(a.moves, c.moves, "level {level}: the games must not coincide");
    }
    // A strong level does not play one and the same opening either: the first moves vary.
    let state = GameState::initial();
    let mut first_moves: Vec<String> = (0..16)
        .map(|seed| format_move(&pick(&state, &[], 8, seed)))
        .collect();
    first_moves.sort();
    first_moves.dedup();
    assert!(
        first_moves.len() >= 2,
        "first moves of the eighth level: {first_moves:?}"
    );
}

#[test]
fn stronger_levels_beat_weaker_ones() {
    // Matches with a large gap between levels: the strong one must win confidently.
    let (strong, weak) = match_points(7, 1, 10, 100);
    assert!(strong >= 18 && weak <= 2, "7 against 1: {strong}:{weak}");
    let (strong, weak) = match_points(5, 2, 12, 200);
    assert!(strong >= 18, "5 against 2: {strong}:{weak}");
    let (strong, weak) = match_points(4, 1, 12, 300);
    assert!(strong >= 18, "4 against 1: {strong}:{weak}");
}

// ---------------------------------------------------------------------------
// Long checks (cargo test --release -- --ignored)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "long check: the time limit on all levels and positions"]
fn long_every_level_stays_within_the_time_limit_on_every_position() {
    for level in MIN_LEVEL..=MAX_LEVEL {
        for (i, fen) in POSITIONS.iter().enumerate() {
            timed_pick(fen, level, 7_000 + 100 * u64::from(level) + i as u64);
        }
    }
}

#[test]
#[ignore = "long check: the ladder of levels, each next one stronger than the previous"]
fn long_each_level_outscores_the_previous_one() {
    for level in MIN_LEVEL..8 {
        let (strong, weak) = match_points(level + 1, level, 40, 9_000 + u64::from(level));
        assert!(
            strong > weak + 12,
            "level {} against {}: {strong}:{weak}",
            level + 1,
            level
        );
    }
    let (strong, weak) = match_points(9, 8, 16, 9_900);
    assert!(strong > weak, "level 9 against 8: {strong}:{weak}");
}

#[test]
#[ignore = "long check: games of the tenth level (several minutes)"]
fn long_top_level_beats_middle_levels_and_finishes_self_play() {
    let (strong, weak) = match_points(10, 6, 4, 9_950);
    assert!(strong >= 6, "level 10 against 6: {strong}:{weak}");
    let record = self_play(10, 10, 9_999);
    assert!(record.moves.len() >= 20);
}

#[test]
#[ignore = "long check: tactics and draws on the tenth level"]
fn long_top_level_handles_draw_rules() {
    let tree = shuffle_tree("W:WKg1,a1,c1,e1:BKb8");
    let state = *tree.current_node().state();
    let history = tree.history_before(tree.current());
    let choice = choose_move(&state, &history, 10, 3, &never()).unwrap();
    assert_eq!(choice.mv, mv(&state.pos, "a7-b8"));
    assert_eq!(choice.score, Some(0));

    let tree = shuffle_tree("W:WKg1:BKb8,d8,f8,h8");
    let state = *tree.current_node().state();
    let history = tree.history_before(tree.current());
    let choice = choose_move(&state, &history, 10, 3, &never()).unwrap();
    assert_ne!(choice.mv, mv(&state.pos, "a7-b8"));
}
