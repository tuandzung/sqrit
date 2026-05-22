use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::postgres::{PgConnection, PgPoolOptions};
use sqlx::{ConnectOptions, Connection, Row, TypeInfo, ValueRef};

use super::types::{sqlx_result_columns, ColumnInfo, QueryResult, SchemaInfo, TableInfo, Value};
use super::Database;

fn pg_row_to_value(row: &sqlx::postgres::PgRow, i: usize) -> Value {
    row.try_get_raw(i).map_or(Value::Null, |raw| {
        if raw.is_null() {
            Value::Null
        } else {
            match raw.type_info().name() {
                "INT2" => Value::Integer(row.get::<i16, _>(i) as i64),
                "INT4" => Value::Integer(row.get::<i32, _>(i) as i64),
                "INT8" => Value::Integer(row.get::<i64, _>(i)),
                "FLOAT4" => Value::Float(row.get::<f32, _>(i) as f64),
                "FLOAT8" => Value::Float(row.get::<f64, _>(i)),
                "BOOL" => Value::Boolean(row.get::<bool, _>(i)),
                "TEXT" | "VARCHAR" | "NAME" | "BPCHAR" => Value::Text(row.get::<String, _>(i)),
                _ => {
                    let type_name = raw.type_info().name().to_string();
                    let s: Result<String, _> = row.try_get(i);
                    match s {
                        Ok(s) => Value::Text(s),
                        Err(_) => Value::Text(format!("<unsupported pg type: {}>", type_name)),
                    }
                }
            }
        }
    })
}

/// Determine if a SQL statement returns rows (SELECT, WITH/CTE, VALUES, TABLE).
fn is_query_returning_rows(sql: &str) -> bool {
    let first_word = super::skip_leading_comments(sql)
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
        .to_uppercase();
    matches!(first_word.as_str(), "SELECT" | "WITH" | "VALUES" | "TABLE")
}

pub struct PgAdapter {
    url: String,
    // Pool retained for cancel()'s side connection and list_columns().
    pool: Option<sqlx::postgres::PgPool>,
    // Dedicated session pinned to the execute() path. Capturing the backend
    // PID once at connect avoids a `SELECT pg_backend_pid()` round-trip on
    // every user query, and keeps cancel() / in_transaction() targeting the
    // same backend even after pool rotation would otherwise rebind a fresh
    // connection (and PID) per query.
    exec_conn: Arc<tokio::sync::Mutex<Option<PgConnection>>>,
    // Backend PID of `exec_conn`, captured at connect. Sentinel 0 = unset;
    // PG backend PIDs are positive so collision is impossible.
    exec_pid: Arc<AtomicU64>,
}

impl PgAdapter {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            pool: None,
            exec_conn: Arc::new(tokio::sync::Mutex::new(None)),
            exec_pid: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[async_trait]
impl Database for PgAdapter {
    async fn connect(&mut self) -> anyhow::Result<()> {
        // min_connections >= 1 so cancel()'s side connection always has a
        // pool slot ready — otherwise pool.acquire() inside cancel() could
        // block at startup before the pool warms up.
        let pool = PgPoolOptions::new()
            .min_connections(1)
            .connect(&self.url)
            .await?;
        let opts: sqlx::postgres::PgConnectOptions = self.url.parse()?;
        let mut conn = opts.connect().await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut conn)
            .await?;
        *self.exec_conn.lock().await = Some(conn);
        self.exec_pid.store(pid as u64, Ordering::Relaxed);
        self.pool = Some(pool);
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(conn) = self.exec_conn.lock().await.take() {
            let _ = conn.close().await;
        }
        self.exec_pid.store(0, Ordering::Relaxed);
        if let Some(pool) = self.pool.take() {
            pool.close().await;
        }
        Ok(())
    }

    async fn execute(&self, query: &str) -> anyhow::Result<QueryResult> {
        let is_select = is_query_returning_rows(query);
        let mut guard = self.exec_conn.lock().await;
        let conn = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        if is_select {
            let rows = sqlx::query(query).fetch_all(&mut *conn).await?;
            let columns = if rows.is_empty() {
                vec![]
            } else {
                sqlx_result_columns(rows[0].columns())
            };

            let result_rows: Vec<_> = rows
                .into_iter()
                .map(|row| {
                    let mut map = std::collections::HashMap::new();
                    for (i, col) in columns.iter().enumerate() {
                        let val = pg_row_to_value(&row, i);
                        map.insert(col.name.clone(), val);
                    }
                    map
                })
                .collect();

            let count = result_rows.len() as u64;
            Ok(QueryResult {
                columns,
                rows: result_rows,
                rows_affected: Some(count),
                total_count: None,
            })
        } else {
            let result = sqlx::query(query).execute(&mut *conn).await?;
            Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: Some(result.rows_affected()),
                total_count: None,
            })
        }
    }

    async fn cancel(&self) -> anyhow::Result<()> {
        let pid = self.exec_pid.load(Ordering::Relaxed);
        if pid == 0 {
            return Ok(());
        }
        let Some(pool) = self.pool.as_ref() else {
            return Ok(());
        };
        // Side connection from the pool — must not reuse the pinned exec
        // connection, which is busy with the cancelled query.
        sqlx::query("SELECT pg_cancel_backend($1)")
            .bind(pid as i32)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn in_transaction(&self) -> anyhow::Result<bool> {
        let pid = self.exec_pid.load(Ordering::Relaxed);
        if pid == 0 {
            return Ok(false);
        }
        let Some(pool) = self.pool.as_ref() else {
            return Ok(false);
        };
        let state: Option<String> =
            sqlx::query_scalar("SELECT state FROM pg_stat_activity WHERE pid = $1")
                .bind(pid as i32)
                .fetch_optional(pool)
                .await?;
        Ok(matches!(state.as_deref(), Some(s) if s.starts_with("idle in transaction")))
    }

    async fn execute_paginated(
        &self,
        query: &str,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<QueryResult> {
        let query = query.trim_end().trim_end_matches(';');
        let paginated = format!(
            "SELECT * FROM ({}) AS sub LIMIT {} OFFSET {}",
            query, limit, offset
        );
        self.execute(&paginated).await
    }

    async fn list_tables(&self) -> anyhow::Result<Vec<String>> {
        let result = self
            .execute(
                "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename",
            )
            .await?;
        Ok(result
            .rows
            .iter()
            .filter_map(|r| {
                r.get("tablename").and_then(|v| match v {
                    Value::Text(s) => Some(s.clone()),
                    _ => None,
                })
            })
            .collect())
    }

    async fn list_views(&self) -> anyhow::Result<Vec<String>> {
        let result = self
            .execute("SELECT viewname FROM pg_views WHERE schemaname = 'public' ORDER BY viewname")
            .await?;
        Ok(result
            .rows
            .iter()
            .filter_map(|r| {
                r.get("viewname").and_then(|v| match v {
                    Value::Text(s) => Some(s.clone()),
                    _ => None,
                })
            })
            .collect())
    }

    async fn list_columns(&self, table: &str) -> anyhow::Result<Vec<ColumnInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let rows = sqlx::query_as::<_, (String, String, bool, bool)>(
            "SELECT column_name, data_type, is_nullable = 'YES', EXISTS (
                SELECT 1 FROM information_schema.table_constraints tc
                JOIN information_schema.key_column_usage kcu
                    ON tc.constraint_name = kcu.constraint_name
                WHERE tc.table_name = $1 AND tc.constraint_type = 'PRIMARY KEY'
                    AND kcu.column_name = c.column_name
            ) as is_pk
            FROM information_schema.columns c
            WHERE table_name = $1 AND table_schema = 'public'
            ORDER BY ordinal_position",
        )
        .bind(table)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(name, data_type, nullable, is_primary_key)| ColumnInfo {
                name,
                data_type,
                nullable,
                is_primary_key,
            })
            .collect())
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
            url: self.url.clone(),
            pool: self.pool.clone(),
            exec_conn: Arc::clone(&self.exec_conn),
            exec_pid: Arc::clone(&self.exec_pid),
        })
    }
}
