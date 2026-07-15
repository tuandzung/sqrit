//! Adapter-specific identifier quoting. Used when building qualified
//! `SELECT * FROM <ns>.<obj>` queries from explorer selections so backtick
//! / double-quote rules are respected.
//!
//! Embedded quote chars are doubled (SQL standard). Anything else is treated
//! as a normal identifier character — no length validation, no reserved-word
//! check.

pub fn quote_pg(ident: &str) -> String {
    let escaped = ident.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

pub fn quote_mysql(ident: &str) -> String {
    let escaped = ident.replace('`', "``");
    format!("`{}`", escaped)
}

pub fn quote_sqlite(ident: &str) -> String {
    // SQLite supports double-quotes for identifiers (preferred over `[name]`
    // and backticks for portability).
    let escaped = ident.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}
