//! PostgreSQL schema_info coverage. Requires the local compose stack.

use sqrit::db::postgres::PgAdapter;
use sqrit::db::Database;

const URL: &str = "postgres://sqrit:sqrit@localhost:15432/sqrit_test";

async fn fresh_adapter() -> PgAdapter {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| URL.to_string());
    let mut adapter = PgAdapter::new(&url);
    adapter.connect().await.unwrap();
    adapter
}

async fn seed(adapter: &PgAdapter) {
    let statements = [
        "DROP SCHEMA IF EXISTS sqrit_t12 CASCADE",
        "DROP SCHEMA IF EXISTS sqrit_t12_shadow CASCADE",
        "CREATE SCHEMA sqrit_t12",
        "CREATE SCHEMA sqrit_t12_shadow",
        "CREATE TABLE sqrit_t12.t12_users (id SERIAL PRIMARY KEY, email TEXT)",
        "CREATE TABLE sqrit_t12_shadow.t12_users (id INT, email TEXT)",
        "CREATE VIEW sqrit_t12.t12_users_v AS SELECT id FROM sqrit_t12.t12_users",
        "CREATE MATERIALIZED VIEW sqrit_t12.t12_users_mv AS SELECT id FROM sqrit_t12.t12_users",
        "CREATE UNIQUE INDEX t12_users_email_idx ON sqrit_t12.t12_users(email)",
        "CREATE INDEX t12_users_email_idx ON sqrit_t12_shadow.t12_users(email)",
        "CREATE OR REPLACE FUNCTION sqrit_t12.t12_fn_one() RETURNS int LANGUAGE sql AS 'SELECT 1'",
        "CREATE OR REPLACE PROCEDURE sqrit_t12.t12_pr_noop() LANGUAGE sql AS $$ SELECT 1 $$",
        "CREATE SEQUENCE sqrit_t12.t12_seq_one",
        "CREATE FUNCTION sqrit_t12.t12_tg_fn() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RETURN NEW; END $$",
        "CREATE TRIGGER t12_tr_users BEFORE UPDATE ON sqrit_t12.t12_users FOR EACH ROW EXECUTE FUNCTION sqrit_t12.t12_tg_fn()",
    ];
    for statement in statements {
        adapter.execute(statement).await.unwrap();
    }
}

#[tokio::test]
#[ignore]
async fn pg_schema_info_covers_all_object_kinds() {
    let adapter = fresh_adapter().await;
    seed(&adapter).await;

    let schema = adapter.schema_info().await.unwrap();
    let namespace = schema
        .namespaces
        .iter()
        .find(|namespace| namespace.name == "sqrit_t12")
        .expect("sqrit_t12 namespace");
    assert!(namespace
        .tables
        .iter()
        .any(|table| table.name == "t12_users"));
    assert!(namespace
        .views
        .iter()
        .any(|view| view.name == "t12_users_v"));
    assert!(namespace
        .materialized_views
        .iter()
        .any(|view| view.name == "t12_users_mv"));
    let indexes = namespace
        .indexes
        .iter()
        .filter(|index| index.name == "t12_users_email_idx")
        .collect::<Vec<_>>();
    assert_eq!(
        indexes.len(),
        1,
        "same-named index in another schema must not join"
    );
    assert!(indexes[0].unique);
    assert!(namespace
        .functions
        .iter()
        .any(|function| function.name == "t12_fn_one"));
    assert!(namespace
        .procedures
        .iter()
        .any(|procedure| procedure.name == "t12_pr_noop"));
    assert!(namespace
        .sequences
        .iter()
        .any(|sequence| sequence.name == "t12_seq_one"));
    assert!(namespace
        .triggers
        .iter()
        .any(|trigger| trigger.name == "t12_tr_users"));
}

#[tokio::test]
#[ignore]
async fn pg_system_schemas_are_filtered() {
    let adapter = fresh_adapter().await;
    let schema = adapter.schema_info().await.unwrap();
    let names = schema
        .namespaces
        .iter()
        .map(|namespace| namespace.name.as_str())
        .collect::<Vec<_>>();
    for system in ["pg_catalog", "information_schema", "pg_toast"] {
        assert!(!names.contains(&system), "should filter {system}");
    }
    assert!(names
        .iter()
        .all(|name| !name.starts_with("pg_temp_") && !name.starts_with("pg_toast_temp_")));
}
