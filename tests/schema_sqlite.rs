//! Verifies SQLite schema_info populates all supported object kinds.

use sqrit::db::sqlite::SqliteAdapter;
use sqrit::db::types::ObjectKind;
use sqrit::db::Database;
use tempfile::tempdir;

async fn seed(adapter: &SqliteAdapter) {
    adapter
        .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT)")
        .await
        .unwrap();
    adapter
        .execute("CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER)")
        .await
        .unwrap();
    adapter
        .execute("CREATE VIEW logs AS SELECT id FROM users")
        .await
        .unwrap();
    adapter
        .execute("CREATE UNIQUE INDEX idx_users_email ON users(email)")
        .await
        .unwrap();
    adapter
        .execute(
            "CREATE TRIGGER trg_users_audit AFTER UPDATE ON users
             BEGIN INSERT INTO orders (user_id) VALUES (NEW.id); END",
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn sqlite_schema_info_returns_namespace_with_all_kinds() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("t.db");
    let mut adapter = SqliteAdapter::new(path.to_str().unwrap());
    adapter.connect().await.unwrap();
    seed(&adapter).await;

    let schema = adapter.schema_info().await.unwrap();
    assert_eq!(schema.namespaces.len(), 1);
    let namespace = &schema.namespaces[0];
    assert_eq!(namespace.name, "");
    assert_eq!(namespace.tables.len(), 2);
    assert_eq!(namespace.views.len(), 1);
    assert_eq!(namespace.indexes.len(), 1);
    assert_eq!(namespace.indexes[0].table, "users");
    assert!(namespace.indexes[0].unique);
    assert_eq!(namespace.triggers.len(), 1);
    assert_eq!(namespace.triggers[0].event, "UPDATE");
    assert!(namespace.materialized_views.is_empty());
    assert!(namespace.functions.is_empty());
    assert!(namespace.procedures.is_empty());
    assert!(namespace.sequences.is_empty());
}

#[tokio::test]
async fn sqlite_index_uniqueness_comes_from_metadata() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("t.db");
    let mut adapter = SqliteAdapter::new(path.to_str().unwrap());
    adapter.connect().await.unwrap();
    adapter
        .execute("CREATE TABLE users (email TEXT)")
        .await
        .unwrap();
    adapter
        .execute("CREATE INDEX unique_email ON users(email)")
        .await
        .unwrap();

    let schema = adapter.schema_info().await.unwrap();
    let index = &schema.namespaces[0].indexes[0];

    assert_eq!(index.name, "unique_email");
    assert!(!index.unique);
}

#[tokio::test]
async fn sqlite_trigger_event_ignores_comments() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("t.db");
    let mut adapter = SqliteAdapter::new(path.to_str().unwrap());
    adapter.connect().await.unwrap();
    adapter
        .execute("CREATE TABLE users (id INTEGER PRIMARY KEY)")
        .await
        .unwrap();
    adapter
        .execute(
            "CREATE TRIGGER trg_users_audit /* DELETE */ AFTER UPDATE ON users
             BEGIN SELECT NEW.id; END",
        )
        .await
        .unwrap();

    let schema = adapter.schema_info().await.unwrap();

    assert_eq!(schema.namespaces[0].triggers[0].event, "UPDATE");
}

#[test]
fn object_kind_predicates_match_explorer_actions() {
    assert!(ObjectKind::Table.supports_select_star());
    assert!(ObjectKind::View.supports_select_star());
    assert!(ObjectKind::MaterializedView.supports_select_star());
    assert!(!ObjectKind::Index.supports_select_star());
    assert_eq!(
        ObjectKind::MaterializedView.group_label(),
        "Materialized Views"
    );
}
