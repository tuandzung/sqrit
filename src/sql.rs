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
    "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET",
    "DELETE", "CREATE", "DROP", "ALTER", "TABLE", "INDEX", "VIEW", "JOIN",
    "INNER", "LEFT", "RIGHT", "OUTER", "CROSS", "ON", "AND", "OR", "NOT",
    "NULL", "IS", "IN", "BETWEEN", "LIKE", "AS", "ORDER", "BY", "GROUP",
    "HAVING", "LIMIT", "OFFSET", "UNION", "ALL", "DISTINCT", "EXISTS",
    "CASE", "WHEN", "THEN", "ELSE", "END", "ASC", "DESC", "PRIMARY", "KEY",
    "FOREIGN", "REFERENCES", "DEFAULT", "CONSTRAINT", "UNIQUE", "CHECK",
    "IF", "BEGIN", "COMMIT", "ROLLBACK", "TRANSACTION", "RETURNING",
    "WITH", "RECURSIVE", "OVER", "PARTITION", "WINDOW", "ROWS", "RANGE",
    "UNBOUNDED", "PRECEDING", "FOLLOWING", "CURRENT", "ROW",
];

const SQL_TYPES: &[&str] = &[
    "INTEGER", "INT", "BIGINT", "SMALLINT", "TINYINT", "FLOAT", "DOUBLE",
    "REAL", "DECIMAL", "NUMERIC", "CHAR", "VARCHAR", "TEXT", "BLOB",
    "BOOLEAN", "BOOL", "DATE", "TIME", "DATETIME", "TIMESTAMP", "SERIAL",
    "BIGSERIAL", "UUID", "JSON", "JSONB", "BYTEA",
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
                tokens.push(Token { kind: TokenKind::Whitespace, text });
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
                tokens.push(Token { kind: TokenKind::Punctuation, text: c.to_string() });
            }
            _ => {
                if let Some(op) = try_operator(&mut chars) {
                    tokens.push(Token { kind: TokenKind::Operator, text: op });
                } else {
                    chars.next();
                    tokens.push(Token { kind: TokenKind::Punctuation, text: c.to_string() });
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

    Token { kind: TokenKind::String, text }
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
    Token { kind: TokenKind::Comment, text }
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

    Token { kind: TokenKind::Comment, text }
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

    Token { kind: TokenKind::Number, text }
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
