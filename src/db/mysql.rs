use async_trait::async_trait;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{Column, Row, TypeInfo, ValueRef};

use super::types::{ColumnInfo, QueryResult, SchemaInfo, TableInfo, Value};
use super::Database;

fn mysql_row_to_value(row: &sqlx::mysql::MySqlRow, i: usize) -> Value {
    row.try_get_raw(i).map_or(Value::Null, |raw| {
        if raw.is_null() {
            Value::Null
        } else {
            match raw.type_info().name() {
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
            rest = stripped.find('\n').map_or("", |i| &trimmed[i + 1..]);
        } else if let Some(stripped) = trimmed.strip_prefix("/*") {
            rest = stripped.find("*/").map_or("", |i| &trimmed[i + 2..]);
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

    matches!(
        first_word.as_str(),
        "SELECT" | "WITH" | "VALUES" | "TABLE"
    )
}

pub struct MySqlAdapter {
    url: String,
    pool: Option<sqlx::mysql::MySqlPool>,
}

impl MySqlAdapter {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            pool: None,
        }
    }
}

#[async_trait]
impl Database for MySqlAdapter {
    async fn connect(&mut self) -> anyhow::Result<()> {
        let pool = MySqlPoolOptions::new().connect(&self.url).await?;
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
            let columns: Vec<String> = if rows.is_empty() {
                vec![]
            } else {
                rows[0]
                    .columns()
                    .iter()
                    .map(|c| c.name().to_string())
                    .collect()
            };

            let result_rows: Vec<_> = rows
                .into_iter()
                .map(|row| {
                    let mut map = std::collections::HashMap::new();
                    for (i, col) in columns.iter().enumerate() {
                        let val = mysql_row_to_value(&row, i);
                        map.insert(col.clone(), val);
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
                "SELECT CAST(TABLE_NAME AS CHAR) AS TABLE_NAME \
                 FROM information_schema.TABLES \
                 WHERE TABLE_SCHEMA = DATABASE() AND TABLE_TYPE = 'BASE TABLE' \
                 ORDER BY TABLE_NAME",
            )
            .await?;
        Ok(result
            .rows
            .iter()
            .filter_map(|r| r.get("TABLE_NAME").and_then(|v| match v {
                Value::Text(s) => Some(s.clone()),
                _ => None,
            }))
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
            .filter_map(|r| r.get("TABLE_NAME").and_then(|v| match v {
                Value::Text(s) => Some(s.clone()),
                _ => None,
            }))
            .collect())
    }

    async fn list_columns(&self, table: &str) -> anyhow::Result<Vec<ColumnInfo>> {
        let pool = self.pool.as_ref().ok_or_else(|| anyhow::anyhow!("not connected"))?;
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
}
