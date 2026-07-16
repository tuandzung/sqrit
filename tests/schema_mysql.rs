//! MySQL schema_info coverage. Requires the local compose stack.

use sqrit::db::mysql::MySqlAdapter;
use sqrit::db::Database;

const URL: &str = "mysql://sqrit:sqrit@localhost:13306/sqrit_test";
const ROOT_URL: &str = "mysql://root:root@localhost:13306/sqrit_test";
const NO_DATABASE_URL: &str = "mysql://sqrit:sqrit@localhost:13306";

async fn fresh_adapter(url: &str) -> MySqlAdapter {
    let mut adapter = MySqlAdapter::new(url);
    adapter.connect().await.unwrap();
    adapter
}

async fn seed(adapter: &MySqlAdapter) {
    for statement in [
        "DROP PROCEDURE IF EXISTS t12_pr_noop",
        "DROP FUNCTION IF EXISTS t12_fn_one",
        "DROP TRIGGER IF EXISTS t12_tr_users",
        "DROP VIEW IF EXISTS t12_users_v",
        "DROP TABLE IF EXISTS t12_users",
    ] {
        adapter.execute(statement).await.unwrap();
    }
    for statement in [
        "CREATE TABLE t12_users (id INT PRIMARY KEY AUTO_INCREMENT, email VARCHAR(255))",
        "CREATE UNIQUE INDEX t12_users_email_idx ON t12_users(email)",
        "CREATE VIEW t12_users_v AS SELECT id FROM t12_users",
        "CREATE TRIGGER t12_tr_users BEFORE UPDATE ON t12_users FOR EACH ROW SET NEW.email = NEW.email",
        "CREATE FUNCTION t12_fn_one() RETURNS INT DETERMINISTIC RETURN 1",
        "CREATE PROCEDURE t12_pr_noop() SELECT 1",
    ] {
        adapter.execute(statement).await.unwrap();
    }
}

#[tokio::test]
#[ignore]
async fn mysql_schema_info_covers_supported_object_kinds() {
    let url = std::env::var("MYSQL_URL").unwrap_or_else(|_| URL.to_string());
    // MySQL 8 requires elevated privileges for CREATE FUNCTION while binary
    // logging is enabled; the normal app user still performs introspection.
    let seed_adapter = fresh_adapter(ROOT_URL).await;
    seed(&seed_adapter).await;
    let adapter = fresh_adapter(&url).await;

    let schema = adapter.schema_info().await.unwrap();
    assert_eq!(schema.namespaces.len(), 1);
    let namespace = &schema.namespaces[0];
    assert_eq!(namespace.name, "sqrit_test");
    assert!(namespace
        .tables
        .iter()
        .any(|table| table.name == "t12_users"));
    assert!(namespace
        .views
        .iter()
        .any(|view| view.name == "t12_users_v"));
    assert!(namespace
        .indexes
        .iter()
        .any(|index| index.name == "t12_users_email_idx" && index.unique));
    assert!(namespace
        .triggers
        .iter()
        .any(|trigger| trigger.name == "t12_tr_users"));
    assert!(namespace
        .functions
        .iter()
        .any(|function| function.name == "t12_fn_one"));
    assert!(namespace
        .procedures
        .iter()
        .any(|procedure| procedure.name == "t12_pr_noop"));
    assert!(namespace.materialized_views.is_empty());
    assert!(namespace.sequences.is_empty());
}

#[tokio::test]
#[ignore]
async fn mysql_schema_info_errors_without_selected_database() {
    let adapter = fresh_adapter(NO_DATABASE_URL).await;

    let error = adapter.schema_info().await.unwrap_err();

    assert!(
        error.to_string().contains("no database selected"),
        "unexpected error: {error:#}"
    );
}
