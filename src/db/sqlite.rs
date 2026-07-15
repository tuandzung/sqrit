use std::sync::Arc;

use async_trait::async_trait;

use super::types::{
    ColumnInfo, IndexObject, Namespace, QueryResult, ResultColumn, SchemaInfo, TableObject,
    TriggerObject, Value, ViewObject,
};
use super::Database;

pub struct SqliteAdapter {
    path: String,
    conn: Option<Arc<std::sync::Mutex<rusqlite::Connection>>>,
    // Interrupt handle captured at connect-time. rusqlite's
    // `InterruptHandle` is `Send + Sync` but not `Clone`, so the Arc layer
    // is what lets `clone_box()` share the same handle (and therefore the
    // same underlying connection) across spawned tasks.
    interrupt: Option<Arc<rusqlite::InterruptHandle>>,
}

impl SqliteAdapter {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            conn: None,
            interrupt: None,
        }
    }

    async fn list_indexes_sqlite(&self) -> anyhow::Result<Vec<IndexObject>> {
        let result = self
            .execute(
                "SELECT name, tbl_name, sql FROM sqlite_master
                 WHERE type = 'index' AND name NOT LIKE 'sqlite_%'
                 ORDER BY name",
            )
            .await?;
        Ok(result
            .rows
            .iter()
            .filter_map(|row| {
                let name = match row.get("name") {
                    Some(Value::Text(name)) => name.clone(),
                    _ => return None,
                };
                let table = match row.get("tbl_name") {
                    Some(Value::Text(table)) => table.clone(),
                    _ => return None,
                };
                let sql = match row.get("sql") {
                    Some(Value::Text(sql)) => sql.as_str(),
                    _ => "",
                };
                Some(IndexObject {
                    name,
                    table,
                    unique: sql.to_uppercase().contains("UNIQUE"),
                })
            })
            .collect())
    }

    async fn list_triggers_sqlite(&self) -> anyhow::Result<Vec<TriggerObject>> {
        let result = self
            .execute(
                "SELECT name, tbl_name, sql FROM sqlite_master
                 WHERE type = 'trigger'
                 ORDER BY name",
            )
            .await?;
        Ok(result
            .rows
            .iter()
            .filter_map(|row| {
                let name = match row.get("name") {
                    Some(Value::Text(name)) => name.clone(),
                    _ => return None,
                };
                let table = match row.get("tbl_name") {
                    Some(Value::Text(table)) => table.clone(),
                    _ => return None,
                };
                let sql = match row.get("sql") {
                    Some(Value::Text(sql)) => sql.to_uppercase(),
                    _ => String::new(),
                };
                let event = ["INSERT", "UPDATE", "DELETE"]
                    .into_iter()
                    .find(|event| sql.split_whitespace().any(|word| word == *event))
                    .unwrap_or("")
                    .to_string();
                Some(TriggerObject { name, table, event })
            })
            .collect())
    }
}

#[async_trait]
impl Database for SqliteAdapter {
    async fn connect(&mut self) -> anyhow::Result<()> {
        let path = self.path.clone();
        let conn = tokio::task::spawn_blocking(move || rusqlite::Connection::open(&path)).await??;
        let interrupt = conn.get_interrupt_handle();
        self.conn = Some(Arc::new(std::sync::Mutex::new(conn)));
        self.interrupt = Some(Arc::new(interrupt));
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.conn = None;
        self.interrupt = None;
        Ok(())
    }

    async fn cancel(&self) -> anyhow::Result<()> {
        if let Some(handle) = self.interrupt.as_ref() {
            handle.interrupt();
        }
        Ok(())
    }

    async fn in_transaction(&self) -> anyhow::Result<bool> {
        let Some(conn) = self.conn.as_ref() else {
            return Ok(false);
        };
        let conn = Arc::clone(conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            Ok(!conn.is_autocommit())
        })
        .await?
    }

    async fn execute(&self, query: &str) -> anyhow::Result<QueryResult> {
        let conn = self
            .conn
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let conn = Arc::clone(conn);
        let query = query.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(&query)?;
            let columns: Vec<ResultColumn> = stmt
                .columns()
                .iter()
                .map(|c| ResultColumn {
                    name: c.name().to_string(),
                    data_type: c.decl_type().map(|s| s.to_string()),
                })
                .collect();
            let mut result_rows = vec![];
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let mut map = std::collections::HashMap::new();
                for (i, col) in columns.iter().enumerate() {
                    let val: Value = match row.get_ref(i)? {
                        rusqlite::types::ValueRef::Null => Value::Null,
                        rusqlite::types::ValueRef::Integer(i) => Value::Integer(i),
                        rusqlite::types::ValueRef::Real(f) => Value::Float(f),
                        rusqlite::types::ValueRef::Text(s) => {
                            Value::Text(String::from_utf8_lossy(s).to_string())
                        }
                        rusqlite::types::ValueRef::Blob(b) => Value::Blob(b.to_vec()),
                    };
                    map.insert(col.name.clone(), val);
                }
                result_rows.push(map);
            }

            let changes = conn.changes();

            Ok(QueryResult {
                columns,
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
            .filter_map(|r| {
                r.get("name").and_then(|v| match v {
                    Value::Text(s) => Some(s.clone()),
                    _ => None,
                })
            })
            .collect())
    }

    async fn list_views(&self) -> anyhow::Result<Vec<String>> {
        let result = self
            .execute("SELECT name FROM sqlite_master WHERE type='view' ORDER BY name")
            .await?;
        Ok(result
            .rows
            .iter()
            .filter_map(|r| {
                r.get("name").and_then(|v| match v {
                    Value::Text(s) => Some(s.clone()),
                    _ => None,
                })
            })
            .collect())
    }

    async fn list_columns(&self, table: &str) -> anyhow::Result<Vec<ColumnInfo>> {
        let conn = self
            .conn
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;
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
        let table_names = self.list_tables().await?;
        let view_names = self.list_views().await?;
        let mut tables = Vec::new();
        for name in &table_names {
            let columns = self.list_columns(name).await?;
            tables.push(TableObject {
                name: name.clone(),
                columns,
            });
        }
        let mut views = Vec::new();
        for name in &view_names {
            let columns = self.list_columns(name).await?;
            views.push(ViewObject {
                name: name.clone(),
                columns,
            });
        }
        Ok(SchemaInfo {
            namespaces: vec![Namespace {
                name: String::new(),
                tables,
                views,
                materialized_views: vec![],
                indexes: self.list_indexes_sqlite().await?,
                triggers: self.list_triggers_sqlite().await?,
                functions: vec![],
                procedures: vec![],
                sequences: vec![],
            }],
        })
    }

    fn clone_box(&self) -> Box<dyn Database> {
        Box::new(Self {
            path: self.path.clone(),
            conn: self.conn.clone(),
            interrupt: self.interrupt.clone(),
        })
    }
}
