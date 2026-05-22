use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Row, TypeInfo, ValueRef};

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
/// Skips leading whitespace, line/block comments to find the first keyword.
fn is_query_returning_rows(sql: &str) -> bool {
    let s = sql.trim_start();
    if s.is_empty() {
        return false;
    }

    // Skip over leading comments
    let mut rest = s;
    loop {
        let trimmed = rest.trim_start();
        if let Some(stripped) = trimmed.strip_prefix("--") {
            rest = stripped.find('\n').map_or("", |i| &stripped[i + 1..]);
        } else if let Some(stripped) = trimmed.strip_prefix("/*") {
            rest = stripped.find("*/").map_or("", |i| &stripped[i + 2..]);
        } else {
            break;
        }
    }

    let first_word = rest
        .trim_start()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
        .to_uppercase();

    matches!(first_word.as_str(), "SELECT" | "WITH" | "VALUES" | "TABLE")
}

pub struct PgAdapter {
    url: String,
    pool: Option<sqlx::postgres::PgPool>,
    // Backend PID of the most recent user query. cancel() reads it to target
    // pg_cancel_backend; in_transaction() reads it to probe pg_stat_activity.
    // Captured at execute() start, after acquiring a pool connection.
    // Sentinel 0 = unset; PG backend PIDs are positive u32 so collision is
    // impossible. Atomic avoids any std::sync::Mutex contact in async paths.
    last_pid: Arc<AtomicU64>,
}

impl PgAdapter {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            pool: None,
            last_pid: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[async_trait]
impl Database for PgAdapter {
    async fn connect(&mut self) -> anyhow::Result<()> {
        // min_connections >= 2 so cancel()'s side connection always has a
        // free slot — otherwise pool.acquire() inside cancel() could block
        // behind the very query we are trying to cancel.
        let pool = PgPoolOptions::new()
            .min_connections(2)
            .connect(&self.url)
            .await?;
        self.pool = Some(pool);
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(pool) = self.pool.take() {
            pool.close().await;
        }
        Ok(())
    }

    async fn execute(&self, query: &str) -> anyhow::Result<QueryResult> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let is_select = is_query_returning_rows(query);

        // Pin one pool connection for the whole query so we can capture its
        // backend PID up-front. cancel() targets that PID via a side
        // connection; in_transaction() probes pg_stat_activity for it later.
        let mut conn = pool.acquire().await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *conn)
            .await?;
        self.last_pid.store(pid as u64, Ordering::Relaxed);

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
        let pid = self.last_pid.load(Ordering::Relaxed);
        if pid == 0 {
            return Ok(());
        }
        let Some(pool) = self.pool.as_ref() else {
            return Ok(());
        };
        // Side connection from the pool — must not reuse the connection
        // already busy running the cancelled query. PgPoolOptions
        // min_connections in connect() guarantees a free slot.
        sqlx::query("SELECT pg_cancel_backend($1)")
            .bind(pid as i32)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn in_transaction(&self) -> anyhow::Result<bool> {
        let pid = self.last_pid.load(Ordering::Relaxed);
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
            last_pid: Arc::clone(&self.last_pid),
        })
    }
}
