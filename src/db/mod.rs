pub mod adapter;
pub mod mysql;
pub mod postgres;
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
