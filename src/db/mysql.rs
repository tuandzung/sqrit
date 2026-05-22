use std::sync::Arc;

use async_trait::async_trait;
use sqlx::mysql::{MySqlConnection, MySqlPoolOptions};
use sqlx::{ConnectOptions, Connection, Row, TypeInfo, ValueRef};

use super::types::{sqlx_result_columns, ColumnInfo, QueryResult, SchemaInfo, TableInfo, Value};
use super::Database;

fn mysql_row_to_value(row: &sqlx::mysql::MySqlRow, i: usize) -> Value {
    row.try_get_raw(i).map_or(Value::Null, |raw| {
        if raw.is_null() {
            Value::Null
        } else {
            match raw.type_info().name() {
                // sqlx-mysql reports "BOOLEAN" for columns declared BOOLEAN / BOOL
                // (stored on disk as TINYINT(1)). Must match before "TINYINT".
                "BOOLEAN" => Value::Boolean(row.get::<bool, _>(i)),
                "TINYINT" | "SMALLINT" | "INT" | "MEDIUMINT" | "BIGINT" => {
                    Value::Integer(row.get::<i64, _>(i))
                }
                "FLOAT" | "DOUBLE" => Value::Float(row.get::<f64, _>(i)),
                "TINYINT UNSIGNED" | "SMALLINT UNSIGNED" | "INT UNSIGNED"
                | "MEDIUMINT UNSIGNED" | "BIGINT UNSIGNED" => {
                    let val = row.get::<u64, _>(i);
                    Value::Text(val.to_string())
                }
                _ => {
                    let s: Result<String, _> = row.try_get(i);
                    match s {
                        Ok(s) => Value::Text(s),
                        Err(_) => {
                            let type_name = raw.type_info().name().to_string();
                            Value::Text(format!("<unsupported mysql type: {}>", type_name))
                        }
                    }
                }
            }
        }
    })
}

fn is_query_returning_rows(sql: &str) -> bool {
    let s = sql.trim_start();
    if s.is_empty() {
        return false;
    }

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

pub struct MySqlAdapter {
    url: String,
    // Pool retained for schema introspection (list_columns) and as the
    // source of side connections for KILL QUERY.
    pool: Option<sqlx::mysql::MySqlPool>,
    // Dedicated session pinned to the execute() path so cancel can target a
    // stable CONNECTION_ID and tx_state tracks one session's history.
    exec_conn: Arc<tokio::sync::Mutex<Option<MySqlConnection>>>,
    // CONNECTION_ID() of the pinned execute session, captured at connect.
    // cancel() targets it via KILL QUERY.
    exec_conn_id: Arc<std::sync::Mutex<Option<u64>>>,
    // Best-effort transaction-open flag, flipped by execute() when the SQL
    // starts with BEGIN / START TRANSACTION (true) or COMMIT / ROLLBACK
    // (false). MySQL 8 has no session-scoped `@@in_transaction` variable
    // and information_schema.innodb_trx requires the PROCESS privilege the
    // average user lacks, so we track the flag client-side instead.
    in_tx: Arc<std::sync::Mutex<bool>>,
}

impl MySqlAdapter {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            pool: None,
            exec_conn: Arc::new(tokio::sync::Mutex::new(None)),
            exec_conn_id: Arc::new(std::sync::Mutex::new(None)),
            in_tx: Arc::new(std::sync::Mutex::new(false)),
        }
    }
}

fn tx_keyword(sql: &str) -> Option<bool> {
    let mut rest = sql.trim_start();
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
    let first = rest
        .trim_start()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
        .to_uppercase();
    match first.as_str() {
        "BEGIN" | "START" => Some(true),
        "COMMIT" | "ROLLBACK" => Some(false),
        _ => None,
    }
}

#[async_trait]
impl Database for MySqlAdapter {
    async fn connect(&mut self) -> anyhow::Result<()> {
        let pool = MySqlPoolOptions::new().connect(&self.url).await?;
        let opts: sqlx::mysql::MySqlConnectOptions = self.url.parse()?;
        let mut conn = opts.connect().await?;
        let conn_id: u64 = sqlx::query_scalar("SELECT CONNECTION_ID()")
            .fetch_one(&mut conn)
            .await?;
        *self.exec_conn.lock().await = Some(conn);
        *self.exec_conn_id.lock().unwrap() = Some(conn_id);
        self.pool = Some(pool);
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(conn) = self.exec_conn.lock().await.take() {
            let _ = conn.close().await;
        }
        *self.exec_conn_id.lock().unwrap() = None;
        if let Some(pool) = self.pool.take() {
            pool.close().await;
        }
        Ok(())
    }

    async fn execute(&self, query: &str) -> anyhow::Result<QueryResult> {
        let is_select = is_query_returning_rows(query);
        let tx_transition = tx_keyword(query);
        let mut guard = self.exec_conn.lock().await;
        let conn = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        let result: anyhow::Result<QueryResult> = if is_select {
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
                        let val = mysql_row_to_value(&row, i);
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
        } else if tx_transition.is_some() {
            // sqlx-mysql can't run BEGIN/START TRANSACTION/COMMIT/ROLLBACK
            // via the prepared-statement protocol — the server rejects them
            // with 1295 "not supported in the prepared statement protocol".
            // Route through the text protocol via `Executor::execute`.
            use sqlx::Executor;
            let stmt = conn.execute(sqlx::raw_sql(query)).await?;
            Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: Some(stmt.rows_affected()),
                total_count: None,
            })
        } else {
            let stmt = sqlx::query(query).execute(&mut *conn).await?;
            Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: Some(stmt.rows_affected()),
                total_count: None,
            })
        };

        if result.is_ok() {
            if let Some(state) = tx_transition {
                *self.in_tx.lock().unwrap() = state;
            }
        }
        result
    }

    async fn cancel(&self) -> anyhow::Result<()> {
        let Some(conn_id) = *self.exec_conn_id.lock().unwrap() else {
            return Ok(());
        };
        let Some(pool) = self.pool.as_ref() else {
            return Ok(());
        };
        // KILL QUERY <id> on a side connection from the pool. Terminates the
        // running statement only; the surrounding session and any open
        // transaction remain open — the user must COMMIT/ROLLBACK explicitly.
        sqlx::query(&format!("KILL QUERY {}", conn_id))
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn in_transaction(&self) -> anyhow::Result<bool> {
        Ok(*self.in_tx.lock().unwrap())
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
                "SELECT CAST(TABLE_NAME AS CHAR) AS TABLE_NAME \
                 FROM information_schema.TABLES \
                 WHERE TABLE_SCHEMA = DATABASE() AND TABLE_TYPE = 'BASE TABLE' \
                 ORDER BY TABLE_NAME",
            )
            .await?;
        Ok(result
            .rows
            .iter()
            .filter_map(|r| {
                r.get("TABLE_NAME").and_then(|v| match v {
                    Value::Text(s) => Some(s.clone()),
                    _ => None,
                })
            })
            .collect())
    }

    async fn list_views(&self) -> anyhow::Result<Vec<String>> {
        let result = self
            .execute(
                "SELECT CAST(TABLE_NAME AS CHAR) AS TABLE_NAME \
                 FROM information_schema.VIEWS \
                 WHERE TABLE_SCHEMA = DATABASE() \
                 ORDER BY TABLE_NAME",
            )
            .await?;
        Ok(result
            .rows
            .iter()
            .filter_map(|r| {
                r.get("TABLE_NAME").and_then(|v| match v {
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
        let rows = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT CAST(COLUMN_NAME AS CHAR) AS COLUMN_NAME, \
             CAST(COLUMN_TYPE AS CHAR) AS COLUMN_TYPE, \
             CAST(IS_NULLABLE AS CHAR) AS IS_NULLABLE, \
             CAST(COLUMN_KEY AS CHAR) AS COLUMN_KEY \
             FROM information_schema.COLUMNS \
             WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? \
             ORDER BY ORDINAL_POSITION",
        )
        .bind(table)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(name, data_type, nullable, key)| ColumnInfo {
                name,
                data_type,
                nullable: nullable == "YES",
                is_primary_key: key == "PRI",
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
            exec_conn_id: Arc::clone(&self.exec_conn_id),
            in_tx: Arc::clone(&self.in_tx),
        })
    }
}
