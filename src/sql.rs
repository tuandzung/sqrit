use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Keyword,
    Type,
    String,
    Comment,
    Number,
    Identifier,
    Operator,
    Punctuation,
    Whitespace,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
}

pub fn keywords() -> &'static [&'static str] {
    KEYWORDS
}

pub fn types() -> &'static [&'static str] {
    SQL_TYPES
}

const KEYWORDS: &[&str] = &[
    "SELECT",
    "FROM",
    "WHERE",
    "INSERT",
    "INTO",
    "VALUES",
    "UPDATE",
    "SET",
    "DELETE",
    "CREATE",
    "DROP",
    "ALTER",
    "TABLE",
    "INDEX",
    "VIEW",
    "JOIN",
    "INNER",
    "LEFT",
    "RIGHT",
    "OUTER",
    "CROSS",
    "ON",
    "AND",
    "OR",
    "NOT",
    "NULL",
    "IS",
    "IN",
    "BETWEEN",
    "LIKE",
    "AS",
    "ORDER",
    "BY",
    "GROUP",
    "HAVING",
    "LIMIT",
    "OFFSET",
    "UNION",
    "ALL",
    "DISTINCT",
    "EXISTS",
    "CASE",
    "WHEN",
    "THEN",
    "ELSE",
    "END",
    "ASC",
    "DESC",
    "PRIMARY",
    "KEY",
    "FOREIGN",
    "REFERENCES",
    "DEFAULT",
    "CONSTRAINT",
    "UNIQUE",
    "CHECK",
    "IF",
    "BEGIN",
    "COMMIT",
    "ROLLBACK",
    "TRANSACTION",
    "RETURNING",
    "WITH",
    "RECURSIVE",
    "OVER",
    "PARTITION",
    "WINDOW",
    "ROWS",
    "RANGE",
    "UNBOUNDED",
    "PRECEDING",
    "FOLLOWING",
    "CURRENT",
    "ROW",
];

const SQL_TYPES: &[&str] = &[
    "INTEGER",
    "INT",
    "BIGINT",
    "SMALLINT",
    "TINYINT",
    "FLOAT",
    "DOUBLE",
    "REAL",
    "DECIMAL",
    "NUMERIC",
    "CHAR",
    "VARCHAR",
    "TEXT",
    "BLOB",
    "BOOLEAN",
    "BOOL",
    "DATE",
    "TIME",
    "DATETIME",
    "TIMESTAMP",
    "SERIAL",
    "BIGSERIAL",
    "UUID",
    "JSON",
    "JSONB",
    "BYTEA",
];

const PUNCTUATION: &[char] = &['(', ')', ',', ';', '.', '*'];

const OPERATORS: &[&str] = &[
    "=", "<>", "!=", "<", ">", "<=", ">=", "+", "-", "/", "%", "||", "::",
];

pub fn tokenize(sql: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = sql.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\n' | '\r' => {
                let mut text = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() {
                        text.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::Whitespace,
                    text,
                });
            }
            '\'' | '"' => {
                tokens.push(tokenize_string(&mut chars));
            }
            '-' if chars.clone().nth(1) == Some('-') => {
                tokens.push(tokenize_line_comment(&mut chars));
            }
            '/' if chars.clone().nth(1) == Some('*') => {
                tokens.push(tokenize_block_comment(&mut chars));
            }
            c if c.is_ascii_digit() => {
                tokens.push(tokenize_number(&mut chars));
            }
            c if is_identifier_start(c) => {
                tokens.push(tokenize_word(&mut chars));
            }
            c if PUNCTUATION.contains(&c) => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::Punctuation,
                    text: c.to_string(),
                });
            }
            _ => {
                if let Some(op) = try_operator(&mut chars) {
                    tokens.push(Token {
                        kind: TokenKind::Operator,
                        text: op,
                    });
                } else {
                    chars.next();
                    tokens.push(Token {
                        kind: TokenKind::Punctuation,
                        text: c.to_string(),
                    });
                }
            }
        }
    }

    tokens
}

fn tokenize_string(chars: &mut Peekable<Chars>) -> Token {
    let quote = chars.next().unwrap();
    let mut text = quote.to_string();

    while let Some(&c) = chars.peek() {
        text.push(c);
        chars.next();
        if c == quote {
            // Check for escaped quote (doubled)
            if chars.peek() == Some(&quote) {
                continue;
            }
            break;
        }
    }

    Token {
        kind: TokenKind::String,
        text,
    }
}

fn tokenize_line_comment(chars: &mut Peekable<Chars>) -> Token {
    let mut text = String::new();
    while let Some(&c) = chars.peek() {
        if c == '\n' {
            break;
        }
        text.push(c);
        chars.next();
    }
    Token {
        kind: TokenKind::Comment,
        text,
    }
}

fn tokenize_block_comment(chars: &mut Peekable<Chars>) -> Token {
    let mut text = String::new();
    let mut found_star = false;

    // Consume /*
    if chars.peek() == Some(&'/') {
        text.push(chars.next().unwrap());
    }
    if chars.peek() == Some(&'*') {
        text.push(chars.next().unwrap());
    }

    while let Some(&c) = chars.peek() {
        text.push(c);
        chars.next();
        if found_star && c == '/' {
            break;
        }
        found_star = c == '*';
    }

    Token {
        kind: TokenKind::Comment,
        text,
    }
}

fn tokenize_number(chars: &mut Peekable<Chars>) -> Token {
    let mut text = String::new();
    let mut has_dot = false;

    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            text.push(c);
            chars.next();
        } else if c == '.' && !has_dot {
            has_dot = true;
            text.push(c);
            chars.next();
        } else {
            break;
        }
    }

    Token {
        kind: TokenKind::Number,
        text,
    }
}

fn tokenize_word(chars: &mut Peekable<Chars>) -> Token {
    let mut text = String::new();

    while let Some(&c) = chars.peek() {
        if is_identifier_char(c) {
            text.push(c);
            chars.next();
        } else {
            break;
        }
    }

    let upper = text.to_uppercase();
    let kind = if KEYWORDS.contains(&upper.as_str()) {
        TokenKind::Keyword
    } else if SQL_TYPES.contains(&upper.as_str()) {
        TokenKind::Type
    } else {
        TokenKind::Identifier
    };

    Token { kind, text }
}

fn try_operator(chars: &mut Peekable<Chars>) -> Option<String> {
    let two: String = chars.clone().take(2).collect();
    if OPERATORS.contains(&two.as_str()) {
        chars.next();
        chars.next();
        return Some(two);
    }

    let one: String = chars.clone().take(1).collect();
    if OPERATORS.contains(&one.as_str()) {
        chars.next();
        return Some(one);
    }

    None
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

use std::ops::Range;

use crate::config::DbType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementRange {
    pub range: Range<usize>,
    pub ordinal: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementScanError {
    Unterminated(&'static str),
}

impl std::fmt::Display for StatementScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unterminated(kind) => write!(f, "unterminated {kind}"),
        }
    }
}

impl std::error::Error for StatementScanError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanKind {
    Sql,
    Quoted,
    Comment,
}

pub fn statement_at_cursor(
    sql: &str,
    cursor: (usize, usize),
    backend: DbType,
) -> Result<Option<StatementRange>, StatementScanError> {
    let kinds = classify(sql, &backend)?;
    let mut ranges = candidate_ranges(sql, &kinds)
        .into_iter()
        .filter_map(|range| trim_executable(sql, &kinds, range))
        .collect::<Vec<_>>();
    if ranges.is_empty() {
        return Ok(None);
    }
    ranges.sort_by_key(|range| range.start);

    let cursor = cursor_byte_offset(sql, cursor);
    let index = ranges
        .iter()
        .position(|range| range.start <= cursor && cursor < range.end)
        .or_else(|| ranges.iter().rposition(|range| range.end <= cursor))
        .unwrap_or(0);
    Ok(Some(StatementRange {
        range: ranges[index].clone(),
        ordinal: index + 1,
        total: ranges.len(),
    }))
}

fn classify(sql: &str, backend: &DbType) -> Result<Vec<ScanKind>, StatementScanError> {
    let bytes = sql.as_bytes();
    let mut kinds = vec![ScanKind::Sql; bytes.len()];
    let mut i = 0;
    while i < bytes.len() {
        i = match bytes[i] {
            b'\'' => consume_quote(bytes, &mut kinds, i, b'\'', "single-quoted string")?,
            b'"' => consume_quote(bytes, &mut kinds, i, b'"', "double-quoted identifier")?,
            b'`' => consume_quote(bytes, &mut kinds, i, b'`', "backtick identifier")?,
            b'-' if bytes.get(i + 1) == Some(&b'-') => consume_line_comment(bytes, &mut kinds, i),
            b'#' if matches!(backend, DbType::Mysql) => consume_line_comment(bytes, &mut kinds, i),
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                consume_block_comment(bytes, &mut kinds, i, matches!(backend, DbType::Postgres))?
            }
            b'$' if matches!(backend, DbType::Postgres) => match dollar_delimiter(bytes, i) {
                Some(delimiter) => consume_dollar(bytes, &mut kinds, i, delimiter)?,
                None => i + 1,
            },
            _ => i + 1,
        };
    }
    Ok(kinds)
}

fn mark(kinds: &mut [ScanKind], range: Range<usize>, kind: ScanKind) {
    kinds[range].fill(kind);
}

fn consume_quote(
    bytes: &[u8],
    kinds: &mut [ScanKind],
    start: usize,
    quote: u8,
    label: &'static str,
) -> Result<usize, StatementScanError> {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if bytes[i] == quote {
            if bytes.get(i + 1) == Some(&quote) {
                i += 2;
                continue;
            }
            let end = i + 1;
            mark(kinds, start..end, ScanKind::Quoted);
            return Ok(end);
        }
        i += 1;
    }
    Err(StatementScanError::Unterminated(label))
}

fn consume_line_comment(bytes: &[u8], kinds: &mut [ScanKind], start: usize) -> usize {
    let end = bytes[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|offset| start + offset)
        .unwrap_or(bytes.len());
    mark(kinds, start..end, ScanKind::Comment);
    end
}

fn consume_block_comment(
    bytes: &[u8],
    kinds: &mut [ScanKind],
    start: usize,
    nested: bool,
) -> Result<usize, StatementScanError> {
    let mut depth = 1usize;
    let mut i = start + 2;
    while i + 1 < bytes.len() {
        if nested && bytes[i..].starts_with(b"/*") {
            depth += 1;
            i += 2;
        } else if bytes[i..].starts_with(b"*/") {
            depth -= 1;
            i += 2;
            if depth == 0 {
                mark(kinds, start..i, ScanKind::Comment);
                return Ok(i);
            }
        } else {
            i += 1;
        }
    }
    Err(StatementScanError::Unterminated("block comment"))
}

fn dollar_delimiter(bytes: &[u8], start: usize) -> Option<&[u8]> {
    let mut i = start + 1;
    if bytes.get(i) == Some(&b'$') {
        return Some(&bytes[start..=i]);
    }
    let first = *bytes.get(i)?;
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    i += 1;
    while let Some(byte) = bytes.get(i) {
        if *byte == b'$' {
            return Some(&bytes[start..=i]);
        }
        if !(byte.is_ascii_alphanumeric() || *byte == b'_') {
            return None;
        }
        i += 1;
    }
    None
}

fn consume_dollar(
    bytes: &[u8],
    kinds: &mut [ScanKind],
    start: usize,
    delimiter: &[u8],
) -> Result<usize, StatementScanError> {
    let body_start = start + delimiter.len();
    let close = bytes[body_start..]
        .windows(delimiter.len())
        .position(|window| window == delimiter)
        .map(|offset| body_start + offset);
    let Some(close) = close else {
        return Err(StatementScanError::Unterminated("dollar-quoted block"));
    };
    let end = close + delimiter.len();
    mark(kinds, start..end, ScanKind::Quoted);
    Ok(end)
}

fn candidate_ranges(sql: &str, kinds: &[ScanKind]) -> Vec<Range<usize>> {
    let bytes = sql.as_bytes();
    let semicolons = bytes
        .iter()
        .enumerate()
        .filter_map(|(i, byte)| (*byte == b';' && kinds[i] == ScanKind::Sql).then_some(i))
        .collect::<Vec<_>>();
    if !semicolons.is_empty() {
        let mut start = 0;
        let mut ranges = Vec::new();
        for semicolon in semicolons {
            ranges.push(start..semicolon + 1);
            start = semicolon + 1;
        }
        ranges.push(start..bytes.len());
        return ranges;
    }

    let mut ranges = Vec::new();
    let mut statement_start = 0;
    let mut line_start = 0;
    for i in 0..=bytes.len() {
        if i != bytes.len() && bytes[i] != b'\n' {
            continue;
        }
        let newline_is_sql = i == bytes.len() || kinds[i] == ScanKind::Sql;
        let line_is_blank = newline_is_sql
            && bytes[line_start..i]
                .iter()
                .enumerate()
                .all(|(offset, byte)| {
                    kinds[line_start + offset] == ScanKind::Sql && byte.is_ascii_whitespace()
                });
        if line_is_blank {
            ranges.push(statement_start..line_start);
            statement_start = if i < bytes.len() { i + 1 } else { i };
        }
        line_start = i.saturating_add(1);
    }
    ranges.push(statement_start..bytes.len());
    ranges
}

fn trim_executable(sql: &str, kinds: &[ScanKind], range: Range<usize>) -> Option<Range<usize>> {
    let slice = &sql[range.clone()];
    let start = range.start + slice.len() - slice.trim_start().len();
    let end = range.end - (slice.len() - slice.trim_end().len());
    if start >= end {
        return None;
    }
    let executable = sql[start..end].char_indices().any(|(offset, ch)| {
        kinds[start + offset] != ScanKind::Comment && !ch.is_whitespace() && ch != ';'
    });
    executable.then_some(start..end)
}

fn cursor_byte_offset(sql: &str, (target_row, target_col): (usize, usize)) -> usize {
    let mut offset = 0;
    for (row, line) in sql.split_inclusive('\n').enumerate() {
        if row == target_row {
            let content = line.strip_suffix('\n').unwrap_or(line);
            let mut col = target_col.min(content.len());
            while col > 0 && !content.is_char_boundary(col) {
                col -= 1;
            }
            return offset + col;
        }
        offset += line.len();
    }
    sql.len()
}
