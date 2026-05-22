use sqrit::db::mysql::MySqlAdapter;
use sqrit::db::types::Value;
use sqrit::db::Database;

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

fn db_url() -> String {
    std::env::var("MYSQL_URL")
        .unwrap_or_else(|_| "mysql://sqrit:sqrit@localhost:13306/sqrit_test".to_string())
}

fn mysql_available() -> bool {
    "localhost:13306"
        .to_socket_addrs()
        .ok()
        .and_then(|mut a| a.next())
        .is_some_and(|a| TcpStream::connect_timeout(&a, Duration::from_millis(500)).is_ok())
}

fn unique_table(test_name: &str) -> String {
    format!("test_{}_{}", test_name, std::process::id())
}

async fn setup() -> MySqlAdapter {
    let mut adapter = MySqlAdapter::new(&db_url());
    adapter.connect().await.unwrap();
    adapter
}

async fn setup_with_table(table: &str) -> MySqlAdapter {
    let adapter = setup().await;
    adapter
        .execute(&format!("DROP TABLE IF EXISTS `{}`", table))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "CREATE TABLE `{}` (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(255) NOT NULL, active BOOLEAN)",
            table
        ))
        .await
        .unwrap();
    adapter
}

macro_rules! maybe_skip {
    () => {
        if !mysql_available() {
            return;
        }
    };
}

// #1 connect establishes connection, list_tables works
#[tokio::test]
#[ignore]
async fn connect_and_list_tables_works() {
    maybe_skip!();
    let adapter = setup().await;
    let tables = adapter.list_tables().await;
    assert!(tables.is_ok());
}

// #2 execute DDL returns rows_affected
#[tokio::test]
#[ignore]
async fn execute_ddl_returns_rows_affected() {
    maybe_skip!();
    let table = unique_table("ddl");
    let adapter = setup().await;
    adapter
        .execute(&format!("DROP TABLE IF EXISTS `{}`", table))
        .await
        .unwrap();
    let result = adapter
        .execute(&format!(
            "CREATE TABLE `{}` (id INT AUTO_INCREMENT PRIMARY KEY)",
            table
        ))
        .await
        .unwrap();
    assert_eq!(result.rows_affected, Some(0));
}

// #3 execute INSERT returns rows_affected
#[tokio::test]
#[ignore]
async fn execute_insert_returns_rows_affected() {
    maybe_skip!();
    let table = unique_table("insert");
    let adapter = setup_with_table(&table).await;
    let result = adapter
        .execute(&format!(
            "INSERT INTO `{}` (name, active) VALUES ('alice', true)",
            table
        ))
        .await
        .unwrap();
    assert_eq!(result.rows_affected, Some(1));
}

// #4 execute SELECT returns correct columns and rows, including NULLs
#[tokio::test]
#[ignore]
async fn execute_select_returns_columns_and_rows() {
    maybe_skip!();
    let table = unique_table("select");
    let adapter = setup_with_table(&table).await;
    adapter
        .execute(&format!(
            "INSERT INTO `{}` (name, active) VALUES ('alice', true)",
            table
        ))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "INSERT INTO `{}` (name, active) VALUES ('bob', false)",
            table
        ))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "INSERT INTO `{}` (name, active) VALUES ('carol', NULL)",
            table
        ))
        .await
        .unwrap();

    let result = adapter
        .execute(&format!(
            "SELECT id, name, active FROM `{}` ORDER BY id",
            table
        ))
        .await
        .unwrap();

    assert_eq!(result.column_names(), vec!["id", "name", "active"]);
    assert_eq!(result.rows.len(), 3);
    assert_eq!(
        result.rows[0].get("name").unwrap(),
        &Value::Text("alice".into())
    );
    assert_eq!(
        result.rows[1].get("name").unwrap(),
        &Value::Text("bob".into())
    );
    assert_eq!(
        result.rows[2].get("name").unwrap(),
        &Value::Text("carol".into())
    );
    assert_eq!(result.rows[2].get("active").unwrap(), &Value::Null);

    // Regression: sqlx-mysql reports type name "BOOLEAN" for TINYINT(1)/BOOL
    // columns. Without an explicit arm in `mysql_row_to_value`, the read fell
    // through to the String branch and produced the literal placeholder
    // "<unsupported mysql type: BOOLEAN>" instead of a Value::Boolean.
    assert_eq!(result.rows[0].get("active").unwrap(), &Value::Boolean(true));
    assert_eq!(
        result.rows[1].get("active").unwrap(),
        &Value::Boolean(false)
    );
}

// #5 list_tables after CREATE TABLE includes the table
#[tokio::test]
#[ignore]
async fn list_tables_includes_created_table() {
    maybe_skip!();
    let table = unique_table("list_tbl");
    let adapter = setup_with_table(&table).await;
    let tables = adapter.list_tables().await.unwrap();
    assert!(tables.contains(&table));
}

// #6 list_columns returns column info
#[tokio::test]
#[ignore]
async fn list_columns_returns_column_info() {
    maybe_skip!();
    let table = unique_table("cols");
    let adapter = setup_with_table(&table).await;
    let columns = adapter.list_columns(&table).await.unwrap();
    assert_eq!(columns.len(), 3);

    assert_eq!(columns[0].name, "id");
    assert!(columns[0].is_primary_key);

    assert_eq!(columns[1].name, "name");
    assert!(!columns[1].nullable);

    assert_eq!(columns[2].name, "active");
    assert!(columns[2].nullable);
}

// #7 execute_paginated respects offset and limit
#[tokio::test]
#[ignore]
async fn execute_paginated_respects_offset_and_limit() {
    maybe_skip!();
    let table = unique_table("page");
    let adapter = setup_with_table(&table).await;
    for i in 0..5 {
        adapter
            .execute(&format!(
                "INSERT INTO `{}` (name, active) VALUES ('user{}', true)",
                table, i
            ))
            .await
            .unwrap();
    }

    let result = adapter
        .execute_paginated(&format!("SELECT * FROM `{}` ORDER BY id", table), 2, 2)
        .await
        .unwrap();
    assert_eq!(result.rows.len(), 2);

    let name_val = result.rows[0].get("name").unwrap();
    assert_eq!(name_val, &Value::Text("user2".into()));
}

// #8 list_views after CREATE VIEW
#[tokio::test]
#[ignore]
async fn list_views_includes_created_view() {
    maybe_skip!();
    let table = unique_table("views");
    let view = format!("{}_active_v", table);
    let adapter = setup_with_table(&table).await;
    adapter
        .execute(&format!(
            "INSERT INTO `{}` (name, active) VALUES ('alice', true)",
            table
        ))
        .await
        .unwrap();
    adapter
        .execute(&format!("DROP VIEW IF EXISTS `{}`", view))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "CREATE VIEW `{}` AS SELECT id, name FROM `{}` WHERE active = true",
            view, table
        ))
        .await
        .unwrap();

    let views = adapter.list_views().await.unwrap();
    assert!(views.contains(&view));
}

// #9 schema_info combines tables and views with columns
#[tokio::test]
#[ignore]
async fn schema_info_returns_tables_and_views() {
    maybe_skip!();
    let table = unique_table("schema");
    let view = format!("{}_all_v", table);
    let adapter = setup_with_table(&table).await;
    adapter
        .execute(&format!(
            "CREATE VIEW `{}` AS SELECT id, name FROM `{}`",
            view, table
        ))
        .await
        .unwrap();

    let info = adapter.schema_info().await.unwrap();
    assert!(info.tables.iter().any(|t| t.name == table));
    let t = info.tables.iter().find(|t| t.name == table).unwrap();
    assert_eq!(t.columns.len(), 3);
    assert!(info.views.iter().any(|v| v.name == view));
    let v = info.views.iter().find(|v| v.name == view).unwrap();
    assert_eq!(v.columns.len(), 2);
}

// #10 disconnect — subsequent execute fails
#[tokio::test]
#[ignore]
async fn disconnect_causes_execute_to_fail() {
    maybe_skip!();
    let mut adapter = setup().await;
    adapter.disconnect().await.unwrap();
    let result = adapter.execute("SELECT 1").await;
    assert!(result.is_err());
}

// #11 invalid SQL returns error
#[tokio::test]
#[ignore]
async fn execute_invalid_sql_returns_error() {
    maybe_skip!();
    let adapter = setup().await;
    let result = adapter.execute("NOT VALID SQL").await;
    assert!(result.is_err());
}

// #12 connect to invalid host returns error
#[tokio::test]
async fn connect_to_invalid_host_returns_error() {
    let mut adapter = MySqlAdapter::new("mysql://invalid:invalid@localhost:99999/nodb");
    let result = adapter.connect().await;
    assert!(result.is_err());
}

// #13 SELECT with leading line comment is treated as row-returning
#[tokio::test]
#[ignore]
async fn execute_select_with_leading_comment_returns_rows() {
    maybe_skip!();
    let table = unique_table("comment_sel");
    let adapter = setup_with_table(&table).await;
    adapter
        .execute(&format!(
            "INSERT INTO `{}` (name, active) VALUES ('alice', true)",
            table
        ))
        .await
        .unwrap();

    let sql = format!("-- leading comment\nSELECT id, name FROM `{}`", table);
    let result = adapter.execute(&sql).await.unwrap();
    assert!(!result.rows.is_empty());
    assert_eq!(
        result.rows[0].get("name").unwrap(),
        &Value::Text("alice".into())
    );
}

// #14 SELECT with leading block comment is treated as row-returning
#[tokio::test]
#[ignore]
async fn execute_select_with_leading_block_comment_returns_rows() {
    maybe_skip!();
    let table = unique_table("block_sel");
    let adapter = setup_with_table(&table).await;
    adapter
        .execute(&format!(
            "INSERT INTO `{}` (name, active) VALUES ('bob', false)",
            table
        ))
        .await
        .unwrap();

    let sql = format!("/* block comment */ SELECT id, name FROM `{}`", table);
    let result = adapter.execute(&sql).await.unwrap();
    assert!(!result.rows.is_empty());
    assert_eq!(
        result.rows[0].get("name").unwrap(),
        &Value::Text("bob".into())
    );
}

// #15 whitespace-only and comment-only input returns error
#[tokio::test]
#[ignore]
async fn execute_whitespace_and_comment_only_returns_error() {
    maybe_skip!();
    let adapter = setup().await;

    let ws = adapter.execute("   \n\t  ").await;
    assert!(ws.is_err());

    let comment = adapter.execute("-- just a comment\n").await;
    assert!(comment.is_err());
}

// Issue #45: MySQL surfaces SQL types via sqlx MySqlColumn::type_info().name().
#[tokio::test]
#[ignore]
async fn select_surfaces_mysql_column_types() {
    maybe_skip!();
    let table = unique_table("types");
    let adapter = setup().await;
    adapter
        .execute(&format!("DROP TABLE IF EXISTS `{}`", table))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "CREATE TABLE `{}` (id INT AUTO_INCREMENT PRIMARY KEY, ts DATETIME, note VARCHAR(255))",
            table
        ))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "INSERT INTO `{}` (ts, note) VALUES (NOW(), 'hi')",
            table
        ))
        .await
        .unwrap();

    let result = adapter
        .execute(&format!("SELECT id, ts, note FROM `{}`", table))
        .await
        .unwrap();

    assert!(
        result.columns[1]
            .data_type
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("datetime"),
        "ts column should expose DATETIME type, got: {:?}",
        result.columns[1].data_type
    );
    assert!(
        result.columns[2]
            .data_type
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("varchar"),
        "note column should expose VARCHAR type, got: {:?}",
        result.columns[2].data_type
    );
}

// T7: cancel() interrupts a long-running query within ~1s via KILL QUERY.
//
// MySQL's SLEEP() returns early (with value 1) when KILL QUERY fires
// instead of raising an error, so the user-visible cancel signal is "fast
// return" rather than a typed error. The 2-second timeout asserts that
// signal directly.
#[tokio::test]
#[ignore]
async fn cancel_interrupts_long_running_query() {
    if !mysql_available() {
        eprintln!("skipping: MySQL not available");
        return;
    }
    let adapter = std::sync::Arc::new(setup().await);

    let runner = std::sync::Arc::clone(&adapter);
    let handle = tokio::spawn(async move { runner.execute("SELECT SLEEP(30)").await });

    tokio::time::sleep(Duration::from_millis(200)).await;
    let started = std::time::Instant::now();
    adapter.cancel().await.unwrap();

    let result = tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("cancel did not return the future within 2s")
        .expect("spawned task panicked");
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(2),
        "cancel must short-circuit SLEEP, took {:?}",
        elapsed
    );
    // MySQL contract documented in ADR 6: KILL QUERY on SELECT SLEEP(N)
    // does NOT error — SLEEP() returns early with value 1 and the query
    // resolves Ok. Lock this in so a future cancel-shape regression cannot
    // silently change the user-visible cancellation contract on MySQL.
    assert!(
        result.is_ok(),
        "cancelled SELECT SLEEP must resolve Ok on MySQL, got: {:?}",
        result
    );
}

// T7: cancel on a fresh, never-queried adapter is a no-op.
#[tokio::test]
#[ignore]
async fn cancel_without_query_is_noop() {
    if !mysql_available() {
        eprintln!("skipping: MySQL not available");
        return;
    }
    let adapter = setup().await;
    adapter.cancel().await.unwrap();
}

// T7: in_transaction() reports true after BEGIN, false again after ROLLBACK.
// Detected via session-scoped `@@in_transaction` on the pinned exec connection.
#[tokio::test]
#[ignore]
async fn in_transaction_tracks_begin_rollback() {
    if !mysql_available() {
        eprintln!("skipping: MySQL not available");
        return;
    }
    let adapter = setup().await;
    assert!(!adapter.in_transaction().await.unwrap());
    adapter.execute("BEGIN").await.unwrap();
    assert!(adapter.in_transaction().await.unwrap());
    adapter.execute("ROLLBACK").await.unwrap();
    assert!(!adapter.in_transaction().await.unwrap());
}
