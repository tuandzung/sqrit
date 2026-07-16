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
