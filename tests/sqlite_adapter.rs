use sqrit::db::sqlite::SqliteAdapter;
use sqrit::db::types::Value;
use sqrit::db::Database;

async fn setup() -> (SqliteAdapter, tempfile::NamedTempFile) {
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();
    let adapter = SqliteAdapter::new(&path);
    (adapter, file)
}

async fn setup_with_table() -> (SqliteAdapter, tempfile::NamedTempFile) {
    let (mut adapter, file) = setup().await;
    adapter.connect().await.unwrap();
    adapter
        .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, active BOOLEAN)")
        .await
        .unwrap();
    (adapter, file)
}

// #1 connect opens file, list_tables empty
#[tokio::test]
async fn connect_opens_file_and_list_tables_is_empty() {
    let (mut adapter, _file) = setup().await;
    adapter.connect().await.unwrap();

    let tables = adapter.list_tables().await.unwrap();
    assert!(tables.is_empty());
}

// #2 execute DDL returns rows_affected
#[tokio::test]
async fn execute_ddl_returns_rows_affected() {
    let (mut adapter, _file) = setup().await;
    adapter.connect().await.unwrap();

    let result = adapter
        .execute("CREATE TABLE test (id INTEGER PRIMARY KEY)")
        .await
        .unwrap();
    assert_eq!(result.rows_affected, Some(0));
}

// #3 execute INSERT returns rows_affected
#[tokio::test]
async fn execute_insert_returns_rows_affected() {
    let (adapter, _file) = setup_with_table().await;

    let result = adapter
        .execute("INSERT INTO users (name, active) VALUES ('alice', true)")
        .await
        .unwrap();
    assert_eq!(result.rows_affected, Some(1));
}

// #4 execute SELECT returns correct columns and rows
#[tokio::test]
async fn execute_select_returns_columns_and_rows() {
    let (adapter, _file) = setup_with_table().await;
    adapter
        .execute("INSERT INTO users (name, active) VALUES ('alice', true)")
        .await
        .unwrap();
    adapter
        .execute("INSERT INTO users (name, active) VALUES ('bob', false)")
        .await
        .unwrap();

    let result = adapter
        .execute("SELECT id, name, active FROM users ORDER BY id")
        .await
        .unwrap();

    let names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["id", "name", "active"]);
    assert_eq!(result.rows.len(), 2);
    assert_eq!(
        result.rows[0].get("name").unwrap(),
        &Value::Text("alice".into())
    );
    assert_eq!(result.rows[0].get("active").unwrap(), &Value::Integer(1));
    assert_eq!(
        result.rows[1].get("name").unwrap(),
        &Value::Text("bob".into())
    );
    assert_eq!(result.rows[1].get("active").unwrap(), &Value::Integer(0));
}

// #5 list_tables after CREATE TABLE includes the table
#[tokio::test]
async fn list_tables_includes_created_table() {
    let (adapter, _file) = setup_with_table().await;

    let tables = adapter.list_tables().await.unwrap();
    assert_eq!(tables, vec!["users".to_string()]);
}

// #6 list_columns returns column info
#[tokio::test]
async fn list_columns_returns_column_info() {
    let (adapter, _file) = setup_with_table().await;

    let columns = adapter.list_columns("users").await.unwrap();
    assert_eq!(columns.len(), 3);

    assert_eq!(columns[0].name, "id");
    assert!(columns[0].is_primary_key);
    assert_eq!(columns[0].data_type, "INTEGER");

    assert_eq!(columns[1].name, "name");
    assert!(!columns[1].nullable);
    assert_eq!(columns[1].data_type, "TEXT");

    assert_eq!(columns[2].name, "active");
    assert!(columns[2].nullable);
    assert_eq!(columns[2].data_type, "BOOLEAN");
}

// #7 execute_paginated respects offset and limit
#[tokio::test]
async fn execute_paginated_respects_offset_and_limit() {
    let (adapter, _file) = setup_with_table().await;
    for i in 0..5 {
        adapter
            .execute(&format!(
                "INSERT INTO users (name, active) VALUES ('user{}', true)",
                i
            ))
            .await
            .unwrap();
    }

    let result = adapter
        .execute_paginated("SELECT * FROM users ORDER BY id", 2, 2)
        .await
        .unwrap();
    assert_eq!(result.rows.len(), 2);

    let name_val = result.rows[0].get("name").unwrap();
    assert_eq!(name_val, &Value::Text("user2".into()));
}

// #8 list_views after CREATE VIEW
#[tokio::test]
async fn list_views_includes_created_view() {
    let (adapter, _file) = setup_with_table().await;
    adapter
        .execute("INSERT INTO users (name, active) VALUES ('alice', true)")
        .await
        .unwrap();
    adapter
        .execute("CREATE VIEW active_users AS SELECT id, name FROM users WHERE active = 1")
        .await
        .unwrap();

    let views = adapter.list_views().await.unwrap();
    assert_eq!(views, vec!["active_users".to_string()]);
}

// #9 schema_info combines tables and views with columns
#[tokio::test]
async fn schema_info_returns_tables_and_views() {
    let (adapter, _file) = setup_with_table().await;
    adapter
        .execute("CREATE VIEW all_users AS SELECT id, name FROM users")
        .await
        .unwrap();

    let info = adapter.schema_info().await.unwrap();
    assert_eq!(info.tables.len(), 1);
    assert_eq!(info.tables[0].name, "users");
    assert_eq!(info.tables[0].columns.len(), 3);
    assert_eq!(info.views.len(), 1);
    assert_eq!(info.views[0].name, "all_users");
    assert_eq!(info.views[0].columns.len(), 2);
}

// #10 disconnect — subsequent execute fails
#[tokio::test]
async fn disconnect_causes_execute_to_fail() {
    let (mut adapter, _file) = setup_with_table().await;
    adapter.disconnect().await.unwrap();

    let result = adapter.execute("SELECT 1").await;
    assert!(result.is_err());
}

// #11 invalid SQL returns error
#[tokio::test]
async fn execute_invalid_sql_returns_error() {
    let (adapter, _file) = setup_with_table().await;

    let result = adapter.execute("NOT VALID SQL").await;
    assert!(result.is_err());
}

// #12 connect to invalid path returns error
#[tokio::test]
async fn connect_to_invalid_path_returns_error() {
    let mut adapter = SqliteAdapter::new("/nonexistent/dir/impossible.db");
    let result = adapter.connect().await;
    assert!(result.is_err());
}

// Issue #45: declared SQL types surface on QueryResult.columns[i].data_type.
#[tokio::test]
async fn select_surfaces_declared_column_types() {
    let (mut adapter, _file) = setup().await;
    adapter.connect().await.unwrap();
    adapter
        .execute("CREATE TABLE events (id INTEGER PRIMARY KEY, ts TIMESTAMP, label TEXT)")
        .await
        .unwrap();

    let result = adapter
        .execute("SELECT id, ts, label FROM events")
        .await
        .unwrap();

    assert_eq!(
        result.columns[0]
            .data_type
            .as_deref()
            .map(str::to_lowercase),
        Some("integer".to_string())
    );
    assert!(
        result.columns[1]
            .data_type
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("timestamp"),
        "ts column should expose timestamp decl type, got: {:?}",
        result.columns[1].data_type
    );
    assert_eq!(
        result.columns[2]
            .data_type
            .as_deref()
            .map(str::to_lowercase),
        Some("text".to_string())
    );
}

// Issue #45: SQLite expressions legitimately have no declared type.
#[tokio::test]
async fn select_expression_has_no_declared_type() {
    let (mut adapter, _file) = setup().await;
    adapter.connect().await.unwrap();

    let result = adapter.execute("SELECT 1 + 1 AS x").await.unwrap();

    assert_eq!(result.columns[0].name, "x");
    assert_eq!(result.columns[0].data_type, None);
}
