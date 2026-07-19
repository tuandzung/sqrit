use sqrit::config::DbType;
use sqrit::sql::{statement_at_cursor, StatementScanError};

fn selected(sql: &str, cursor: (usize, usize), backend: DbType) -> (String, usize, usize) {
    let found = statement_at_cursor(sql, cursor, backend)
        .expect("scan succeeds")
        .expect("statement exists");
    (sql[found.range].to_string(), found.ordinal, found.total)
}

#[test]
fn semicolons_split_and_override_blank_lines_globally() {
    let sql = "SELECT 1\n\nSELECT 2;\n\nSELECT 3";
    assert_eq!(
        selected(sql, (0, 2), DbType::Sqlite),
        ("SELECT 1\n\nSELECT 2;".into(), 1, 2)
    );
    assert_eq!(
        selected(sql, (4, 2), DbType::Sqlite),
        ("SELECT 3".into(), 2, 2)
    );
}

#[test]
fn blank_lines_split_when_no_real_semicolon_exists() {
    let sql = "  SELECT 1  \n\n\n SELECT 2  ";
    assert_eq!(
        selected(sql, (0, 0), DbType::Sqlite),
        ("SELECT 1".into(), 1, 2)
    );
    assert_eq!(
        selected(sql, (3, 4), DbType::Sqlite),
        ("SELECT 2".into(), 2, 2)
    );
}

#[test]
fn protected_regions_do_not_create_boundaries() {
    let sql = "SELECT ';', \"a;b\", `c;d`;\n/* ;\n\n */ SELECT 2;\n-- ;\nSELECT 3;";
    assert_eq!(
        selected(sql, (0, 8), DbType::Mysql),
        ("SELECT ';', \"a;b\", `c;d`;".into(), 1, 3)
    );
    assert_eq!(
        selected(sql, (2, 8), DbType::Mysql),
        ("/* ;\n\n */ SELECT 2;".into(), 2, 3)
    );
}

#[test]
fn mysql_executable_comment_semicolons_are_not_statement_boundaries() {
    let sql = "SELECT /*!50003 1; 2 */ 3; SELECT 4;";
    assert_eq!(
        selected(sql, (0, 20), DbType::Mysql),
        ("SELECT /*!50003 1; 2 */ 3;".into(), 1, 2)
    );
}

#[test]
fn sqlite_bracketed_identifier_cannot_expose_destructive_sql() {
    let sql = "SELECT [safe; DROP TABLE users]; SELECT 2;";
    assert_eq!(
        selected(sql, (0, 18), DbType::Sqlite),
        ("SELECT [safe; DROP TABLE users];".into(), 1, 2)
    );
}

#[test]
fn unterminated_sqlite_bracketed_identifier_fails_closed() {
    let error =
        statement_at_cursor("SELECT [safe; DROP TABLE users", (0, 18), DbType::Sqlite).unwrap_err();
    assert_eq!(error.to_string(), "unterminated bracketed identifier");
}

#[test]
fn postgres_dollar_blocks_and_nested_comments_are_protected() {
    let sql = "DO $$ BEGIN PERFORM ';';\n\nEND $$;\n/* outer /* inner ; */ end */ SELECT 2;";
    assert_eq!(
        selected(sql, (1, 2), DbType::Postgres),
        ("DO $$ BEGIN PERFORM ';';\n\nEND $$;".into(), 1, 2)
    );
    assert_eq!(
        selected(sql, (3, 30), DbType::Postgres),
        ("/* outer /* inner ; */ end */ SELECT 2;".into(), 2, 2)
    );
}

#[test]
fn sqlite_multi_action_trigger_fails_closed() {
    let sql = "CREATE TRIGGER audit AFTER UPDATE ON users\nBEGIN\n  INSERT INTO log VALUES ('updated');\n  UPDATE stats SET n = n + 1;\nEND;\nSELECT 2;";
    let error = statement_at_cursor(sql, (3, 4), DbType::Sqlite).unwrap_err();
    assert_eq!(
        error.to_string(),
        "cannot safely scan SQLite trigger definition"
    );
}

#[test]
fn sqlite_explain_trigger_fails_closed_with_cursor_in_body() {
    for prefix in ["EXPLAIN", "EXPLAIN QUERY PLAN"] {
        let sql = format!(
            "{prefix} CREATE TRIGGER audit AFTER UPDATE ON users\nBEGIN\n  DELETE FROM log;\n  INSERT INTO log VALUES ('updated');\nEND;"
        );
        let error = statement_at_cursor(&sql, (2, 4), DbType::Sqlite).unwrap_err();
        assert_eq!(
            error.to_string(),
            "cannot safely scan SQLite trigger definition",
            "{prefix}"
        );
    }
}

#[test]
fn sqlite_ordinary_explain_statement_remains_selectable() {
    let sql = "EXPLAIN SELECT 1; SELECT 2;";
    assert_eq!(
        selected(sql, (0, 8), DbType::Sqlite),
        ("EXPLAIN SELECT 1;".into(), 1, 2)
    );
}

#[test]
fn mysql_compound_definitions_fail_closed() {
    for sql in [
        "CREATE PROCEDURE p() BEGIN SELECT 1; SELECT 2; END;",
        "CREATE FUNCTION f() RETURNS INT BEGIN RETURN 1; END;",
        "CREATE TRIGGER t BEFORE INSERT ON users FOR EACH ROW BEGIN SET @x = 1; SET @y = 2; END;",
        "CREATE EVENT e DO BEGIN INSERT INTO log VALUES (1); INSERT INTO log VALUES (2); END;",
    ] {
        let error = statement_at_cursor(sql, (0, 40), DbType::Mysql).unwrap_err();
        assert_eq!(
            error.to_string(),
            "cannot safely scan MySQL compound definition"
        );
    }
}

#[test]
fn mysql_executable_comment_words_participate_in_compound_safety() {
    let sql = "CREATE /*!50003 TRIGGER */ audit BEFORE DELETE ON users FOR EACH ROW BEGIN DELETE FROM log; INSERT INTO audit VALUES (OLD.id); END;";
    let error = statement_at_cursor(sql, (0, 90), DbType::Mysql).unwrap_err();
    assert_eq!(
        error.to_string(),
        "cannot safely scan MySQL compound definition"
    );
}

#[test]
fn mysql_alter_event_compound_definition_fails_closed() {
    let sql = "ALTER EVENT cleanup DO BEGIN DELETE FROM log; INSERT INTO audit VALUES (1); END;";
    let error = statement_at_cursor(sql, (0, 48), DbType::Mysql).unwrap_err();
    assert_eq!(
        error.to_string(),
        "cannot safely scan MySQL compound definition"
    );
}

#[test]
fn mysql_labeled_alter_event_compound_definition_fails_closed() {
    for sql in [
        "ALTER EVENT cleanup DO body: BEGIN DELETE FROM log; INSERT INTO audit VALUES (1); END;",
        "ALTER EVENT cleanup DO body$part: BEGIN DELETE FROM log; INSERT INTO audit VALUES (1); END;",
        "ALTER EVENT cleanup DO body標籤part: BEGIN DELETE FROM log; INSERT INTO audit VALUES (1); END;",
        "ALTER EVENT cleanup DO `body-part`: BEGIN DELETE FROM log; INSERT INTO audit VALUES (1); END;",
        "ALTER EVENT do DO body$part: BEGIN DELETE FROM log; INSERT INTO audit VALUES (1); END;",
        "ALTER EVENT cleanup DO /* body */ BEGIN DELETE FROM log; INSERT INTO audit VALUES (1); END;",
        "ALTER EVENT cleanup DO /* label */ body$part /* colon */ : /* block */ BEGIN DELETE FROM log; INSERT INTO audit VALUES (1); END;",
    ] {
        let error = statement_at_cursor(sql, (0, 55), DbType::Mysql).unwrap_err();
        assert_eq!(
            error.to_string(),
            "cannot safely scan MySQL compound definition",
            "{sql}"
        );
    }
}

#[test]
fn mysql_noncompound_alter_event_remains_selectable() {
    for sql in [
        "ALTER EVENT cleanup DO DELETE FROM log WHERE expired = 1;",
        "ALTER EVENT cleanup DO UPDATE jobs SET begin = NOW();",
        "ALTER EVENT cleanup DO INSERT INTO begin VALUES (1);",
    ] {
        assert_eq!(selected(sql, (0, 30), DbType::Mysql).0, sql);
    }
}

#[test]
fn postgres_begin_atomic_routines_fail_closed() {
    for sql in [
        "CREATE FUNCTION f() RETURNS int BEGIN ATOMIC INSERT INTO log VALUES (1); RETURN 1; END;",
        "CREATE PROCEDURE p() BEGIN ATOMIC DELETE FROM log; INSERT INTO audit VALUES (1); END;",
    ] {
        let error = statement_at_cursor(sql, (0, 65), DbType::Postgres).unwrap_err();
        assert_eq!(
            error.to_string(),
            "cannot safely scan PostgreSQL compound definition"
        );
    }
}

#[test]
fn mysql_show_create_metadata_statements_are_not_compound_definitions() {
    for object in ["TRIGGER", "PROCEDURE", "FUNCTION", "EVENT"] {
        let sql = format!("SHOW CREATE {object} object_name;");
        assert_eq!(selected(&sql, (0, 6), DbType::Mysql).0, sql, "{object}");
    }
}

#[test]
fn compound_definition_after_safe_statement_still_fails_closed() {
    for (sql, backend) in [
        (
            "SELECT 0; CREATE TRIGGER audit AFTER UPDATE ON users BEGIN INSERT INTO log VALUES (1); UPDATE stats SET n = n + 1; END;",
            DbType::Sqlite,
        ),
        (
            "SELECT 0; CREATE PROCEDURE p() BEGIN SELECT 1; SELECT 2; END;",
            DbType::Mysql,
        ),
        (
            "SELECT 0; CREATE /*!50003 TRIGGER */ audit BEFORE DELETE ON users FOR EACH ROW BEGIN DELETE FROM log; INSERT INTO audit VALUES (OLD.id); END;",
            DbType::Mysql,
        ),
        (
            "SELECT 0; CREATE FUNCTION f() RETURNS int BEGIN ATOMIC DELETE FROM log; RETURN 1; END;",
            DbType::Postgres,
        ),
    ] {
        assert!(statement_at_cursor(sql, (0, 55), backend).is_err());
    }
}

#[test]
fn mysql_hash_comments_are_backend_specific() {
    let sql = "# hidden ;\nSELECT 1\n\nSELECT 2";
    assert_eq!(
        selected(sql, (1, 2), DbType::Mysql),
        ("# hidden ;\nSELECT 1".into(), 1, 2)
    );
    assert_eq!(
        selected(sql, (0, 3), DbType::Postgres),
        ("# hidden ;".into(), 1, 2)
    );
}

#[test]
fn postgres_tagged_dollar_blocks_are_protected() {
    let sql = "DO $body$ BEGIN\nPERFORM ';';\n\nEND $body$;\nSELECT 2;";
    assert_eq!(
        selected(sql, (1, 2), DbType::Postgres),
        ("DO $body$ BEGIN\nPERFORM ';';\n\nEND $body$;".into(), 1, 2)
    );
    assert_eq!(
        selected(sql, (4, 2), DbType::Postgres),
        ("SELECT 2;".into(), 2, 2)
    );
}

#[test]
fn mysql_dash_dash_requires_following_whitespace_or_control() {
    let sql = "SELECT 1--2; SELECT 3;";
    assert_eq!(
        selected(sql, (0, 10), DbType::Mysql),
        ("SELECT 1--2;".into(), 1, 2)
    );
    assert_eq!(
        selected(sql, (0, 15), DbType::Mysql),
        ("SELECT 3;".into(), 2, 2)
    );
}

#[test]
fn sqlite_and_postgres_plain_strings_treat_terminal_backslash_literally() {
    let sql = "SELECT 'path\\'; SELECT 2;";
    for backend in [DbType::Sqlite, DbType::Postgres] {
        assert_eq!(
            selected(sql, (0, 4), backend),
            ("SELECT 'path\\';".into(), 1, 2)
        );
    }
}

#[test]
fn mysql_default_and_postgres_escape_strings_keep_escaped_quotes_protected() {
    for (sql, backend) in [
        ("SELECT 'it\\'s; safe'; SELECT 2;", DbType::Mysql),
        ("SELECT E'it\\'s; safe'; SELECT 2;", DbType::Postgres),
    ] {
        let found = selected(sql, (0, 12), backend);
        assert_eq!((found.1, found.2), (1, 2));
    }
}

#[test]
fn mysql_backtick_identifier_treats_terminal_backslash_literally() {
    let sql = "SELECT `path\\`; SELECT 2;";
    assert_eq!(
        selected(sql, (0, 4), DbType::Mysql),
        ("SELECT `path\\`;".into(), 1, 2)
    );
}

#[test]
fn doubled_quotes_keep_semicolons_protected() {
    let sql = "SELECT 'it''s; safe', \"a\"\";b\"; SELECT 2;";
    assert_eq!(
        selected(sql, (0, 18), DbType::Sqlite),
        ("SELECT 'it''s; safe', \"a\"\";b\";".into(), 1, 2)
    );

    let sql = "SELECT `a``;b`; SELECT 2;";
    assert_eq!(
        selected(sql, (0, 10), DbType::Mysql),
        ("SELECT `a``;b`;".into(), 1, 2)
    );
}

#[test]
fn blank_line_inside_quoted_region_is_not_a_boundary() {
    let sql = "SELECT 'one\n\n two'\n\nSELECT 2";
    assert_eq!(
        selected(sql, (1, 0), DbType::Sqlite),
        ("SELECT 'one\n\n two'".into(), 1, 2)
    );
}

#[test]
fn blank_line_fallback_ignores_every_multiline_protected_region() {
    for (sql, backend, expected) in [
        (
            "SELECT \"one\n\n two\"\n\nSELECT 2",
            DbType::Sqlite,
            "SELECT \"one\n\n two\"",
        ),
        (
            "SELECT `one\n\n two`\n\nSELECT 2",
            DbType::Mysql,
            "SELECT `one\n\n two`",
        ),
        (
            "SELECT [one\n\n two]\n\nSELECT 2",
            DbType::Sqlite,
            "SELECT [one\n\n two]",
        ),
        (
            "SELECT 1 /* one\n\n two */\n\nSELECT 2",
            DbType::Postgres,
            "SELECT 1 /* one\n\n two */",
        ),
        (
            "SELECT $body$one\n\n two$body$\n\nSELECT 2",
            DbType::Postgres,
            "SELECT $body$one\n\n two$body$",
        ),
    ] {
        assert_eq!(selected(sql, (1, 0), backend).0, expected, "{sql}");
    }
}

#[test]
fn postgres_dollar_opener_requires_identifier_boundary() {
    let sql = "SELECT foo$tag$; SELECT 2;";
    assert_eq!(
        selected(sql, (0, 2), DbType::Postgres),
        ("SELECT foo$tag$;".into(), 1, 2)
    );
}

#[test]
fn postgres_dollar_tags_accept_non_ascii_identifiers() {
    let sql = "DO $é$ BEGIN\nPERFORM ';';\nEND $é$;\nSELECT 2;";
    assert_eq!(
        selected(sql, (1, 2), DbType::Postgres),
        ("DO $é$ BEGIN\nPERFORM ';';\nEND $é$;".into(), 1, 2)
    );
    assert_eq!(
        selected(sql, (3, 2), DbType::Postgres),
        ("SELECT 2;".into(), 2, 2)
    );
}

#[test]
fn unicode_cursor_columns_select_the_following_statement() {
    let sql = "DROP TABLE é; SELECT 1;";
    assert_eq!(
        selected(sql, (0, 14), DbType::Sqlite),
        ("SELECT 1;".into(), 2, 2)
    );
}

#[test]
fn postgres_dollar_tags_accept_non_alphabetic_high_bytes() {
    let sql = "DO $💾$ BEGIN\nPERFORM 1;\nEND $💾$;\nSELECT 2;";
    assert_eq!(
        selected(sql, (1, 2), DbType::Postgres),
        ("DO $💾$ BEGIN\nPERFORM 1;\nEND $💾$;".into(), 1, 2)
    );
    assert_eq!(
        selected(sql, (3, 2), DbType::Postgres),
        ("SELECT 2;".into(), 2, 2)
    );
}

#[test]
fn postgres_dollar_opener_rejects_high_byte_identifier_prefix() {
    let sql = "SELECT 💾$tag$; SELECT 2;";
    assert_eq!(
        selected(sql, (0, 2), DbType::Postgres),
        ("SELECT 💾$tag$;".into(), 1, 2)
    );
}

#[test]
fn separator_and_surrounding_space_choose_the_documented_neighbor() {
    let sql = "  SELECT 1;   SELECT 2;   ";
    assert_eq!(selected(sql, (0, 0), DbType::Sqlite).0, "SELECT 1;");
    assert_eq!(selected(sql, (0, 10), DbType::Sqlite).0, "SELECT 1;");
    assert_eq!(selected(sql, (0, 12), DbType::Sqlite).0, "SELECT 1;");
    assert_eq!(selected(sql, (0, 17), DbType::Sqlite).0, "SELECT 2;");
    assert_eq!(selected(sql, (0, sql.len()), DbType::Sqlite).0, "SELECT 2;");
}

#[test]
fn empty_and_comment_only_buffers_have_no_statement() {
    assert_eq!(
        statement_at_cursor(" -- only\n/* comments */ ; ", (0, 0), DbType::Sqlite).unwrap(),
        None
    );
}

#[test]
fn unterminated_regions_fail_closed() {
    for (sql, backend, expected) in [
        ("SELECT 'open", DbType::Sqlite, "single-quoted string"),
        (
            "SELECT \"open",
            DbType::Postgres,
            "double-quoted identifier",
        ),
        ("SELECT `open", DbType::Mysql, "backtick identifier"),
        ("SELECT /* open", DbType::Sqlite, "block comment"),
        ("DO $tag$ open", DbType::Postgres, "dollar-quoted block"),
    ] {
        let error = statement_at_cursor(sql, (0, 0), backend).unwrap_err();
        assert_eq!(error, StatementScanError::Unterminated(expected));
    }
}
