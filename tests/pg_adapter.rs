use sqrit::db::postgres::PgAdapter;
use sqrit::db::types::Value;
use sqrit::db::Database;

fn db_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sqrit:sqrit@localhost:15432/sqrit_test".to_string()
    })
}

fn unique_table(test_name: &str) -> String {
    format!("test_{}_{}", test_name, std::process::id())
}

async fn setup() -> PgAdapter {
    let mut adapter = PgAdapter::new(&db_url());
    adapter.connect().await.unwrap();
    adapter
}

async fn setup_with_table(table: &str) -> PgAdapter {
    let adapter = setup().await;
    adapter
        .execute(&format!(
            "DROP TABLE IF EXISTS \"{}\" CASCADE",
            table
        ))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "CREATE TABLE \"{}\" (id SERIAL PRIMARY KEY, name TEXT NOT NULL, active BOOLEAN)",
            table
        ))
        .await
        .unwrap();
    adapter
}

// #1 connect establishes connection, list_tables works
#[tokio::test]
async fn connect_and_list_tables_works() {
    let adapter = setup().await;
    let tables = adapter.list_tables().await;
    assert!(tables.is_ok());
}

// #2 execute DDL returns rows_affected
#[tokio::test]
async fn execute_ddl_returns_rows_affected() {
    let table = unique_table("ddl");
    let adapter = setup().await;
    adapter
        .execute(&format!("DROP TABLE IF EXISTS \"{}\" CASCADE", table))
        .await
        .unwrap();
    let result = adapter
        .execute(&format!("CREATE TABLE \"{}\" (id SERIAL PRIMARY KEY)", table))
        .await
        .unwrap();
    assert_eq!(result.rows_affected, Some(0));
}

// #3 execute INSERT returns rows_affected
#[tokio::test]
async fn execute_insert_returns_rows_affected() {
    let table = unique_table("insert");
    let adapter = setup_with_table(&table).await;
    let result = adapter
        .execute(&format!("INSERT INTO \"{}\" (name, active) VALUES ('alice', true)", table))
        .await
        .unwrap();
    assert_eq!(result.rows_affected, Some(1));
}

// #4 execute SELECT returns correct columns and rows
#[tokio::test]
async fn execute_select_returns_columns_and_rows() {
    let table = unique_table("select");
    let adapter = setup_with_table(&table).await;
    adapter
        .execute(&format!("INSERT INTO \"{}\" (name, active) VALUES ('alice', true)", table))
        .await
        .unwrap();
    adapter
        .execute(&format!("INSERT INTO \"{}\" (name, active) VALUES ('bob', false)", table))
        .await
        .unwrap();

    let result = adapter
        .execute(&format!("SELECT id, name, active FROM \"{}\" ORDER BY id", table))
        .await
        .unwrap();

    assert_eq!(result.columns, vec!["id", "name", "active"]);
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0].get("name").unwrap(), &Value::Text("alice".into()));
    assert_eq!(result.rows[0].get("active").unwrap(), &Value::Boolean(true));
    assert_eq!(result.rows[1].get("name").unwrap(), &Value::Text("bob".into()));
    assert_eq!(result.rows[1].get("active").unwrap(), &Value::Boolean(false));
}

// #5 list_tables after CREATE TABLE includes the table
#[tokio::test]
async fn list_tables_includes_created_table() {
    let table = unique_table("list_tbl");
    let adapter = setup_with_table(&table).await;
    let tables = adapter.list_tables().await.unwrap();
    assert!(tables.contains(&table));
}

// #6 list_columns returns column info
#[tokio::test]
async fn list_columns_returns_column_info() {
    let table = unique_table("cols");
    let adapter = setup_with_table(&table).await;
    let columns = adapter.list_columns(&table).await.unwrap();
    assert_eq!(columns.len(), 3);

    assert_eq!(columns[0].name, "id");
    assert!(columns[0].is_primary_key);
    assert!(!columns[0].nullable);

    assert_eq!(columns[1].name, "name");
    assert!(!columns[1].nullable);

    assert_eq!(columns[2].name, "active");
    assert!(columns[2].nullable);
}

// #7 execute_paginated respects offset and limit
#[tokio::test]
async fn execute_paginated_respects_offset_and_limit() {
    let table = unique_table("page");
    let adapter = setup_with_table(&table).await;
    for i in 0..5 {
        adapter
            .execute(&format!(
                "INSERT INTO \"{}\" (name, active) VALUES ('user{}', true)",
                table, i
            ))
            .await
            .unwrap();
    }

    let result = adapter
        .execute_paginated(
            &format!("SELECT * FROM \"{}\" ORDER BY id", table),
            2,
            2,
        )
        .await
        .unwrap();
    assert_eq!(result.rows.len(), 2);

    let name_val = result.rows[0].get("name").unwrap();
    assert_eq!(name_val, &Value::Text("user2".into()));
}

// #8 list_views after CREATE VIEW
#[tokio::test]
async fn list_views_includes_created_view() {
    let table = unique_table("views");
    let view = format!("{}_active_v", table);
    let adapter = setup_with_table(&table).await;
    adapter
        .execute(&format!("INSERT INTO \"{}\" (name, active) VALUES ('alice', true)", table))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "DROP VIEW IF EXISTS \"{}\"",
            view
        ))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "CREATE VIEW \"{}\" AS SELECT id, name FROM \"{}\" WHERE active = true",
            view, table
        ))
        .await
        .unwrap();

    let views = adapter.list_views().await.unwrap();
    assert!(views.contains(&view));
}

// #9 schema_info combines tables and views with columns
#[tokio::test]
async fn schema_info_returns_tables_and_views() {
    let table = unique_table("schema");
    let view = format!("{}_all_v", table);
    let adapter = setup_with_table(&table).await;
    adapter
        .execute(&format!(
            "CREATE VIEW \"{}\" AS SELECT id, name FROM \"{}\"",
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
async fn disconnect_causes_execute_to_fail() {
    let mut adapter = setup().await;
    adapter.disconnect().await.unwrap();
    let result = adapter.execute("SELECT 1").await;
    assert!(result.is_err());
}

// #11 invalid SQL returns error
#[tokio::test]
async fn execute_invalid_sql_returns_error() {
    let adapter = setup().await;
    let result = adapter.execute("NOT VALID SQL").await;
    assert!(result.is_err());
}

// #12 connect to invalid host returns error
#[tokio::test]
async fn connect_to_invalid_host_returns_error() {
    let mut adapter = PgAdapter::new("postgres://invalid:invalid@localhost:99999/nodb");
    let result = adapter.connect().await;
    assert!(result.is_err());
}
