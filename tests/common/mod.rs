//! Общие вспомогательные функции интеграционных тестов.
#![allow(dead_code)]

use rucheckers::board::{Color, Position, Square};
use rucheckers::moves::{has_capture, legal_moves, Move};
use rucheckers::notation::{format_move, parse_move};
use rucheckers::rules::{occurrences, GameState, Outcome};
use rucheckers::tree::{GameTree, NodeId};

/// Позиция из записи FEN.
pub fn pos(fen: &str) -> Position {
    Position::from_fen(fen).unwrap_or_else(|e| panic!("FEN {fen:?}: {e}"))
}

/// Поле по имени.
pub fn sq(name: &str) -> Square {
    Square::parse(name).unwrap_or_else(|| panic!("поле {name:?}"))
}

/// Отсортированные записи всех допустимых ходов.
pub fn move_names(pos: &Position) -> Vec<String> {
    let mut names: Vec<String> = legal_moves(pos).iter().map(format_move).collect();
    names.sort();
    names
}

/// Ход по записи среди допустимых в позиции.
pub fn mv(pos: &Position, text: &str) -> Move {
    parse_move(text, &legal_moves(pos)).unwrap_or_else(|e| panic!("ход {text:?} в позиции\n{pos}\n{e}"))
}

/// Делает ход в дереве по записи.
pub fn play(tree: &mut GameTree, text: &str) -> NodeId {
    let m = mv(tree.current_node().position(), text);
    tree.play(&m).unwrap_or_else(|e| panic!("ход {text:?}: {e}"))
}

/// Делает в дереве несколько ходов подряд.
pub fn play_all(tree: &mut GameTree, texts: &[&str]) {
    for text in texts {
        play(tree, text);
    }
}

/// Результат в текущем узле дерева.
pub fn outcome(tree: &GameTree) -> Option<Outcome> {
    tree.current_node().outcome()
}

/// Ищет последовательность из `plies` тихих ходов, в которой:
/// * каждый ход удовлетворяет условию `allow(номер полухода от 1, позиция до хода, ход)`;
/// * ни у одной из сторон не возникает взятия;
/// * позиции не повторяются (в том числе не совпадают с `avoid`);
/// * после каждого хода у соперника есть ходы.
///
/// Поиск детерминирован (перебор с возвратом).
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
        "в исходной позиции есть взятие:\n{start}"
    );
    let mut seen = avoid.to_vec();
    seen.push(*start);
    let mut line = Vec::new();
    let mut budget = 2_000_000;
    assert!(
        go(start, 1, plies, &mut seen, &mut line, allow, &mut budget),
        "не найдена тихая линия из {plies} полуходов в позиции\n{start}"
    );
    line
}

/// Ходит ли этим ходом дамка.
pub fn is_king_move(pos: &Position, m: &Move) -> bool {
    pos.kings & m.from().bit() != 0
}

/// Результат состояния с учётом всех предыдущих позиций партии (без оптимизаций):
/// независимая от дерева проверка повторений.
pub fn outcome_with_full_history(state: &GameState, earlier: &[Position]) -> Option<Outcome> {
    state.outcome(occurrences(earlier, &state.pos))
}

/// Простейший независимый решатель: может ли сторона, чья очередь хода, форсированно
/// лишить соперника ходов не более чем за `plies` полуходов. Правила ничьей не учитываются.
pub fn wins_within(pos: &Position, plies: u32) -> bool {
    if plies == 0 {
        return false;
    }
    legal_moves(pos)
        .iter()
        .any(|m| loses_within(&pos.after(m), plies - 1))
}

/// Проигрывает ли сторона, чья очередь хода, не более чем за `plies` полуходов при любой игре.
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

/// Первые ходы, форсированно выигрывающие не более чем за `plies` полуходов.
pub fn winning_moves(pos: &Position, plies: u32) -> Vec<Move> {
    legal_moves(pos)
        .into_iter()
        .filter(|m| loses_within(&pos.after(m), plies - 1))
        .collect()
}

/// Цвет по-русски (для сообщений тестов).
pub fn color_name(color: Color) -> &'static str {
    match color {
        Color::White => "белые",
        Color::Black => "чёрные",
    }
}
