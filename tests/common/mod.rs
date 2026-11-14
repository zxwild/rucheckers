//! Helper functions shared by the integration tests.
#![allow(dead_code)]

use rucheckers::board::{Position, Square};
use rucheckers::moves::{has_capture, legal_moves, Move};
use rucheckers::notation::{format_move, parse_move};
use rucheckers::rules::{occurrences, GameState, Outcome};
use rucheckers::tree::{GameTree, NodeId};

/// A position from its FEN text.
pub fn pos(fen: &str) -> Position {
    Position::from_fen(fen).unwrap_or_else(|e| panic!("FEN {fen:?}: {e}"))
}

/// A square by its name.
pub fn sq(name: &str) -> Square {
    Square::parse(name).unwrap_or_else(|| panic!("square {name:?}"))
}

/// The sorted notations of all legal moves.
pub fn move_names(pos: &Position) -> Vec<String> {
    let mut names: Vec<String> = legal_moves(pos).iter().map(format_move).collect();
    names.sort();
    names
}

/// The move with the given notation among the legal moves of a position.
pub fn mv(pos: &Position, text: &str) -> Move {
    parse_move(text, &legal_moves(pos))
        .unwrap_or_else(|e| panic!("move {text:?} in the position\n{pos}\n{e}"))
}

/// Plays a move in a tree by its notation.
pub fn play(tree: &mut GameTree, text: &str) -> NodeId {
    let m = mv(tree.current_node().position(), text);
    tree.play(&m).unwrap_or_else(|e| panic!("move {text:?}: {e}"))
}

/// Plays several moves in a row in a tree.
pub fn play_all(tree: &mut GameTree, texts: &[&str]) {
    for text in texts {
        play(tree, text);
    }
}

/// The result in the current node of a tree.
pub fn outcome(tree: &GameTree) -> Option<Outcome> {
    tree.current_node().outcome()
}

/// Looks for a sequence of `plies` quiet moves in which:
/// * every move satisfies `allow(ply number from 1, position before the move, move)`;
/// * neither side ever has a capture;
/// * positions do not repeat (and do not coincide with any of `avoid`);
/// * after every move the opponent has moves.
///
/// The search is deterministic (backtracking).
pub fn quiet_line(
    start: &Position,
    avoid: &[Position],
    plies: usize,
    allow: &dyn Fn(usize, &Position, &Move) -> bool,
) -> Vec<Move> {
    #[allow(clippy::too_many_arguments)]
    fn go(
        pos: &Position,
        ply: usize,
        plies: usize,
        seen: &mut Vec<Position>,
        line: &mut Vec<Move>,
        allow: &dyn Fn(usize, &Position, &Move) -> bool,
        budget: &mut u32,
    ) -> bool {
        if ply > plies {
            return true;
        }
        if *budget == 0 {
            return false;
        }
        *budget -= 1;
        for m in legal_moves(pos) {
            if m.is_capture() || !allow(ply, pos, &m) {
                continue;
            }
            let next = pos.after(&m);
            if seen.contains(&next) || has_capture(&next, next.side) || legal_moves(&next).is_empty() {
                continue;
            }
            seen.push(next);
            line.push(m);
            if go(&next, ply + 1, plies, seen, line, allow, budget) {
                return true;
            }
            seen.pop();
            line.pop();
        }
        false
    }
    assert!(
        !has_capture(start, start.side),
        "the starting position has a capture:\n{start}"
    );
    let mut seen = avoid.to_vec();
    seen.push(*start);
    let mut line = Vec::new();
    let mut budget = 2_000_000;
    assert!(
        go(start, 1, plies, &mut seen, &mut line, allow, &mut budget),
        "no quiet line of {plies} plies found in the position\n{start}"
    );
    line
}

/// Whether this move is made by a king.
pub const fn is_king_move(pos: &Position, m: &Move) -> bool {
    pos.kings & m.from().bit() != 0
}

/// The result of a state given all earlier positions of the game (without
/// optimizations): a check of repetitions that is independent of the tree.
pub fn outcome_with_full_history(state: &GameState, earlier: &[Position]) -> Option<Outcome> {
    state.outcome(occurrences(earlier, &state.pos))
}

/// The simplest independent solver: whether the side to move can force the opponent out
/// of moves in at most `plies` plies. The draw rules are not taken into account.
pub fn wins_within(pos: &Position, plies: u32) -> bool {
    if plies == 0 {
        return false;
    }
    legal_moves(pos)
        .iter()
        .any(|m| loses_within(&pos.after(m), plies - 1))
}

/// Whether the side to move loses in at most `plies` plies whatever it plays.
pub fn loses_within(pos: &Position, plies: u32) -> bool {
    let moves = legal_moves(pos);
    if moves.is_empty() {
        return true;
    }
    if plies == 0 {
        return false;
    }
    moves.iter().all(|m| wins_within(&pos.after(m), plies - 1))
}

/// The first moves that win by force in at most `plies` plies.
pub fn winning_moves(pos: &Position, plies: u32) -> Vec<Move> {
    legal_moves(pos)
        .into_iter()
        .filter(|m| loses_within(&pos.after(m), plies - 1))
        .collect()
}
