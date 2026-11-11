//! Сохранение и загрузка партии в формате PDN (Portable Draughts Notation).
//!
//! Записывается одна партия со всеми вариантами. Тип игры — русские шашки:
//! `GameType "25,W,8,8,A0,0"`. Ходы пишутся в буквенно-цифровой нотации, взятия —
//! через двоеточие с полным маршрутом, варианты — в круглых скобках после хода,
//! которому они служат альтернативой.
//!
//! Разбор не паникует на произвольных данных и возвращает [`PdnError`]
//! с номером строки, по которому интерфейс показывает понятное сообщение.

use crate::board::{Color, Position};
use crate::moves::legal_moves;
use crate::notation::{format_move, parse_move, NotationError};
use crate::rules::Outcome;
use crate::tree::{GameTree, NodeId, TreeError};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Значение тега `GameType` для русских шашек.
pub const GAME_TYPE: &str = "25,W,8,8,A0,0";
/// Номер русских шашек в стандарте PDN.
pub const GAME_TYPE_NUMBER: u32 = 25;
/// Файлы крупнее этого размера не читаются.
pub const MAX_FILE_SIZE: usize = 4 * 1024 * 1024;
/// Наибольшее число ходов (узлов дерева) в загружаемой партии.
pub const MAX_NODES: usize = 100_000;

/// Что именно не так с записью хода.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveProblem {
    /// Текст не является записью хода.
    Syntax,
    /// Ход невозможен в позиции.
    Illegal,
    /// Сокращённая запись взятия неоднозначна.
    Ambiguous,
    /// Ход записан после окончания партии.
    GameOver,
}

/// Ошибка загрузки PDN.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PdnError {
    /// Файл слишком велик.
    TooLarge,
    /// В файле нет ни тегов, ни ходов.
    Empty,
    /// Содержимое не похоже на PDN.
    NotPdn,
    /// Партия другого вида шашек (значение тега `GameType`).
    WrongGameType(String),
    /// Неверная начальная позиция в теге `FEN`.
    BadFen(String),
    /// Незакрытый тег или строка в заголовке.
    BadTag { line: usize },
    /// Не удалось разобрать или выполнить ход.
    BadMove {
        line: usize,
        token: String,
        problem: MoveProblem,
    },
    /// Скобки вариантов не сбалансированы.
    UnbalancedVariation { line: usize },
    /// Вариант начат до первого хода линии.
    VariationWithoutMove { line: usize },
    /// Комментарий `{…}` не закрыт.
    UnterminatedComment { line: usize },
    /// В партии слишком много ходов.
    TooManyMoves,
}

impl fmt::Display for PdnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdnError::TooLarge => write!(f, "file is too large"),
            PdnError::Empty => write!(f, "file is empty"),
            PdnError::NotPdn => write!(f, "not a PDN file"),
            PdnError::WrongGameType(v) => write!(f, "unsupported game type '{v}'"),
            PdnError::BadFen(v) => write!(f, "bad start position '{v}'"),
            PdnError::BadTag { line } => write!(f, "line {line}: malformed tag"),
            PdnError::BadMove { line, token, problem } => {
                write!(f, "line {line}: bad move '{token}' ({problem:?})")
            }
            PdnError::UnbalancedVariation { line } => write!(f, "line {line}: unbalanced parentheses"),
            PdnError::VariationWithoutMove { line } => {
                write!(f, "line {line}: variation before any move")
            }
            PdnError::UnterminatedComment { line } => write!(f, "line {line}: unterminated comment"),
            PdnError::TooManyMoves => write!(f, "too many moves"),
        }
    }
}

impl std::error::Error for PdnError {}

/// Загруженная партия: теги заголовка и дерево ходов.
#[derive(Clone, Debug)]
pub struct PdnGame {
    pub tags: Vec<(String, String)>,
    pub tree: GameTree,
}

impl PdnGame {
    /// Значение тега (имя сравнивается без учёта регистра).
    pub fn tag(&self, name: &str) -> Option<&str> {
        find_tag(&self.tags, name)
    }

    /// Цвет, которым играл человек, если он записан тегами `WhiteType`/`BlackType`.
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

/// Обозначение результата в теге `Result` и в конце записи ходов.
pub fn result_token(outcome: Option<Outcome>) -> &'static str {
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

/// Теги заголовка для партии человека с компьютером.
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

/// Сегодняшняя дата (UTC) в формате PDN: `ГГГГ.ММ.ДД`.
pub fn today() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    format!("{year:04}.{month:02}.{day:02}")
}

/// Григорианская дата по числу дней от 1 января 1970 года.
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

/// Сериализует дерево партии в текст PDN.
///
/// Теги `Result`, `GameType` и при нестандартной начальной позиции `FEN`
/// добавляются автоматически; одноимённые теги из `tags` игнорируются.
pub fn to_pdn(tree: &GameTree, tags: &[(String, String)]) -> String {
    let result = result_token(tree.get(tree.main_line_end()).and_then(|n| n.outcome()));
    let root_pos = *tree.root().position();
    let mut out = String::new();

    let reserved = ["result", "gametype", "fen", "setup"];
    let mut write_tag = |name: &str, value: &str| {
        out.push_str(&format!("[{} \"{}\"]\n", name, escape_tag_value(value)));
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

    // Обход без рекурсии: глубина вложенности вариантов не ограничена стеком вызовов.
    enum Item {
        Line(NodeId, bool),
        Open,
        Close,
    }
    let first_index: u32 = if root_pos.side == Color::Black { 1 } else { 0 };
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

/// Разбирает содержимое файла: проверяет размер и терпимо относится к кодировке
/// (не-UTF-8 байты в комментариях и именах заменяются, ходы всегда записаны в ASCII).
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
    fn new(text: &'a str) -> Scanner<'a> {
        Scanner {
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
                        // Одиночный служебный символ вне своего контекста.
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

/// Отделяет номер хода (`12.`, `12...`) от записи хода. Возвращает остаток слова.
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

/// Разбирает текст PDN. Из файла с несколькими партиями читается первая.
pub fn from_pdn(text: &str) -> Result<PdnGame, PdnError> {
    let mut scanner = Scanner::new(text);
    let mut tags: Vec<(String, String)> = Vec::new();

    // Заголовок.
    let mut pending = loop {
        match scanner.next()? {
            Token::Tag { name, value } => tags.push((name, value)),
            other => break other,
        }
    };

    if let Some(value) = find_tag(&tags, "GameType") {
        let number = value
            .split(',')
            .next()
            .map(str::trim)
            .and_then(|n| n.parse::<u32>().ok());
        if number != Some(GAME_TYPE_NUMBER) {
            return Err(PdnError::WrongGameType(printable(value)));
        }
    }
    let start = match find_tag(&tags, "FEN") {
        Some(fen) => Position::from_fen(fen).map_err(|_| PdnError::BadFen(printable(fen)))?,
        None => Position::initial(),
    };

    let mut tree = GameTree::from_position(start);
    let mut cursor = GameTree::ROOT;
    // Для каждого открытого варианта: узел, к которому нужно вернуться, и признак того,
    // что во внешней линии уже был ход.
    let mut stack: Vec<(NodeId, bool)> = Vec::new();
    let mut line_has_move = false;
    let mut seen_movetext = false;

    loop {
        let line = scanner.line;
        match pending {
            Token::End => break,
            // Начало следующей партии: читаем только первую.
            Token::Tag { .. } => break,
            Token::Open => {
                seen_movetext = true;
                let parent = tree.get(cursor).and_then(|n| n.parent());
                match parent {
                    Some(parent) if line_has_move => {
                        stack.push((cursor, line_has_move));
                        cursor = parent;
                        line_has_move = false;
                    }
                    _ => return Err(PdnError::VariationWithoutMove { line }),
                }
            }
            Token::Close => {
                seen_movetext = true;
                match stack.pop() {
                    Some((resume, had_move)) => {
                        cursor = resume;
                        line_has_move = had_move;
                    }
                    None => return Err(PdnError::UnbalancedVariation { line }),
                }
            }
            Token::Word(word) => {
                let is_annotation =
                    word.starts_with('$') || word.chars().all(|c| matches!(c, '!' | '?' | '.'));
                if is_result_token(&word) {
                    if stack.is_empty() {
                        break;
                    }
                } else if !is_annotation {
                    let rest = strip_move_number(&word);
                    if !rest.is_empty() {
                        let bad = |problem| PdnError::BadMove {
                            line,
                            token: printable(rest),
                            problem,
                        };
                        let Some(node) = tree.get(cursor) else {
                            return Err(PdnError::NotPdn);
                        };
                        let legal = legal_moves(node.position());
                        let mv = match parse_move(rest, &legal) {
                            Ok(mv) => mv,
                            Err(NotationError::Syntax) => {
                                // Постороннее содержимое распознаётся по первому же слову.
                                if !seen_movetext && tags.is_empty() {
                                    return Err(PdnError::NotPdn);
                                }
                                return Err(bad(MoveProblem::Syntax));
                            }
                            Err(NotationError::Illegal) => return Err(bad(MoveProblem::Illegal)),
                            Err(NotationError::Ambiguous) => return Err(bad(MoveProblem::Ambiguous)),
                        };
                        if tree.len() >= MAX_NODES {
                            return Err(PdnError::TooManyMoves);
                        }
                        cursor = match tree.add_move(cursor, &mv) {
                            Ok(id) => id,
                            Err(TreeError::GameOver) => return Err(bad(MoveProblem::GameOver)),
                            Err(_) => return Err(bad(MoveProblem::Illegal)),
                        };
                        line_has_move = true;
                    }
                    seen_movetext = true;
                }
            }
        }
        pending = scanner.next()?;
    }

    if !stack.is_empty() {
        return Err(PdnError::UnbalancedVariation { line: scanner.line });
    }
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
        let text = to_pdn(&tree, &[("White".to_string(), "Иван \"Гроза\" И.".to_string())]);
        let game = from_pdn(&text).unwrap();
        assert_eq!(game.tag("white"), Some("Иван \"Гроза\" И."));
        assert_eq!(game.tag("GameType"), Some(GAME_TYPE));
        assert_eq!(game.tag("Result"), Some("*"));
    }
}
