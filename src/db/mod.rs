pub mod types;
pub mod adapter;
pub mod sqlite;
pub mod postgres;
pub mod mysql;

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

    fn clone_box(&self) -> Box<dyn Database>;
}
