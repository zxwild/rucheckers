//! Coordinate notation, delegated to the selected game where needed.

use super::{russian, Move, Square};

#[must_use]
pub fn format_move(mv: &Move) -> String {
    if let Some(native) = mv.russian {
        return russian::notation::format_move(&native);
    }
    mv.squares().map(Square::name).collect::<Vec<_>>().join("-")
}
/// Matches coordinate notation to one of the legal moves.
///
/// # Errors
/// Returns a syntax, illegal-move or ambiguity error.
pub fn parse_move(text: &str, legal: &[Move]) -> Result<Move, russian::notation::NotationError> {
    if legal.first().is_some_and(|mv| mv.russian.is_some()) {
        let native: Vec<_> = legal.iter().filter_map(|mv| mv.russian).collect();
        return russian::notation::parse_move(text, &native).map(Move::from_russian);
    }
    let names: Vec<_> = text
        .trim()
        .trim_end_matches(['!', '?', '+', '#'])
        .split(['-', ':', 'x', 'X', '×'])
        .map(Square::parse)
        .collect();
    if names.len() < 2 || names.iter().any(Option::is_none) {
        return Err(russian::notation::NotationError::Syntax);
    }
    let path: Vec<_> = names.into_iter().flatten().collect();
    if let Some(mv) = legal.iter().find(|mv| mv.squares().eq(path.iter().copied())) {
        return Ok(*mv);
    }
    if path.len() != 2 {
        return Err(russian::notation::NotationError::Illegal);
    }
    let mut matching = legal
        .iter()
        .filter(|mv| mv.from() == path[0] && mv.to() == path[1]);
    let first = matching.next().ok_or(russian::notation::NotationError::Illegal)?;
    if matching.any(|mv| mv.captured() != first.captured() || mv.promotes() != first.promotes()) {
        return Err(russian::notation::NotationError::Ambiguous);
    }
    Ok(*first)
}
