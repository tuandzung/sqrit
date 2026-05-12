use sqrit::sql::{TokenKind, tokenize};

fn kinds(tokens: &[sqrit::sql::Token]) -> Vec<TokenKind> {
    tokens.iter().map(|t| t.kind.clone()).collect()
}

// T11 #1: keyword tokenized
#[test]
fn keyword_tokenized() {
    let tokens = tokenize("SELECT");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Keyword);
    assert_eq!(tokens[0].text, "SELECT");
}

// T11 #2: full query produces keyword/punctuation/whitespace/identifier sequence
#[test]
fn full_query_tokenized() {
    let tokens = tokenize("SELECT * FROM users");
    assert!(tokens.len() >= 7);
    let kinds = kinds(&tokens);
    assert!(kinds.contains(&TokenKind::Keyword));     // SELECT, FROM
    assert!(kinds.contains(&TokenKind::Punctuation)); // *
    assert!(kinds.contains(&TokenKind::Whitespace));
    assert!(kinds.contains(&TokenKind::Identifier));  // users
}

// T11 #3: string literal tokenized
#[test]
fn string_literal_tokenized() {
    let tokens = tokenize("'hello'");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::String);
    assert_eq!(tokens[0].text, "'hello'");

    let tokens = tokenize("\"world\"");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::String);
    assert_eq!(tokens[0].text, "\"world\"");
}

// T11 #4: line comment tokenized
#[test]
fn line_comment_tokenized() {
    let tokens = tokenize("-- this is a comment");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Comment);
    assert_eq!(tokens[0].text, "-- this is a comment");
}

// T11 #5: block comment tokenized
#[test]
fn block_comment_tokenized() {
    let tokens = tokenize("/* multi\nline */");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Comment);
    assert_eq!(tokens[0].text, "/* multi\nline */");
}

// T11 #6: number tokenized
#[test]
fn number_tokenized() {
    let tokens = tokenize("42");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Number);
    assert_eq!(tokens[0].text, "42");

    let tokens = tokenize("3.14");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Number);
    assert_eq!(tokens[0].text, "3.14");
}

// T11 #7: SQL type tokenized
#[test]
fn sql_type_tokenized() {
    let tokens = tokenize("INTEGER");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Type);
    assert_eq!(tokens[0].text, "INTEGER");

    let tokens = tokenize("VARCHAR");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Type);
    assert_eq!(tokens[0].text, "VARCHAR");
}

// T11 #8: multi-line query with mixed types
#[test]
fn multiline_mixed_query() {
    let sql = "SELECT id, name\nFROM users\nWHERE age > 21\n-- filter adults\nLIMIT 10";
    let tokens = tokenize(sql);
    let kinds = kinds(&tokens);

    assert!(kinds.contains(&TokenKind::Keyword));     // SELECT, FROM, WHERE, LIMIT
    assert!(kinds.contains(&TokenKind::Identifier));  // id, name, users, age
    assert!(kinds.contains(&TokenKind::Operator));    // >
    assert!(kinds.contains(&TokenKind::Comment));     // -- filter adults
    assert!(kinds.contains(&TokenKind::Number));      // 21, 10
    assert!(kinds.contains(&TokenKind::Punctuation)); // ,
    assert!(kinds.contains(&TokenKind::Whitespace));

    // Verify roundtrip: concatenated token texts reproduce the input
    let reconstructed: String = tokens.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(reconstructed, sql);
}
