use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::mysql::{MySqlConnection, MySqlPoolOptions};
use sqlx::{ConnectOptions, Connection, Row, TypeInfo, ValueRef};

use super::types::{
    sqlx_result_columns, ColumnInfo, IndexObject, Namespace, QueryResult, RoutineObject,
    SchemaInfo, TableObject, TriggerObject, Value, ViewObject,
};
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

pub struct MySqlAdapter {
    url: String,
    // Pool retained for schema introspection (list_columns) and as the
    // source of side connections for KILL QUERY.
    pool: Option<sqlx::mysql::MySqlPool>,
    // Dedicated session pinned to the execute() path so cancel can target a
    // stable CONNECTION_ID and tx_state tracks one session's history.
    exec_conn: Arc<tokio::sync::Mutex<Option<MySqlConnection>>>,
    // CONNECTION_ID() of the pinned execute session, captured at connect.
    // cancel() targets it via KILL QUERY. Sentinel 0 = unset; live MySQL
    // CONNECTION_ID() is always > 0.
    exec_conn_id: Arc<AtomicU64>,
    // Best-effort transaction-open flag, flipped by execute() when the SQL
    // starts with BEGIN / START TRANSACTION (true) or COMMIT / ROLLBACK
    // (false). MySQL 8 has no session-scoped `@@in_transaction` variable
    // and information_schema.innodb_trx requires the PROCESS privilege the
    // average user lacks, so we track the flag client-side instead.
    in_tx: Arc<AtomicBool>,
}

impl MySqlAdapter {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            pool: None,
            exec_conn: Arc::new(tokio::sync::Mutex::new(None)),
            exec_conn_id: Arc::new(AtomicU64::new(0)),
            in_tx: Arc::new(AtomicBool::new(false)),
        }
    }

    async fn current_database_mysql(&self) -> anyhow::Result<String> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let (database,): (Option<String>,) =
            sqlx::query_as("SELECT DATABASE()").fetch_one(pool).await?;
        database.ok_or_else(|| anyhow::anyhow!("no database selected"))
    }

    async fn list_indexes_mysql(&self, schema: &str) -> anyhow::Result<Vec<IndexObject>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let rows = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT CAST(index_name AS CHAR), CAST(table_name AS CHAR), MIN(non_unique)
             FROM information_schema.statistics
             WHERE table_schema = ?
             GROUP BY index_name, table_name
             ORDER BY index_name",
        )
        .bind(schema)
        .fetch_all(pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, table, non_unique)| IndexObject {
                name,
                table,
                unique: non_unique == 0,
            })
            .collect())
    }

    async fn list_triggers_mysql(&self, schema: &str) -> anyhow::Result<Vec<TriggerObject>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT CAST(trigger_name AS CHAR), CAST(event_object_table AS CHAR),
                    CAST(event_manipulation AS CHAR)
             FROM information_schema.triggers
             WHERE trigger_schema = ?
             ORDER BY trigger_name",
        )
        .bind(schema)
        .fetch_all(pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, table, event)| TriggerObject { name, table, event })
            .collect())
    }

    async fn list_routines_mysql(
        &self,
        schema: &str,
        routine_type: &str,
    ) -> anyhow::Result<Vec<RoutineObject>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let rows = sqlx::query_as::<_, (String, Option<String>)>(
            "SELECT CAST(routine_name AS CHAR), CAST(data_type AS CHAR)
             FROM information_schema.routines
             WHERE routine_schema = ? AND routine_type = ?
             ORDER BY routine_name",
        )
        .bind(schema)
        .bind(routine_type)
        .fetch_all(pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, return_type)| RoutineObject { name, return_type })
            .collect())
    }
}

fn tx_keyword(sql: &str) -> Option<bool> {
    let mut words = super::skip_leading_comments(sql, true)
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
        .map(str::to_uppercase);
    let first = words.next().unwrap_or_default();
    match first.as_str() {
        "BEGIN" => Some(true),
        // `START` is overloaded — `START TRANSACTION` opens a tx, but
        // `START SLAVE` / `START REPLICA` / `START GROUP_REPLICATION` are
        // replication control statements and must not flip in_tx.
        "START" => match words.next().as_deref() {
            Some("TRANSACTION") => Some(true),
            _ => None,
        },
        "COMMIT" | "ROLLBACK" => Some(false),
        _ => None,
    }
}

#[async_trait]
impl Database for MySqlAdapter {
    async fn connect(&mut self) -> anyhow::Result<()> {
        // min_connections >= 1 so cancel()'s KILL QUERY always has a free
        // side connection — the pinned exec connection lives outside the
        // pool, but the pool itself still needs a slot free for KILL.
        let pool = MySqlPoolOptions::new()
            .min_connections(1)
            .connect(&self.url)
            .await?;
        let opts: sqlx::mysql::MySqlConnectOptions = self.url.parse()?;
        let mut conn = opts.connect().await?;
        let conn_id: u64 = sqlx::query_scalar("SELECT CONNECTION_ID()")
            .fetch_one(&mut conn)
            .await?;
        *self.exec_conn.lock().await = Some(conn);
        self.exec_conn_id.store(conn_id, Ordering::Relaxed);
        self.pool = Some(pool);
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(conn) = self.exec_conn.lock().await.take() {
            let _ = conn.close().await;
        }
        self.exec_conn_id.store(0, Ordering::Relaxed);
        self.in_tx.store(false, Ordering::Relaxed);
        if let Some(pool) = self.pool.take() {
            pool.close().await;
        }
        Ok(())
    }

    async fn execute(&self, query: &str) -> anyhow::Result<QueryResult> {
        if super::skip_leading_comments(query, true).is_empty() {
            anyhow::bail!("query is empty");
        }
        let is_select = super::is_query_returning_rows(query, true);
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
        } else {
            // MySQL rejects some valid statements (including transaction and
            // stored-routine DDL) through the prepared-statement protocol.
            use sqlx::Executor;
            let stmt = conn.execute(sqlx::raw_sql(query)).await?;
            Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: Some(stmt.rows_affected()),
                total_count: None,
            })
        };

        if result.is_ok() {
            if let Some(state) = tx_transition {
                self.in_tx.store(state, Ordering::Relaxed);
            }
        }
        result
    }

    async fn cancel(&self) -> anyhow::Result<()> {
        let conn_id = self.exec_conn_id.load(Ordering::Relaxed);
        if conn_id == 0 {
            return Ok(());
        }
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
        Ok(self.in_tx.load(Ordering::Relaxed))
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
        let schema = self.current_database_mysql().await?;
        let table_names = self.list_tables().await?;
        let view_names = self.list_views().await?;
        let mut tables = Vec::with_capacity(table_names.len());
        for name in &table_names {
            let columns = self.list_columns(name).await?;
            tables.push(TableObject {
                name: name.clone(),
                columns,
            });
        }
        let mut views = Vec::with_capacity(view_names.len());
        for name in &view_names {
            let columns = self.list_columns(name).await?;
            views.push(ViewObject {
                name: name.clone(),
                columns,
            });
        }
        Ok(SchemaInfo {
            namespaces: vec![Namespace {
                name: schema.clone(),
                tables,
                views,
                materialized_views: vec![],
                indexes: self.list_indexes_mysql(&schema).await?,
                triggers: self.list_triggers_mysql(&schema).await?,
                functions: self.list_routines_mysql(&schema, "FUNCTION").await?,
                procedures: self.list_routines_mysql(&schema, "PROCEDURE").await?,
                sequences: vec![],
            }],
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

#[cfg(test)]
mod tx_keyword_tests {
    use super::tx_keyword;

    #[test]
    fn begin_after_line_comment() {
        assert_eq!(tx_keyword("-- comment\n   BEGIN"), Some(true));
    }

    #[test]
    fn start_transaction_after_block_comment() {
        assert_eq!(
            tx_keyword("/* block */ START TRANSACTION READ WRITE"),
            Some(true),
        );
    }

    #[test]
    fn commit_after_whitespace() {
        assert_eq!(tx_keyword("\n\t COMMIT"), Some(false));
    }

    #[test]
    fn rollback_after_whitespace() {
        assert_eq!(tx_keyword("  ROLLBACK"), Some(false));
    }

    #[test]
    fn select_returns_none() {
        assert_eq!(tx_keyword("SELECT 1"), None);
    }

    #[test]
    fn begin_embedded_in_string_returns_none() {
        // The leading word is SELECT, not BEGIN — even though BEGIN appears
        // inside a literal later in the query.
        assert_eq!(tx_keyword("SELECT 'BEGIN TRANSACTION' AS msg"), None);
    }

    #[test]
    fn commit_embedded_in_update_returns_none() {
        assert_eq!(
            tx_keyword("UPDATE logs SET note = 'should not COMMIT here'"),
            None,
        );
    }

    #[test]
    fn only_comments_returns_none() {
        assert_eq!(tx_keyword("-- just a comment\n/* and another */"), None);
    }

    #[test]
    fn empty_input_returns_none() {
        assert_eq!(tx_keyword(""), None);
        assert_eq!(tx_keyword("   "), None);
    }

    #[test]
    fn start_slave_is_not_a_tx_start() {
        assert_eq!(tx_keyword("START SLAVE"), None);
    }

    #[test]
    fn start_replica_is_not_a_tx_start() {
        assert_eq!(tx_keyword("start replica"), None);
    }

    #[test]
    fn start_group_replication_is_not_a_tx_start() {
        assert_eq!(tx_keyword("START GROUP_REPLICATION"), None);
    }

    #[test]
    fn bare_start_alone_is_not_a_tx_start() {
        assert_eq!(tx_keyword("START"), None);
    }
}
