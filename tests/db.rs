use sqrit::db::quote::{quote_mysql, quote_pg, quote_sqlite};

#[test]
fn quote_pg_wraps_and_doubles_quotes() {
    assert_eq!(quote_pg("users"), "\"users\"");
    assert_eq!(quote_pg("weird\"name"), "\"weird\"\"name\"");
}

#[test]
fn quote_mysql_wraps_and_doubles_backticks() {
    assert_eq!(quote_mysql("users"), "`users`");
    assert_eq!(quote_mysql("weird`name"), "`weird``name`");
}

#[test]
fn quote_sqlite_wraps_and_doubles_quotes() {
    assert_eq!(quote_sqlite("users"), "\"users\"");
    assert_eq!(quote_sqlite("weird\"name"), "\"weird\"\"name\"");
}
