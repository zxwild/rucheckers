//! Saving and loading a game in the PDN format (Portable Draughts Notation).
//!
//! One game is written with all its variations. The game type is Russian draughts:
//! `GameType "25,W,8,8,A0,0"`. Moves are written in alphanumeric notation, captures
//! with colons and the full route, variations in parentheses after the move they are
//! an alternative to.
//!
//! Parsing does not panic on arbitrary data; it returns a [`PdnError`] with a line
//! number, from which the interface builds a message the player can understand.

use crate::board::{Color, Position};
use crate::moves::legal_moves;
use crate::notation::{format_move, parse_move, NotationError};
use crate::rules::Outcome;
use crate::tree::{GameTree, Node, NodeId, TreeError};
use std::fmt;
use std::ops::ControlFlow;
use std::time::{SystemTime, UNIX_EPOCH};

/// Value of the `GameType` tag for Russian draughts.
pub const GAME_TYPE: &str = "25,W,8,8,A0,0";
/// Number of Russian draughts in the PDN standard.
pub const GAME_TYPE_NUMBER: u32 = 25;
/// Files larger than this are not read.
pub const MAX_FILE_SIZE: usize = 4 * 1024 * 1024;
/// Greatest number of moves (tree nodes) in a loaded game.
pub const MAX_NODES: usize = 100_000;

/// What exactly is wrong with a move.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveProblem {
    /// The text is not a move.
    Syntax,
    /// The move is impossible in the position.
    Illegal,
    /// A shortened capture is ambiguous.
    Ambiguous,
    /// The move is written after the end of the game.
    GameOver,
}

/// Error of loading PDN.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PdnError {
    /// The file is too large.
    TooLarge,
    /// The file has neither tags nor moves.
    Empty,
    /// The contents do not look like PDN.
    NotPdn,
    /// A game of another draughts variant (the value of the `GameType` tag).
    WrongGameType(String),
    /// A wrong starting position in the `FEN` tag.
    BadFen(String),
    /// An unterminated tag or string in the header.
    BadTag { line: usize },
    /// A move could not be parsed or played.
    BadMove {
        line: usize,
        token: String,
        problem: MoveProblem,
    },
    /// The parentheses of variations are unbalanced.
    UnbalancedVariation { line: usize },
    /// A variation starts before the first move of its line.
    VariationWithoutMove { line: usize },
    /// A comment `{…}` is not closed.
    UnterminatedComment { line: usize },
    /// The game has too many moves.
    TooManyMoves,
}

impl fmt::Display for PdnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => write!(f, "file is too large"),
            Self::Empty => write!(f, "file is empty"),
            Self::NotPdn => write!(f, "not a PDN file"),
            Self::WrongGameType(v) => write!(f, "unsupported game type '{v}'"),
            Self::BadFen(v) => write!(f, "bad start position '{v}'"),
            Self::BadTag { line } => write!(f, "line {line}: malformed tag"),
            Self::BadMove { line, token, problem } => {
                write!(f, "line {line}: bad move '{token}' ({problem:?})")
            }
            Self::UnbalancedVariation { line } => write!(f, "line {line}: unbalanced parentheses"),
            Self::VariationWithoutMove { line } => {
                write!(f, "line {line}: variation before any move")
            }
            Self::UnterminatedComment { line } => write!(f, "line {line}: unterminated comment"),
            Self::TooManyMoves => write!(f, "too many moves"),
        }
    }
}

impl std::error::Error for PdnError {}

/// A loaded game: the header tags and the tree of moves.
#[derive(Clone, Debug)]
pub struct PdnGame {
    pub tags: Vec<(String, String)>,
    pub tree: GameTree,
}

impl PdnGame {
    /// Value of a tag (the name is compared case-insensitively).
    #[must_use]
    pub fn tag(&self, name: &str) -> Option<&str> {
        find_tag(&self.tags, name)
    }

    /// The colour the human played, if the `WhiteType`/`BlackType` tags record it.
    pub fn human_color(&self) -> Option<Color> {
        let white = self.tag("WhiteType").map(str::to_ascii_lowercase);
        let black = self.tag("BlackType").map(str::to_ascii_lowercase);
        match (white.as_deref(), black.as_deref()) {
            (Some("human"), Some("program")) => Some(Color::White),
            (Some("program"), Some("human")) => Some(Color::Black),
            _ => None,
        }
    }
}

fn find_tag<'a>(tags: &'a [(String, String)], name: &str) -> Option<&'a str> {
    tags.iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

/// The result token of the `Result` tag and of the end of the move text.
#[must_use]
pub const fn result_token(outcome: Option<Outcome>) -> &'static str {
    match outcome {
        Some(Outcome::Win {
            winner: Color::White, ..
        }) => "1-0",
        Some(Outcome::Win {
            winner: Color::Black, ..
        }) => "0-1",
        Some(Outcome::Draw(_)) => "1/2-1/2",
        None => "*",
    }
}

/// Header tags for a game between a human and the computer.
#[must_use]
pub fn standard_tags(human: Color, human_name: &str, computer_name: &str) -> Vec<(String, String)> {
    let (white, black, white_type, black_type) = match human {
        Color::White => (human_name, computer_name, "human", "program"),
        Color::Black => (computer_name, human_name, "program", "human"),
    };
    vec![
        ("Event".to_string(), "?".to_string()),
        ("Site".to_string(), "?".to_string()),
        ("Date".to_string(), today()),
        ("Round".to_string(), "?".to_string()),
        ("White".to_string(), white.to_string()),
        ("Black".to_string(), black.to_string()),
        ("WhiteType".to_string(), white_type.to_string()),
        ("BlackType".to_string(), black_type.to_string()),
    ]
}

/// Today's date (UTC) in the PDN format: `YYYY.MM.DD`.
#[must_use]
pub fn today() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    format!("{year:04}.{month:02}.{day:02}")
}

/// The Gregorian date for a number of days since 1 January 1970.
#[must_use]
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn escape_tag_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// A pending step of the tree walk in [`to_pdn`].
enum Item {
    /// Write the move of this node and continue along its line; the flag marks a move
    /// that starts a variation.
    Line(NodeId, bool),
    /// Write an opening parenthesis.
    Open,
    /// Write a closing parenthesis.
    Close,
}

/// Serializes a game tree into PDN text.
///
/// The tags `Result`, `GameType` and, for a non-standard starting position, `FEN` are
/// added automatically; tags of the same names in `tags` are ignored.
#[must_use]
pub fn to_pdn(tree: &GameTree, tags: &[(String, String)]) -> String {
    let result = result_token(tree.get(tree.main_line_end()).and_then(Node::outcome));
    let root_pos = *tree.root().position();
    let mut out = String::new();

    let reserved = ["result", "gametype", "fen", "setup"];
    let mut write_tag = |name: &str, value: &str| {
        out.push('[');
        out.push_str(name);
        out.push_str(" \"");
        out.push_str(&escape_tag_value(value));
        out.push_str("\"]\n");
    };
    for (name, value) in tags {
        let valid_name = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if valid_name && !reserved.contains(&name.to_ascii_lowercase().as_str()) {
            write_tag(name, value);
        }
    }
    write_tag("Result", result);
    write_tag("GameType", GAME_TYPE);
    if root_pos != Position::initial() {
        write_tag("SetUp", "1");
        write_tag("FEN", &root_pos.to_fen());
    }
    out.push('\n');

    // The tree is walked without recursion, so the nesting depth of variations is not
    // limited by the call stack.
    let first_index = u32::from(root_pos.side == Color::Black);
    let mut tokens: Vec<String> = Vec::new();
    let mut need_number = true;
    let mut stack: Vec<Item> = Vec::new();
    if let Some(&first) = tree.root().children().first() {
        stack.push(Item::Line(first, false));
    }
    while let Some(item) = stack.pop() {
        match item {
            Item::Open => {
                tokens.push("(".to_string());
                need_number = true;
            }
            Item::Close => {
                tokens.push(")".to_string());
                need_number = true;
            }
            Item::Line(id, is_alternative) => {
                let Some(node) = tree.get(id) else { continue };
                let Some(mv) = node.mv() else { continue };
                let index = node.ply() - 1 + first_index;
                let number = index / 2 + 1;
                if index % 2 == 0 {
                    tokens.push(format!("{number}."));
                } else if need_number {
                    tokens.push(format!("{number}..."));
                }
                need_number = false;
                tokens.push(format_move(mv));

                if let Some(&next) = node.children().first() {
                    stack.push(Item::Line(next, false));
                }
                if !is_alternative {
                    let siblings = tree.siblings(id);
                    if siblings.first() == Some(&id) {
                        for &alt in siblings[1..].iter().rev() {
                            stack.push(Item::Close);
                            stack.push(Item::Line(alt, true));
                            stack.push(Item::Open);
                        }
                    }
                }
            }
        }
    }
    tokens.push(result.to_string());

    let mut line_len = 0;
    for token in &tokens {
        if line_len > 0 && line_len + 1 + token.len() > 78 {
            out.push('\n');
            line_len = 0;
        } else if line_len > 0 {
            out.push(' ');
            line_len += 1;
        }
        out.push_str(token);
        line_len += token.len();
    }
    out.push('\n');
    out
}

/// Parses the contents of a file: checks the size and is tolerant of the encoding
/// (bytes that are not UTF-8 are replaced in comments and names; moves are always ASCII).
///
/// # Errors
///
/// Returns [`PdnError::TooLarge`] for a file over [`MAX_FILE_SIZE`] bytes and otherwise
/// the same errors as [`from_pdn`].
pub fn from_bytes(bytes: &[u8]) -> Result<PdnGame, PdnError> {
    if bytes.len() > MAX_FILE_SIZE {
        return Err(PdnError::TooLarge);
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    from_pdn(&String::from_utf8_lossy(bytes))
}

fn is_result_token(word: &str) -> bool {
    matches!(
        word,
        "*" | "1-0" | "0-1" | "1/2-1/2" | "2-0" | "0-2" | "1-1" | "0-0" | "½-½"
    )
}

fn printable(token: &str) -> String {
    let mut out: String = token
        .chars()
        .take(24)
        .map(|c| if c.is_control() { '?' } else { c })
        .collect();
    if token.chars().count() > 24 {
        out.push('…');
    }
    out
}

struct Scanner<'a> {
    text: &'a str,
    pos: usize,
    line: usize,
}

enum Token {
    Tag { name: String, value: String },
    Open,
    Close,
    Word(String),
    End,
}

impl<'a> Scanner<'a> {
    const fn new(text: &'a str) -> Self {
        Self {
            text,
            pos: 0,
            line: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
        }
        Some(ch)
    }

    fn skip_line(&mut self) {
        while let Some(ch) = self.bump() {
            if ch == '\n' {
                break;
            }
        }
    }

    fn at_line_start(&self) -> bool {
        self.pos == 0 || self.text.as_bytes()[self.pos - 1] == b'\n'
    }

    fn next(&mut self) -> Result<Token, PdnError> {
        loop {
            let Some(ch) = self.peek() else {
                return Ok(Token::End);
            };
            match ch {
                c if c.is_whitespace() => {
                    self.bump();
                }
                ';' => self.skip_line(),
                '%' if self.at_line_start() => self.skip_line(),
                '{' => {
                    let line = self.line;
                    loop {
                        match self.bump() {
                            Some('}') => break,
                            Some(_) => {}
                            None => return Err(PdnError::UnterminatedComment { line }),
                        }
                    }
                }
                '[' => return self.tag(),
                '(' => {
                    self.bump();
                    return Ok(Token::Open);
                }
                ')' => {
                    self.bump();
                    return Ok(Token::Close);
                }
                _ => {
                    let start = self.pos;
                    while let Some(c) = self.peek() {
                        if c.is_whitespace() || matches!(c, '(' | ')' | '{' | '}' | '[' | ']' | ';') {
                            break;
                        }
                        self.bump();
                    }
                    if self.pos == start {
                        // A lone special character outside its context.
                        self.bump();
                    }
                    return Ok(Token::Word(self.text[start..self.pos].to_string()));
                }
            }
        }
    }

    fn tag(&mut self) -> Result<Token, PdnError> {
        let line = self.line;
        let bad = PdnError::BadTag { line };
        self.bump(); // '['
        while self.peek().is_some_and(|c| c == ' ' || c == '\t') {
            self.bump();
        }
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
            self.bump();
        }
        let name = self.text[start..self.pos].to_string();
        if name.is_empty() {
            return Err(bad);
        }
        while self.peek().is_some_and(|c| c == ' ' || c == '\t') {
            self.bump();
        }
        if self.bump() != Some('"') {
            return Err(bad);
        }
        let mut value = String::new();
        loop {
            match self.bump() {
                Some('\\') => match self.bump() {
                    Some(c) if c != '\n' => value.push(c),
                    _ => return Err(bad),
                },
                Some('"') => break,
                Some('\n') | None => return Err(bad),
                Some(c) => value.push(c),
            }
        }
        while self.peek().is_some_and(|c| c == ' ' || c == '\t') {
            self.bump();
        }
        if self.bump() != Some(']') {
            return Err(bad);
        }
        Ok(Token::Tag { name, value })
    }
}

/// Separates the move number (`12.`, `12...`) from the move. Returns the rest of the word.
fn strip_move_number(word: &str) -> &str {
    let digits = word.len() - word.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 {
        return word;
    }
    let rest = &word[digits..];
    let after_dots = rest.trim_start_matches('.');
    if after_dots.len() < rest.len() || rest.is_empty() {
        after_dots
    } else {
        word
    }
}

/// The start position named by the header tags, after checking the game type.
fn start_position(tags: &[(String, String)]) -> Result<Position, PdnError> {
    if let Some(value) = find_tag(tags, "GameType") {
        let number = value
            .split(',')
            .next()
            .map(str::trim)
            .and_then(|n| n.parse::<u32>().ok());
        if number != Some(GAME_TYPE_NUMBER) {
            return Err(PdnError::WrongGameType(printable(value)));
        }
    }
    match find_tag(tags, "FEN") {
        Some(fen) => Position::from_fen(fen).map_err(|_| PdnError::BadFen(printable(fen))),
        None => Ok(Position::initial()),
    }
}

/// Builds the game tree from the move text: moves, variations and the result.
struct MoveText {
    tree: GameTree,
    /// The node the next move is added to.
    cursor: NodeId,
    /// For every open variation: the node to return to and whether the outer line
    /// already had a move.
    stack: Vec<(NodeId, bool)>,
    line_has_move: bool,
    /// Whether anything that belongs to the move text has been read.
    seen_movetext: bool,
    /// Whether the game has a header; without one, the first unreadable word means
    /// that the file is not PDN at all.
    has_tags: bool,
}

impl MoveText {
    fn new(start: Position, has_tags: bool) -> Self {
        Self {
            tree: GameTree::from_position(start),
            cursor: GameTree::ROOT,
            stack: Vec::new(),
            line_has_move: false,
            seen_movetext: false,
            has_tags,
        }
    }

    /// `(`: the variation is an alternative to the last move of the current line.
    fn open_variation(&mut self, line: usize) -> Result<(), PdnError> {
        self.seen_movetext = true;
        match self.tree.get(self.cursor).and_then(Node::parent) {
            Some(parent) if self.line_has_move => {
                self.stack.push((self.cursor, self.line_has_move));
                self.cursor = parent;
                self.line_has_move = false;
                Ok(())
            }
            _ => Err(PdnError::VariationWithoutMove { line }),
        }
    }

    /// `)`: back to the line the variation branched from.
    fn close_variation(&mut self, line: usize) -> Result<(), PdnError> {
        self.seen_movetext = true;
        let (resume, had_move) = self.stack.pop().ok_or(PdnError::UnbalancedVariation { line })?;
        self.cursor = resume;
        self.line_has_move = had_move;
        Ok(())
    }

    /// A word of the move text: a move (possibly with its number), an annotation or the
    /// game result. A result outside any variation ends the game.
    fn word(&mut self, word: &str, line: usize) -> Result<ControlFlow<()>, PdnError> {
        if is_result_token(word) {
            return Ok(if self.stack.is_empty() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            });
        }
        let is_annotation = word.starts_with('$') || word.chars().all(|c| matches!(c, '!' | '?' | '.'));
        if is_annotation {
            return Ok(ControlFlow::Continue(()));
        }
        let rest = strip_move_number(word);
        if !rest.is_empty() {
            self.add_move(rest, line)?;
        }
        self.seen_movetext = true;
        Ok(ControlFlow::Continue(()))
    }

    fn add_move(&mut self, text: &str, line: usize) -> Result<(), PdnError> {
        let bad = |problem| PdnError::BadMove {
            line,
            token: printable(text),
            problem,
        };
        let node = self.tree.get(self.cursor).ok_or(PdnError::NotPdn)?;
        let legal = legal_moves(node.position());
        let mv = parse_move(text, &legal).map_err(|error| match error {
            // Foreign content is recognised by its very first word.
            NotationError::Syntax if !self.seen_movetext && !self.has_tags => PdnError::NotPdn,
            NotationError::Syntax => bad(MoveProblem::Syntax),
            NotationError::Illegal => bad(MoveProblem::Illegal),
            NotationError::Ambiguous => bad(MoveProblem::Ambiguous),
        })?;
        if self.tree.len() >= MAX_NODES {
            return Err(PdnError::TooManyMoves);
        }
        self.cursor = self
            .tree
            .add_move(self.cursor, &mv)
            .map_err(|error| match error {
                TreeError::GameOver => bad(MoveProblem::GameOver),
                _ => bad(MoveProblem::Illegal),
            })?;
        self.line_has_move = true;
        Ok(())
    }

    /// The finished tree; `line` is where the text ended.
    fn finish(self, line: usize) -> Result<GameTree, PdnError> {
        if self.stack.is_empty() {
            Ok(self.tree)
        } else {
            Err(PdnError::UnbalancedVariation { line })
        }
    }
}

/// Parses PDN text. Only the first game of a file with several games is read.
///
/// # Errors
///
/// Returns a [`PdnError`] if the text is not a PDN record of a Russian draughts game:
/// it is empty or foreign, the header or the variation parentheses are malformed, or a
/// move cannot be read or played. The error carries the line number where it applies.
pub fn from_pdn(text: &str) -> Result<PdnGame, PdnError> {
    let mut scanner = Scanner::new(text);
    let mut tags: Vec<(String, String)> = Vec::new();

    // The header.
    let mut pending = loop {
        match scanner.next()? {
            Token::Tag { name, value } => tags.push((name, value)),
            other => break other,
        }
    };

    let mut moves = MoveText::new(start_position(&tags)?, !tags.is_empty());
    loop {
        let line = scanner.line;
        match pending {
            // A tag starts the next game: only the first one is read.
            Token::End | Token::Tag { .. } => break,
            Token::Open => moves.open_variation(line)?,
            Token::Close => moves.close_variation(line)?,
            Token::Word(word) => {
                if moves.word(&word, line)?.is_break() {
                    break;
                }
            }
        }
        pending = scanner.next()?;
    }
    let tree = moves.finish(scanner.line)?;

    if tags.is_empty() && tree.is_empty() {
        return Err(if text.trim().is_empty() {
            PdnError::Empty
        } else {
            PdnError::NotPdn
        });
    }
    Ok(PdnGame { tags, tree })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(31), (1970, 2, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(20_767), (2026, 11, 10));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        let date = today();
        assert_eq!(date.len(), 10);
        assert_eq!(date.matches('.').count(), 2);
    }

    #[test]
    fn move_numbers_are_separated_from_moves() {
        assert_eq!(strip_move_number("12."), "");
        assert_eq!(strip_move_number("12..."), "");
        assert_eq!(strip_move_number("12"), "");
        assert_eq!(strip_move_number("1.c3-d4"), "c3-d4");
        assert_eq!(strip_move_number("3...f6-e5"), "f6-e5");
        assert_eq!(strip_move_number("c3-d4"), "c3-d4");
        assert_eq!(strip_move_number("1-0"), "1-0");
    }

    #[test]
    fn tag_values_are_escaped() {
        assert_eq!(escape_tag_value("a\"b\\c\nd"), "a\\\"b\\\\c d");
        let tree = GameTree::new();
        let text = to_pdn(&tree, &[("White".to_string(), "José \"El Rayo\" Ñ.".to_string())]);
        let game = from_pdn(&text).unwrap();
        assert_eq!(game.tag("white"), Some("José \"El Rayo\" Ñ."));
        assert_eq!(game.tag("GameType"), Some(GAME_TYPE));
        assert_eq!(game.tag("Result"), Some("*"));
    }
}
