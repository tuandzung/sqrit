pub mod adapter;
pub mod mysql;
pub mod postgres;
pub mod quote;
pub mod sqlite;
pub mod types;

use async_trait::async_trait;
use types::{ColumnInfo, QueryResult, SchemaInfo};

/// Skip leading whitespace, line (`--`), and block (`/* */`) comments and
/// return the remaining SQL with leading whitespace also trimmed. Shared by
/// adapter helpers that need to identify the first real keyword
/// (e.g. SELECT vs DDL, BEGIN/COMMIT tracking).
pub(crate) fn skip_leading_comments(sql: &str, mysql_hash_comments: bool) -> &str {
    let mut rest = sql;
    loop {
        let trimmed = rest.trim_start();
        if let Some(stripped) = trimmed.strip_prefix("--") {
            rest = stripped.find('\n').map_or("", |i| &stripped[i + 1..]);
        } else if let Some(stripped) = trimmed.strip_prefix("/*") {
            rest = stripped.find("*/").map_or("", |i| &stripped[i + 2..]);
        } else if mysql_hash_comments {
            if let Some(stripped) = trimmed.strip_prefix('#') {
                rest = stripped.find('\n').map_or("", |i| &stripped[i + 1..]);
                continue;
            }
            return trimmed;
        } else {
            return trimmed;
        }
    }
}

pub(crate) fn is_query_returning_rows(sql: &str, mysql_hash_comments: bool) -> bool {
    let first_word = skip_leading_comments(sql, mysql_hash_comments)
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("");
    ["SELECT", "WITH", "VALUES", "TABLE"]
        .iter()
        .any(|keyword| first_word.eq_ignore_ascii_case(keyword))
}

#[async_trait]
pub trait Database: Send + Sync {
    async fn connect(&mut self) -> anyhow::Result<()>;
    async fn disconnect(&mut self) -> anyhow::Result<()>;
    async fn execute(&self, query: &str) -> anyhow::Result<QueryResult>;
    async fn execute_paginated(
        &self,
        query: &str,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<QueryResult>;
    async fn list_tables(&self) -> anyhow::Result<Vec<String>>;
    async fn list_views(&self) -> anyhow::Result<Vec<String>>;
    async fn list_columns(&self, table: &str) -> anyhow::Result<Vec<ColumnInfo>>;
    async fn schema_info(&self) -> anyhow::Result<SchemaInfo>;

    /// Cancel any query currently running on this connection. No-op when
    /// nothing is in flight. Each adapter uses its native mechanism — see
    /// ADR 6 (SQLite: InterruptHandle, PG: pg_cancel_backend, MySQL: KILL
    /// QUERY).
    async fn cancel(&self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Whether the connection is currently inside an open transaction.
    /// Called after `cancel()` to decide the status-bar ROLLBACK hint.
    /// Best-effort; defaults to `false` for adapters that do not track it.
    async fn in_transaction(&self) -> anyhow::Result<bool> {
        Ok(false)
    }

    fn clone_box(&self) -> Box<dyn Database>;
}

#[cfg(test)]
mod tests {
    use super::is_query_returning_rows;

    #[test]
    fn row_returning_detection_skips_comments_and_covers_all_keywords() {
        for sql in [
            "SELECT 1",
            "WITH q AS (SELECT 1) SELECT * FROM q",
            "VALUES (1)",
            "TABLE users",
            "-- comment\n/* comment */ SELECT 1",
        ] {
            assert!(is_query_returning_rows(sql, false), "{sql}");
        }
        assert!(!is_query_returning_rows("INSERT INTO t VALUES (1)", false));
    }

    #[test]
    fn mysql_hash_comments_are_skipped_only_for_mysql() {
        let sql = "# comment\nSELECT 1";
        assert!(is_query_returning_rows(sql, true));
        assert!(!is_query_returning_rows(sql, false));
    }
}
