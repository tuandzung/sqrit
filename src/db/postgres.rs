use async_trait::async_trait;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Column, Row, TypeInfo, ValueRef};

use super::types::{ColumnInfo, QueryResult, ResultColumn, SchemaInfo, TableInfo, Value};
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
}

impl PgAdapter {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            pool: None,
        }
    }
}

#[async_trait]
impl Database for PgAdapter {
    async fn connect(&mut self) -> anyhow::Result<()> {
        let pool = PgPoolOptions::new().connect(&self.url).await?;
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

        if is_select {
            let rows = sqlx::query(query).fetch_all(pool).await?;
            let columns: Vec<ResultColumn> = if rows.is_empty() {
                vec![]
            } else {
                rows[0]
                    .columns()
                    .iter()
                    .map(|c| ResultColumn {
                        name: c.name().to_string(),
                        data_type: Some(c.type_info().name().to_string()),
                    })
                    .collect()
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
            let result = sqlx::query(query).execute(pool).await?;
            Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: Some(result.rows_affected()),
                total_count: None,
            })
        }
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
        })
    }
}
