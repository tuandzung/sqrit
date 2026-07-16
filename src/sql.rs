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
    UnsafeCompoundDdl(&'static str),
}

impl std::fmt::Display for StatementScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unterminated(kind) => write!(f, "unterminated {kind}"),
            Self::UnsafeCompoundDdl(kind) => write!(f, "cannot safely scan {kind}"),
        }
    }
}

impl std::error::Error for StatementScanError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanKind {
    Sql,
    Quoted,
    Comment,
    ExecutableComment,
}

impl ScanKind {
    fn contributes_words(self) -> bool {
        matches!(self, Self::Sql | Self::ExecutableComment)
    }
}

struct SqlAnalysis<'a> {
    text: &'a str,
    kinds: Vec<ScanKind>,
    lexemes: Vec<Lexeme<'a>>,
}

#[derive(Debug, Clone, Copy)]
enum LexemeKind<'a> {
    Word(&'a str),
    OpenParen,
    CloseParen,
    Comma,
    StatementEnd,
}

#[derive(Debug, Clone, Copy)]
struct Lexeme<'a> {
    offset: usize,
    kind: LexemeKind<'a>,
}

pub(crate) struct QueryClassification<'a> {
    pub(crate) has_executable_sql: bool,
    pub(crate) returns_rows: bool,
    pub(crate) words: Vec<&'a str>,
}

pub fn statement_at_cursor(
    sql: &str,
    cursor: (usize, usize),
    backend: DbType,
) -> Result<Option<StatementRange>, StatementScanError> {
    let analysis = classify(sql, &backend)?;
    if let Some(kind) = unsafe_compound_ddl(&analysis, &backend) {
        return Err(StatementScanError::UnsafeCompoundDdl(kind));
    }
    let ranges = candidate_ranges(sql, &analysis.kinds)
        .into_iter()
        .filter_map(|range| trim_executable(sql, &analysis.kinds, range))
        .collect::<Vec<_>>();
    if ranges.is_empty() {
        return Ok(None);
    }
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

pub fn query_returns_rows(sql: &str, backend: &DbType) -> bool {
    classify_query(sql, backend).is_ok_and(|query| query.returns_rows)
}

pub(crate) fn classify_query<'a>(
    sql: &'a str,
    backend: &DbType,
) -> Result<QueryClassification<'a>, StatementScanError> {
    let analysis = classify(sql, backend)?;
    let words = analysis
        .lexemes
        .iter()
        .filter_map(|lexeme| match lexeme.kind {
            LexemeKind::Word(word) => Some(word),
            _ => None,
        })
        .collect::<Vec<_>>();
    let returns_rows = main_statement_word(&analysis.lexemes).is_some_and(|word| {
        ["SELECT", "VALUES", "TABLE"]
            .iter()
            .any(|keyword| word.eq_ignore_ascii_case(keyword))
    });
    let has_executable_sql = trim_executable(sql, &analysis.kinds, 0..sql.len()).is_some();
    Ok(QueryClassification {
        has_executable_sql,
        returns_rows,
        words,
    })
}

fn main_statement_word<'a>(lexemes: &[Lexeme<'a>]) -> Option<&'a str> {
    let first = lexemes
        .iter()
        .position(|lexeme| matches!(lexeme.kind, LexemeKind::Word(_) | LexemeKind::StatementEnd))?;
    let LexemeKind::Word(first_word) = lexemes[first].kind else {
        return None;
    };
    if !first_word.eq_ignore_ascii_case("WITH") {
        return Some(first_word);
    }

    let mut depth = 0usize;
    let mut saw_as = false;
    let mut in_body = false;
    let mut completed_body = false;
    for lexeme in &lexemes[first + 1..] {
        match lexeme.kind {
            LexemeKind::StatementEnd => return None,
            LexemeKind::Word(word) if depth == 0 => {
                if completed_body {
                    return Some(word);
                }
                if word.eq_ignore_ascii_case("AS") {
                    saw_as = true;
                }
            }
            LexemeKind::OpenParen => {
                if depth == 0 && saw_as {
                    in_body = true;
                }
                depth += 1;
            }
            LexemeKind::CloseParen if depth > 0 => {
                depth -= 1;
                if depth == 0 && in_body {
                    completed_body = true;
                }
            }
            LexemeKind::Comma if depth == 0 && completed_body => {
                saw_as = false;
                in_body = false;
                completed_body = false;
            }
            _ => {}
        }
    }
    None
}

fn unsafe_compound_ddl(analysis: &SqlAnalysis<'_>, backend: &DbType) -> Option<&'static str> {
    let is_compound = |word: &str| {
        ["PROCEDURE", "FUNCTION", "TRIGGER", "EVENT"]
            .iter()
            .any(|kind| word.eq_ignore_ascii_case(kind))
    };

    for range in candidate_ranges(analysis.text, &analysis.kinds) {
        let words = analysis
            .lexemes
            .iter()
            .filter(|lexeme| range.contains(&lexeme.offset))
            .filter_map(|lexeme| match lexeme.kind {
                LexemeKind::Word(word) => Some(word),
                _ => None,
            })
            .collect::<Vec<_>>();
        let Some(first) = words.first() else {
            continue;
        };
        match backend {
            DbType::Sqlite => {
                if !first.eq_ignore_ascii_case("CREATE") {
                    continue;
                }
                let object = if words.get(1).is_some_and(|word| {
                    word.eq_ignore_ascii_case("TEMP") || word.eq_ignore_ascii_case("TEMPORARY")
                }) {
                    2
                } else {
                    1
                };
                if words
                    .get(object)
                    .is_some_and(|word| word.eq_ignore_ascii_case("TRIGGER"))
                {
                    return Some("SQLite trigger definition");
                }
            }
            DbType::Mysql => {
                if first.eq_ignore_ascii_case("ALTER") {
                    let event = words
                        .get(1)
                        .is_some_and(|word| word.eq_ignore_ascii_case("EVENT"))
                        || (words
                            .get(1)
                            .is_some_and(|word| word.eq_ignore_ascii_case("DEFINER"))
                            && words
                                .iter()
                                .skip(2)
                                .any(|word| word.eq_ignore_ascii_case("EVENT")));
                    let compound_body = words.windows(2).any(|pair| {
                        pair[0].eq_ignore_ascii_case("DO") && pair[1].eq_ignore_ascii_case("BEGIN")
                    });
                    if event && compound_body {
                        return Some("MySQL compound definition");
                    }
                    continue;
                }
                if !first.eq_ignore_ascii_case("CREATE") {
                    continue;
                }
                let mut object = 1;
                if words
                    .get(object)
                    .is_some_and(|word| word.eq_ignore_ascii_case("OR"))
                    && words
                        .get(object + 1)
                        .is_some_and(|word| word.eq_ignore_ascii_case("REPLACE"))
                {
                    object += 2;
                }
                let object = if words
                    .get(object)
                    .is_some_and(|word| word.eq_ignore_ascii_case("DEFINER"))
                {
                    words
                        .iter()
                        .skip(object + 1)
                        .take(6)
                        .find(|word| is_compound(word) || word.eq_ignore_ascii_case("VIEW"))
                } else {
                    words.get(object)
                };
                if object.is_some_and(|word| is_compound(word)) {
                    return Some("MySQL compound definition");
                }
            }
            DbType::Postgres => {
                if !first.eq_ignore_ascii_case("CREATE") {
                    continue;
                }
                let mut object = 1;
                if words
                    .get(object)
                    .is_some_and(|word| word.eq_ignore_ascii_case("OR"))
                    && words
                        .get(object + 1)
                        .is_some_and(|word| word.eq_ignore_ascii_case("REPLACE"))
                {
                    object += 2;
                }
                let is_routine = words.get(object).is_some_and(|word| {
                    word.eq_ignore_ascii_case("FUNCTION") || word.eq_ignore_ascii_case("PROCEDURE")
                });
                let begin_atomic = words.windows(2).any(|pair| {
                    pair[0].eq_ignore_ascii_case("BEGIN") && pair[1].eq_ignore_ascii_case("ATOMIC")
                });
                if is_routine && begin_atomic {
                    return Some("PostgreSQL compound definition");
                }
            }
        }
    }
    None
}

fn lexemes<'a>(sql: &'a str, kinds: &[ScanKind]) -> Vec<Lexeme<'a>> {
    let bytes = sql.as_bytes();
    let mut lexemes = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if kinds[i].contributes_words() && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
            let start = i;
            i += 1;
            while i < bytes.len()
                && kinds[i].contributes_words()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_')
            {
                i += 1;
            }
            lexemes.push(Lexeme {
                offset: start,
                kind: LexemeKind::Word(&sql[start..i]),
            });
            continue;
        }
        if kinds[i].contributes_words() {
            let kind = match bytes[i] {
                b'(' => Some(LexemeKind::OpenParen),
                b')' => Some(LexemeKind::CloseParen),
                b',' => Some(LexemeKind::Comma),
                b';' if kinds[i] == ScanKind::Sql => Some(LexemeKind::StatementEnd),
                _ => None,
            };
            if let Some(kind) = kind {
                lexemes.push(Lexeme { offset: i, kind });
            }
        }
        i += 1;
    }
    lexemes
}

fn classify<'a>(sql: &'a str, backend: &DbType) -> Result<SqlAnalysis<'a>, StatementScanError> {
    let bytes = sql.as_bytes();
    let mut kinds = vec![ScanKind::Sql; bytes.len()];
    let mut i = 0;
    while i < bytes.len() {
        i = match bytes[i] {
            b'[' if matches!(backend, DbType::Sqlite) => consume_bracket(bytes, &mut kinds, i)?,
            b'\'' => consume_quote(
                bytes,
                &mut kinds,
                i,
                b'\'',
                "single-quoted string",
                quote_uses_backslash(bytes, i, b'\'', backend),
            )?,
            b'"' => consume_quote(
                bytes,
                &mut kinds,
                i,
                b'"',
                "double-quoted identifier",
                quote_uses_backslash(bytes, i, b'"', backend),
            )?,
            b'`' => consume_quote(
                bytes,
                &mut kinds,
                i,
                b'`',
                "backtick identifier",
                quote_uses_backslash(bytes, i, b'`', backend),
            )?,
            b'-' if bytes.get(i + 1) == Some(&b'-')
                && (!matches!(backend, DbType::Mysql)
                    || bytes.get(i + 2).is_some_and(|byte| {
                        byte.is_ascii_whitespace() || byte.is_ascii_control()
                    })) =>
            {
                consume_line_comment(bytes, &mut kinds, i)
            }
            b'#' if matches!(backend, DbType::Mysql) => consume_line_comment(bytes, &mut kinds, i),
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let kind = if matches!(backend, DbType::Mysql) && bytes.get(i + 2) == Some(&b'!') {
                    ScanKind::ExecutableComment
                } else {
                    ScanKind::Comment
                };
                consume_block_comment(
                    bytes,
                    &mut kinds,
                    i,
                    matches!(backend, DbType::Postgres),
                    kind,
                )?
            }
            b'$' if matches!(backend, DbType::Postgres) => match dollar_delimiter(bytes, i) {
                Some(delimiter) => consume_dollar(bytes, &mut kinds, i, delimiter)?,
                None => i + 1,
            },
            _ => i + 1,
        };
    }
    let lexemes = lexemes(sql, &kinds);
    Ok(SqlAnalysis {
        text: sql,
        kinds,
        lexemes,
    })
}

fn quote_uses_backslash(bytes: &[u8], start: usize, quote: u8, backend: &DbType) -> bool {
    match backend {
        DbType::Mysql => quote != b'`',
        DbType::Postgres => {
            quote == b'\''
                && start > 0
                && matches!(bytes[start - 1], b'e' | b'E')
                && (start == 1
                    || !(bytes[start - 2].is_ascii_alphanumeric()
                        || bytes[start - 2] == b'_'
                        || bytes[start - 2] >= 0x80))
        }
        DbType::Sqlite => false,
    }
}

fn consume_bracket(
    bytes: &[u8],
    kinds: &mut [ScanKind],
    start: usize,
) -> Result<usize, StatementScanError> {
    let Some(close) = bytes[start + 1..].iter().position(|byte| *byte == b']') else {
        return Err(StatementScanError::Unterminated("bracketed identifier"));
    };
    let end = start + close + 2;
    mark(kinds, start..end, ScanKind::Quoted);
    Ok(end)
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
    backslash_escapes: bool,
) -> Result<usize, StatementScanError> {
    let mut i = start + 1;
    while i < bytes.len() {
        if backslash_escapes && bytes[i] == b'\\' && i + 1 < bytes.len() {
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
    kind: ScanKind,
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
                mark(kinds, start..i, kind);
                return Ok(i);
            }
        } else {
            i += 1;
        }
    }
    Err(StatementScanError::Unterminated("block comment"))
}

fn dollar_delimiter(bytes: &[u8], start: usize) -> Option<&[u8]> {
    if start > 0 {
        let previous = bytes[start - 1];
        if previous.is_ascii_alphanumeric()
            || previous == b'_'
            || previous == b'$'
            || previous >= 0x80
        {
            return None;
        }
    }

    let mut i = start + 1;
    if bytes.get(i) == Some(&b'$') {
        return Some(&bytes[start..=i]);
    }
    let first = *bytes.get(i)?;
    if !(first.is_ascii_alphabetic() || first == b'_' || first >= 0x80) {
        return None;
    }

    i += 1;
    while let Some(byte) = bytes.get(i) {
        if *byte == b'$' {
            return Some(&bytes[start..=i]);
        }
        if !(byte.is_ascii_alphanumeric() || *byte == b'_' || *byte >= 0x80) {
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
            let col = content
                .char_indices()
                .nth(target_col)
                .map_or(content.len(), |(offset, _)| offset);
            return offset + col;
        }
        offset += line.len();
    }
    sql.len()
}
