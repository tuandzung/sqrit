use std::sync::Arc;

use async_trait::async_trait;

use super::types::{ColumnInfo, QueryResult, SchemaInfo, TableInfo, Value};
use super::Database;

pub struct SqliteAdapter {
    path: String,
    conn: Option<Arc<std::sync::Mutex<rusqlite::Connection>>>,
}

impl SqliteAdapter {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            conn: None,
        }
    }
}

#[async_trait]
impl Database for SqliteAdapter {
    async fn connect(&mut self) -> anyhow::Result<()> {
        let path = self.path.clone();
        let conn = tokio::task::spawn_blocking(move || rusqlite::Connection::open(&path))
            .await??;
        self.conn = Some(Arc::new(std::sync::Mutex::new(conn)));
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.conn = None;
        Ok(())
    }

    async fn execute(&self, query: &str) -> anyhow::Result<QueryResult> {
        let conn = self.conn.as_ref().ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let conn = Arc::clone(conn);
        let query = query.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(&query)?;
            let column_names: Vec<String> = stmt
                .column_names()
                .iter()
                .map(|s| s.to_string())
                .collect();
            let mut result_rows = vec![];
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let mut map = std::collections::HashMap::new();
                for (i, name) in column_names.iter().enumerate() {
                    let val: Value = match row.get_ref(i)? {
                        rusqlite::types::ValueRef::Null => Value::Null,
                        rusqlite::types::ValueRef::Integer(i) => Value::Integer(i),
                        rusqlite::types::ValueRef::Real(f) => Value::Float(f),
                        rusqlite::types::ValueRef::Text(s) => {
                            Value::Text(String::from_utf8_lossy(s).to_string())
                        }
                        rusqlite::types::ValueRef::Blob(b) => Value::Blob(b.to_vec()),
                    };
                    map.insert(name.clone(), val);
                }
                result_rows.push(map);
            }

            let changes = conn.changes();

            Ok(QueryResult {
                columns: column_names,
                rows: result_rows,
                rows_affected: Some(changes),
                total_count: None,
            })
        })
        .await?
    }

    async fn execute_paginated(
        &self,
        query: &str,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<QueryResult> {
        let query = query.trim_end().trim_end_matches(';');
        let paginated = format!(
            "SELECT * FROM ({}) LIMIT {} OFFSET {}",
            query, limit, offset
        );
        self.execute(&paginated).await
    }

    async fn list_tables(&self) -> anyhow::Result<Vec<String>> {
        let result = self
            .execute(
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .await?;
        Ok(result
            .rows
            .iter()
            .filter_map(|r| r.get("name").and_then(|v| match v {
                Value::Text(s) => Some(s.clone()),
                _ => None,
            }))
            .collect())
    }

    async fn list_views(&self) -> anyhow::Result<Vec<String>> {
        let result = self
            .execute(
                "SELECT name FROM sqlite_master WHERE type='view' ORDER BY name",
            )
            .await?;
        Ok(result
            .rows
            .iter()
            .filter_map(|r| r.get("name").and_then(|v| match v {
                Value::Text(s) => Some(s.clone()),
                _ => None,
            }))
            .collect())
    }

    async fn list_columns(&self, table: &str) -> anyhow::Result<Vec<ColumnInfo>> {
        let conn = self.conn.as_ref().ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let conn = Arc::clone(conn);
        let table = table.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{}\")", table))?;
            let mut columns = Vec::new();
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let name: String = row.get("name")?;
                let data_type: String = row.get("type")?;
                let notnull: i32 = row.get("notnull")?;
                let pk: i32 = row.get("pk")?;
                columns.push(ColumnInfo {
                    name,
                    data_type,
                    nullable: notnull == 0,
                    is_primary_key: pk > 0,
                });
            }
            Ok(columns)
        })
        .await?
    }

    async fn schema_info(&self) -> anyhow::Result<SchemaInfo> {
        let tables = self.list_tables().await?;
        let views = self.list_views().await?;
        let mut table_infos = Vec::new();
        for name in &tables {
            let columns = self.list_columns(name).await?;
            table_infos.push(TableInfo {
                name: name.clone(),
                columns,
            });
        }
        let mut view_infos = Vec::new();
        for name in &views {
            let columns = self.list_columns(name).await?;
            view_infos.push(super::types::ViewInfo {
                name: name.clone(),
                columns,
            });
        }
        Ok(SchemaInfo {
            tables: table_infos,
            views: view_infos,
        })
    }

    fn clone_box(&self) -> Box<dyn Database> {
        Box::new(Self {
            path: self.path.clone(),
            conn: self.conn.clone(),
        })
    }
}
