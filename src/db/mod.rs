pub mod adapter;
pub mod mysql;
pub mod postgres;
pub mod quote;
pub mod sqlite;
pub mod types;

use async_trait::async_trait;
use types::{ColumnInfo, QueryResult, SchemaInfo};

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
    use crate::config::DbType;
    use crate::sql::query_returns_rows;

    #[test]
    fn row_returning_detection_skips_comments_and_covers_all_keywords() {
        for sql in [
            "SELECT 1",
            "WITH q AS (SELECT 1) SELECT * FROM q",
            "VALUES (1)",
            "TABLE users",
            "-- comment\n/* comment */ SELECT 1",
        ] {
            assert!(query_returns_rows(sql, &DbType::Postgres), "{sql}");
        }
        assert!(!query_returns_rows(
            "INSERT INTO t VALUES (1)",
            &DbType::Postgres
        ));
    }

    #[test]
    fn mysql_hash_comments_are_skipped_only_for_mysql() {
        let sql = "# comment\nSELECT 1";
        assert!(query_returns_rows(sql, &DbType::Mysql));
        assert!(!query_returns_rows(sql, &DbType::Postgres));
    }

    #[test]
    fn postgres_nested_block_comments_are_skipped_to_matching_depth() {
        let sql = "/* outer /* inner */ still outer */ SELECT 1";
        assert!(query_returns_rows(sql, &DbType::Postgres));
    }

    #[test]
    fn mysql_dash_comment_requires_following_whitespace() {
        assert!(query_returns_rows("-- comment\nSELECT 1", &DbType::Mysql));
        assert!(!query_returns_rows("--x\nSELECT 1", &DbType::Mysql));
    }

    #[test]
    fn mysql_executable_comments_contribute_words() {
        assert!(query_returns_rows("/*!50003 SELECT 1 */", &DbType::Mysql));
        assert!(!query_returns_rows(
            "/*!50003 SELECT 1 */",
            &DbType::Postgres
        ));
    }

    #[test]
    fn cte_row_detection_uses_the_depth_zero_main_statement() {
        for sql in [
            "WITH q AS (SELECT 1) SELECT * FROM q",
            "WITH q AS (SELECT 1) VALUES (1)",
            "WITH q AS (SELECT 1) TABLE q",
            "WITH a AS (SELECT 1), b AS (SELECT * FROM a) SELECT * FROM b",
        ] {
            assert!(query_returns_rows(sql, &DbType::Postgres), "{sql}");
        }

        for sql in [
            "WITH q AS (SELECT 1) INSERT INTO t SELECT * FROM q",
            "WITH q AS (SELECT 1) UPDATE t SET n = 2",
            "WITH q AS (SELECT 1) DELETE FROM t",
        ] {
            assert!(!query_returns_rows(sql, &DbType::Postgres), "{sql}");
        }
    }
}
