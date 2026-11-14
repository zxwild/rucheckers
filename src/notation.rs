//! Russian draughts notation: `c3-d4` is a quiet move, `c3:e5:g7` is a capture.

use crate::board::Square;
use crate::moves::Move;
use std::fmt;

/// Writes a move: a quiet move with a hyphen, a capture with colons and every square of the route.
#[must_use]
pub fn format_move(mv: &Move) -> String {
    let sep = if mv.is_capture() { ':' } else { '-' };
    let mut out = String::with_capacity(3 * (mv.path().len() + 1));
    out.push_str(&mv.from().name());
    for sq in mv.path() {
        out.push(sep);
        out.push_str(&sq.name());
    }
    out
}

/// Error of parsing a move.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotationError {
    /// The text does not look like a move.
    Syntax,
    /// There is no such move in the position.
    Illegal,
    /// The text matches several different captures.
    Ambiguous,
}

impl fmt::Display for NotationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Syntax => "not a move",
            Self::Illegal => "illegal move",
            Self::Ambiguous => "ambiguous capture",
        })
    }
}

impl std::error::Error for NotationError {}

/// Parses a sequence of squares separated by `-`, `:` or `x`.
///
/// # Errors
///
/// Returns [`NotationError::Syntax`] if the text is not such a sequence of at least
/// two playable squares.
pub fn parse_squares(text: &str) -> Result<Vec<Square>, NotationError> {
    let text = text.trim().trim_end_matches(['!', '?', '+', '#']);
    let mut squares = Vec::new();
    for part in text.split(['-', ':', 'x', 'X', '×']) {
        squares.push(Square::parse(part).ok_or(NotationError::Syntax)?);
    }
    if squares.len() < 2 {
        return Err(NotationError::Syntax);
    }
    Ok(squares)
}

/// Finds among the legal moves `legal` the one written in `text`.
///
/// The full notation (`c3:e5:g7`) is accepted and, for captures, a shortened one as
/// well (`c3:g7`, or with some of the intermediate squares) as long as it determines
/// the result of the move unambiguously.
///
/// # Errors
///
/// Returns [`NotationError::Syntax`] if the text is not a move at all,
/// [`NotationError::Illegal`] if no legal move matches it and
/// [`NotationError::Ambiguous`] if captures with different results match it.
pub fn parse_move(text: &str, legal: &[Move]) -> Result<Move, NotationError> {
    let squares = parse_squares(text)?;
    let from = squares[0];
    let to = squares[squares.len() - 1];

    // An exact match of the route.
    if let Some(mv) = legal
        .iter()
        .find(|m| m.from() == from && m.path() == &squares[1..])
    {
        return Ok(*mv);
    }

    // A shortened capture: the given squares appear in the route in the same order.
    let inner = &squares[1..squares.len() - 1];
    let mut found: Option<Move> = None;
    for mv in legal {
        if !mv.is_capture() || mv.from() != from || mv.to() != to {
            continue;
        }
        let mut rest = inner.iter().peekable();
        for sq in &mv.path()[..mv.path().len() - 1] {
            if rest.peek() == Some(&sq) {
                rest.next();
            }
        }
        if rest.peek().is_some() {
            continue;
        }
        match found {
            None => found = Some(*mv),
            Some(prev) => {
                // Different routes are acceptable only if they lead to the same position.
                if prev.captured() != mv.captured() || prev.promotes() != mv.promotes() {
                    return Err(NotationError::Ambiguous);
                }
            }
        }
    }
    found.ok_or(NotationError::Illegal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Position;
    use crate::moves::legal_moves;

    fn moves(fen: &str) -> Vec<Move> {
        legal_moves(&Position::from_fen(fen).unwrap())
    }

    #[test]
    fn quiet_moves_use_a_dash() {
        let legal = legal_moves(&Position::initial());
        let mut names: Vec<String> = legal.iter().map(format_move).collect();
        names.sort();
        assert_eq!(
            names,
            ["a3-b4", "c3-b4", "c3-d4", "e3-d4", "e3-f4", "g3-f4", "g3-h4"]
        );
        for mv in &legal {
            assert_eq!(parse_move(&format_move(mv), &legal), Ok(*mv));
        }
    }

    #[test]
    fn captures_use_colons_and_list_the_route() {
        let legal = moves("W:Wc3:Bd4,f6");
        assert_eq!(legal.len(), 1);
        assert_eq!(format_move(&legal[0]), "c3:e5:g7");
        assert_eq!(parse_move("c3:e5:g7", &legal), Ok(legal[0]));
        assert_eq!(parse_move("C3xE5xG7", &legal), Ok(legal[0]));
        assert_eq!(parse_move("c3:g7", &legal), Ok(legal[0]), "shortened notation");
        assert_eq!(
            parse_move("c3:e5", &legal),
            Err(NotationError::Illegal),
            "the capture is not carried to its end"
        );
        assert_eq!(
            parse_move("c3-b4", &legal),
            Err(NotationError::Illegal),
            "capturing is mandatory"
        );
    }

    #[test]
    fn garbage_is_a_syntax_error() {
        let legal = legal_moves(&Position::initial());
        for text in [
            "",
            "c3",
            "c3-",
            "-d4",
            "c3--d4",
            "b1-c2",
            "日本語",
            "c3-d4-",
            "1-0",
            "c3 d4",
            "z9:y8",
        ] {
            assert_eq!(parse_move(text, &legal), Err(NotationError::Syntax), "{text:?}");
        }
        assert_eq!(parse_move("a1-b2", &legal), Err(NotationError::Illegal));
        assert_eq!(
            parse_move("c3-d4!", &legal).map(|m| format_move(&m)),
            Ok("c3-d4".to_string())
        );
    }

    #[test]
    fn short_capture_notation_is_accepted_when_the_result_is_the_same() {
        // The king goes round a rhombus of four pieces clockwise or anticlockwise. The
        // routes differ, but the captured pieces and the last square are the same, so
        // the shortened notation is accepted.
        let legal = moves("W:WKd2:Bc3,c5,e5,e3");
        let names: Vec<String> = legal.iter().map(format_move).collect();
        assert!(names.contains(&"d2:f4:d6:b4:d2".to_string()), "{names:?}");
        assert!(names.contains(&"d2:b4:d6:f4:d2".to_string()), "{names:?}");
        let short = parse_move("d2:d2", &legal).unwrap();
        assert_eq!(short.captured_count(), 4);
        assert_eq!(short.to(), short.from());
        // The intermediate square b4 is in both routes; the result of the move does not
        // depend on it.
        let via_b4 = parse_move("d2:b4:d2", &legal).unwrap();
        assert_eq!(via_b4.captured(), short.captured());
        assert!(via_b4.path().contains(&Square::parse("b4").unwrap()));
        // The text cannot contain a square that is in neither route.
        assert_eq!(parse_move("d2:h6:d2", &legal), Err(NotationError::Illegal));
        // The full notation of each move is parsed back into that move.
        for mv in &legal {
            assert_eq!(parse_move(&format_move(mv), &legal), Ok(*mv));
        }
    }

    #[test]
    fn short_capture_notation_is_rejected_when_ambiguous() {
        // Two captures with the same first and last squares but different captured pieces.
        // The list is made of moves from two positions: the parsing logic itself is tested.
        let mut list = moves("W:WKc1:Bd2,e5");
        list.extend(moves("W:WKc1:Bb2,c5"));
        let names: Vec<String> = list.iter().map(format_move).collect();
        assert!(names.contains(&"c1:f4:d6".to_string()), "{names:?}");
        assert!(names.contains(&"c1:a3:d6".to_string()), "{names:?}");
        assert_eq!(parse_move("c1:d6", &list), Err(NotationError::Ambiguous));
        assert_eq!(
            parse_move("c1:f4:d6", &list).map(|m| format_move(&m)),
            Ok("c1:f4:d6".to_string())
        );
        assert_eq!(
            parse_move("c1:a3:d6", &list).map(|m| format_move(&m)),
            Ok("c1:a3:d6".to_string())
        );
        // The last square c7 can be reached by one route only.
        assert_eq!(
            parse_move("c1:c7", &list).map(|m| format_move(&m)),
            Ok("c1:f4:c7".to_string())
        );
    }
}
