use std::sync::Arc;
use std::time::Duration;

use sqrit::db::postgres::PgAdapter;
use sqrit::db::types::{ObjectKind, ObjectRef, Value};
use sqrit::db::Database;

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sqrit:sqrit@localhost:15432/sqrit_test".to_string())
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
        .execute(&format!("DROP TABLE IF EXISTS \"{}\" CASCADE", table))
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

fn pg_object(
    kind: ObjectKind,
    name: &str,
    relation: Option<&str>,
    identity_arguments: Option<&str>,
) -> ObjectRef {
    ObjectRef {
        namespace: "public".into(),
        kind,
        name: name.into(),
        relation: relation.map(str::to_string),
        identity_arguments: identity_arguments.map(str::to_string),
    }
}

#[tokio::test]
async fn object_definition_returns_none_for_unsupported_kind_without_connection() {
    let adapter = PgAdapter::new(&db_url());
    assert_eq!(
        adapter
            .object_definition(&pg_object(ObjectKind::Table, "users", None, None))
            .await
            .unwrap(),
        None
    );
}

// #1 connect establishes connection, list_tables works
#[tokio::test]
#[ignore]
async fn connect_and_list_tables_works() {
    let adapter = setup().await;
    let tables = adapter.list_tables().await;
    assert!(tables.is_ok());
}

// #2 execute DDL returns rows_affected
#[tokio::test]
#[ignore]
async fn execute_ddl_returns_rows_affected() {
    let table = unique_table("ddl");
    let adapter = setup().await;
    adapter
        .execute(&format!("DROP TABLE IF EXISTS \"{}\" CASCADE", table))
        .await
        .unwrap();
    let result = adapter
        .execute(&format!(
            "CREATE TABLE \"{}\" (id SERIAL PRIMARY KEY)",
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
    let table = unique_table("insert");
    let adapter = setup_with_table(&table).await;
    let result = adapter
        .execute(&format!(
            "INSERT INTO \"{}\" (name, active) VALUES ('alice', true)",
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
    let table = unique_table("select");
    let adapter = setup_with_table(&table).await;
    adapter
        .execute(&format!(
            "INSERT INTO \"{}\" (name, active) VALUES ('alice', true)",
            table
        ))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "INSERT INTO \"{}\" (name, active) VALUES ('bob', false)",
            table
        ))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "INSERT INTO \"{}\" (name, active) VALUES ('carol', NULL)",
            table
        ))
        .await
        .unwrap();

    let result = adapter
        .execute(&format!(
            "SELECT id, name, active FROM \"{}\" ORDER BY id",
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
    assert_eq!(result.rows[0].get("active").unwrap(), &Value::Boolean(true));
    assert_eq!(
        result.rows[1].get("name").unwrap(),
        &Value::Text("bob".into())
    );
    assert_eq!(
        result.rows[1].get("active").unwrap(),
        &Value::Boolean(false)
    );
    assert_eq!(
        result.rows[2].get("name").unwrap(),
        &Value::Text("carol".into())
    );
    assert_eq!(result.rows[2].get("active").unwrap(), &Value::Null);
}

// #5 list_tables after CREATE TABLE includes the table
#[tokio::test]
#[ignore]
async fn list_tables_includes_created_table() {
    let table = unique_table("list_tbl");
    let adapter = setup_with_table(&table).await;
    let tables = adapter.list_tables().await.unwrap();
    assert!(tables.contains(&table));
}

// #6 list_columns returns column info
#[tokio::test]
#[ignore]
async fn list_columns_returns_column_info() {
    let table = unique_table("cols");
    let adapter = setup_with_table(&table).await;
    let columns = adapter.list_columns(&table).await.unwrap();
    assert_eq!(columns.len(), 3);

    assert_eq!(columns[0].name, "id");
    assert_eq!(columns[0].data_type, "integer");
    assert!(columns[0].is_primary_key);
    assert!(!columns[0].nullable);

    assert_eq!(columns[1].name, "name");
    assert_eq!(columns[1].data_type, "text");
    assert!(!columns[1].nullable);

    assert_eq!(columns[2].name, "active");
    assert_eq!(columns[2].data_type, "boolean");
    assert!(columns[2].nullable);
}

// #7 execute_paginated respects offset and limit
#[tokio::test]
#[ignore]
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
        .execute_paginated(&format!("SELECT * FROM \"{}\" ORDER BY id", table), 2, 2)
        .await
        .unwrap();
    assert_eq!(result.rows.len(), 2);

    let name_val = result.rows[0].get("name").unwrap();
    assert_eq!(name_val, &Value::Text("user2".into()));
}

#[tokio::test]
#[ignore]
async fn data_modifying_cte_is_rejected_before_unbounded_execution() {
    let table = unique_table("data_cte_page");
    let adapter = setup_with_table(&table).await;
    adapter
        .execute(&format!(
            "INSERT INTO \"{}\" (name, active) VALUES ('move', true)",
            table
        ))
        .await
        .unwrap();

    let query = format!(
        "WITH moved AS (DELETE FROM \"{}\" WHERE name = 'move' RETURNING name) SELECT name FROM moved",
        table
    );
    let error = adapter.execute(&query).await.unwrap_err();
    assert!(
        error.to_string().contains("unbounded results"),
        "unexpected error: {error}"
    );
    let error = adapter.execute_paginated(&query, 0, 2).await.unwrap_err();
    assert!(
        error.to_string().contains("unbounded results"),
        "unexpected error: {error}"
    );
    let remaining = adapter
        .execute(&format!(
            "SELECT COUNT(*) AS count FROM \"{}\" WHERE name = 'move'",
            table
        ))
        .await
        .unwrap();
    assert_eq!(remaining.rows[0].get("count"), Some(&Value::Integer(1)));
}

#[tokio::test]
#[ignore]
async fn search_target_cycle_with_data_cte_is_rejected_before_execution() {
    let table = unique_table("search_data_cte");
    let adapter = setup_with_table(&table).await;
    adapter
        .execute(&format!(
            "INSERT INTO \"{}\" (name, active) VALUES ('move', true)",
            table
        ))
        .await
        .unwrap();

    let query = format!(
        "WITH RECURSIVE walk(n) AS (VALUES (1) UNION ALL SELECT n + 1 FROM walk WHERE n < 2) SEARCH DEPTH FIRST BY n SET cycle, moved AS (DELETE FROM \"{}\" WHERE name = 'move' RETURNING name) SELECT name FROM moved",
        table
    );
    let error = adapter.execute(&query).await.unwrap_err();
    assert!(
        error.to_string().contains("unbounded results"),
        "unexpected error: {error}"
    );
    let error = adapter.execute_paginated(&query, 0, 2).await.unwrap_err();
    assert!(
        error.to_string().contains("unbounded results"),
        "unexpected error: {error}"
    );
    let remaining = adapter
        .execute(&format!(
            "SELECT COUNT(*) AS count FROM \"{}\" WHERE name = 'move'",
            table
        ))
        .await
        .unwrap();
    assert_eq!(remaining.rows[0].get("count"), Some(&Value::Integer(1)));
}

// #8 list_views after CREATE VIEW
#[tokio::test]
#[ignore]
async fn list_views_includes_created_view() {
    let table = unique_table("views");
    let view = format!("{}_active_v", table);
    let adapter = setup_with_table(&table).await;
    adapter
        .execute(&format!(
            "INSERT INTO \"{}\" (name, active) VALUES ('alice', true)",
            table
        ))
        .await
        .unwrap();
    adapter
        .execute(&format!("DROP VIEW IF EXISTS \"{}\"", view))
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
#[ignore]
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
    let namespace = info
        .namespaces
        .iter()
        .find(|namespace| namespace.name == "public")
        .unwrap();
    assert!(namespace.tables.iter().any(|t| t.name == table));
    let t = namespace.tables.iter().find(|t| t.name == table).unwrap();
    assert_eq!(t.columns.len(), 3);
    assert!(namespace.views.iter().any(|v| v.name == view));
    let v = namespace.views.iter().find(|v| v.name == view).unwrap();
    assert_eq!(v.columns.len(), 2);
}

// #10 disconnect — subsequent execute fails
#[tokio::test]
#[ignore]
async fn disconnect_causes_execute_to_fail() {
    let mut adapter = setup().await;
    adapter.disconnect().await.unwrap();
    let result = adapter.execute("SELECT 1").await;
    assert!(result.is_err());
}

// #11 invalid SQL returns error
#[tokio::test]
#[ignore]
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

// Issue #45: PG surfaces SQL types via sqlx PgColumn::type_info().name().
#[tokio::test]
#[ignore]
async fn select_surfaces_pg_column_types() {
    let table = unique_table("types");
    let adapter = setup().await;
    adapter
        .execute(&format!("DROP TABLE IF EXISTS \"{}\" CASCADE", table))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "CREATE TABLE \"{}\" (id SERIAL PRIMARY KEY, ts TIMESTAMPTZ, note TEXT)",
            table
        ))
        .await
        .unwrap();
    adapter
        .execute(&format!(
            "INSERT INTO \"{}\" (ts, note) VALUES (NOW(), 'hi')",
            table
        ))
        .await
        .unwrap();

    let result = adapter
        .execute(&format!("SELECT id, ts, note FROM \"{}\"", table))
        .await
        .unwrap();

    assert!(
        result.columns[1]
            .data_type
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("timestamp"),
        "ts column should expose timestamp type, got: {:?}",
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

// T7: cancel() interrupts a long-running query within ~1s via pg_cancel_backend.
#[tokio::test]
#[ignore]
async fn cancel_interrupts_long_running_query() {
    let adapter = Arc::new(setup().await);

    let runner = Arc::clone(&adapter);
    let handle = tokio::spawn(async move { runner.execute("SELECT pg_sleep(30)").await });

    // Give the server time to register the query as active so pg_cancel_backend
    // has something to cancel.
    tokio::time::sleep(Duration::from_millis(200)).await;
    adapter.cancel().await.unwrap();

    let result = tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("cancel did not interrupt query within 2s")
        .expect("spawned task panicked");
    assert!(result.is_err(), "cancelled query should return an error");
}

// T7: cancel on a fresh, never-queried adapter is a no-op.
#[tokio::test]
#[ignore]
async fn cancel_without_query_is_noop() {
    let adapter = setup().await;
    adapter.cancel().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn object_definition_returns_postgres_native_ddl() {
    let adapter = setup().await;
    for sql in [
        "DROP TABLE IF EXISTS definition_users CASCADE",
        "DROP VIEW IF EXISTS \"odd \"\"view\"",
        "DROP FUNCTION IF EXISTS definition_trigger_fn() CASCADE",
        "DROP FUNCTION IF EXISTS definition_lookup(integer)",
        "DROP PROCEDURE IF EXISTS definition_noop()",
        "CREATE TABLE definition_users(id integer, email text)",
        "CREATE VIEW definition_users_v AS SELECT id FROM definition_users",
        "CREATE VIEW \"odd \"\"view\" AS SELECT 1 AS id",
        "CREATE MATERIALIZED VIEW definition_users_mv AS SELECT id FROM definition_users",
        "CREATE INDEX definition_users_email_idx ON definition_users(email)",
        "CREATE FUNCTION definition_lookup(integer) RETURNS integer LANGUAGE sql AS 'SELECT $1'",
        "CREATE PROCEDURE definition_noop() LANGUAGE sql AS 'SELECT 1'",
        "CREATE FUNCTION definition_trigger_fn() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RETURN NEW; END'",
        "CREATE TRIGGER definition_users_trigger BEFORE UPDATE ON definition_users FOR EACH ROW EXECUTE FUNCTION definition_trigger_fn()",
    ] {
        adapter.execute(sql).await.unwrap();
    }

    let cases = [
        pg_object(ObjectKind::View, "definition_users_v", None, None),
        pg_object(
            ObjectKind::MaterializedView,
            "definition_users_mv",
            None,
            None,
        ),
        pg_object(
            ObjectKind::Index,
            "definition_users_email_idx",
            Some("definition_users"),
            None,
        ),
        pg_object(
            ObjectKind::Trigger,
            "definition_users_trigger",
            Some("definition_users"),
            None,
        ),
        pg_object(
            ObjectKind::Function,
            "definition_lookup",
            None,
            Some("integer"),
        ),
        pg_object(ObjectKind::Procedure, "definition_noop", None, Some("")),
    ];
    for object in cases {
        let ddl = adapter
            .object_definition(&object)
            .await
            .unwrap()
            .expect("definition");
        assert!(ddl.ends_with(';'), "missing terminator: {ddl}");
        assert!(ddl.to_uppercase().contains("CREATE"), "not DDL: {ddl}");
    }
    let escaped = adapter
        .object_definition(&pg_object(ObjectKind::View, "odd \"view", None, None))
        .await
        .unwrap()
        .unwrap();
    assert!(escaped.starts_with("CREATE VIEW \"public\".\"odd \"\"view\" AS"));

    assert!(adapter
        .object_definition(&pg_object(
            ObjectKind::View,
            "definition_missing",
            None,
            None
        ))
        .await
        .is_err());
}

// T7: in_transaction() reports true after BEGIN, false again after ROLLBACK.
#[tokio::test]
#[ignore]
async fn in_transaction_tracks_begin_rollback() {
    let adapter = setup().await;
    // Run one statement so the adapter has captured a backend PID.
    adapter.execute("SELECT 1").await.unwrap();
    assert!(!adapter.in_transaction().await.unwrap());
    adapter.execute("BEGIN").await.unwrap();
    assert!(adapter.in_transaction().await.unwrap());
    adapter.execute("ROLLBACK").await.unwrap();
    assert!(!adapter.in_transaction().await.unwrap());
}
