mod common;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sqrit::app::{App, FocusedPane, QueryStatus};
use sqrit::config::DbType;
use sqrit::db::types::{ColumnInfo, IndexObject, Namespace, ObjectKind, SchemaInfo, TableObject};
use sqrit::explorer::{NodeKey, TreeItem};
use sqrit::mode::Mode;

fn make_schema(namespace: &str) -> SchemaInfo {
    let mut namespace = Namespace::empty(namespace);
    namespace.tables.push(TableObject {
        name: "users".to_string(),
        columns: vec![ColumnInfo {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            is_primary_key: true,
        }],
    });
    namespace.indexes.push(IndexObject {
        name: "idx_users".to_string(),
        table: "users".to_string(),
        unique: false,
    });
    SchemaInfo {
        namespaces: vec![namespace],
    }
}

fn make_explorer_app(db_type: DbType, namespace: &str) -> App {
    let mut app = common::test_app();
    app.config.connections[0].db_type = db_type;
    app.active_connection = Some("test".to_string());
    app.mode = Mode::Explorer;
    app.focused_pane = FocusedPane::Explorer;
    app.explorer_state.set_schema(make_schema(namespace));
    app
}

fn press(app: &mut App, code: KeyCode) {
    let mode = app.mode;
    mode.handle_key(KeyEvent::new(code, KeyModifiers::NONE), app);
}

fn select_object(app: &mut App, kind: ObjectKind) {
    app.explorer_state.toggle_key(NodeKey::Group {
        ns: app.explorer_state.schema.as_ref().unwrap().namespaces[0]
            .name
            .clone(),
        kind,
    });
    app.explorer_state.selected = app
        .explorer_state
        .items()
        .iter()
        .position(
            |item| matches!(item, TreeItem::Object { kind: item_kind, .. } if *item_kind == kind),
        )
        .unwrap();
}

#[test]
fn s_on_sqlite_table_quotes_unqualified_name() {
    let mut app = make_explorer_app(DbType::Sqlite, "");
    select_object(&mut app, ObjectKind::Table);

    press(&mut app, KeyCode::Char('s'));

    assert_eq!(
        app.pending_query.as_deref(),
        Some(r#"SELECT * FROM "users" LIMIT 100"#)
    );
    assert_eq!(app.mode, Mode::Results);
}

#[test]
fn s_on_postgres_table_quotes_qualified_name() {
    let mut app = make_explorer_app(DbType::Postgres, "public");
    select_object(&mut app, ObjectKind::Table);

    press(&mut app, KeyCode::Char('s'));

    assert_eq!(
        app.pending_query.as_deref(),
        Some(r#"SELECT * FROM "public"."users" LIMIT 100"#)
    );
}

#[test]
fn s_on_mysql_table_quotes_qualified_name() {
    let mut app = make_explorer_app(DbType::Mysql, "sqrit_test");
    select_object(&mut app, ObjectKind::Table);

    press(&mut app, KeyCode::Char('s'));

    assert_eq!(
        app.pending_query.as_deref(),
        Some("SELECT * FROM `sqrit_test`.`users` LIMIT 100")
    );
}

#[test]
fn s_on_column_uses_parent_table() {
    let mut app = make_explorer_app(DbType::Sqlite, "");
    select_object(&mut app, ObjectKind::Table);
    app.explorer_state.toggle_key(NodeKey::Object {
        ns: String::new(),
        kind: ObjectKind::Table,
        name: "users".to_string(),
    });
    app.explorer_state.selected = app
        .explorer_state
        .items()
        .iter()
        .position(|item| matches!(item, TreeItem::Column { name, .. } if name == "id"))
        .unwrap();

    press(&mut app, KeyCode::Char('s'));

    assert_eq!(
        app.pending_query.as_deref(),
        Some(r#"SELECT * FROM "users" LIMIT 100"#)
    );
}

#[test]
fn s_on_table_resets_previous_results_page() {
    let mut app = make_explorer_app(DbType::Sqlite, "");
    select_object(&mut app, ObjectKind::Table);
    app.results_state.page_offset = app.results_state.page_size * 2;
    app.results_state.selected_row = 4;
    app.results_state.has_next_page = true;

    press(&mut app, KeyCode::Char('s'));

    assert_eq!(app.results_state.page_offset, 0);
    assert_eq!(app.results_state.selected_row, 0);
    assert!(!app.results_state.has_next_page);
}

#[test]
fn pressing_s_on_an_index_replaces_prior_query_error() {
    let mut app = make_explorer_app(DbType::Sqlite, "");
    select_object(&mut app, ObjectKind::Index);
    app.query_status = QueryStatus::Error("old query failed".to_string());
    let before = app.pending_query.clone();

    press(&mut app, KeyCode::Char('s'));

    assert_eq!(app.pending_query, before);
    assert_eq!(app.query_status, QueryStatus::Idle);
    assert!(app.status_message.contains("no SELECT"));
    assert!(app.status_bar_text().contains("no SELECT"));
    assert!(!app.status_bar_text().contains("old query failed"));
    assert_eq!(app.mode, Mode::Explorer);
}

#[test]
fn unsupported_s_preserves_running_query_status() {
    let mut app = make_explorer_app(DbType::Sqlite, "");
    select_object(&mut app, ObjectKind::Index);
    app.query_status = QueryStatus::Running;

    press(&mut app, KeyCode::Char('s'));

    assert_eq!(app.query_status, QueryStatus::Running);
    assert!(app.pending_query.is_none());
    assert!(app.status_bar_text().contains("running..."));
    assert!(app.status_bar_text().contains("no SELECT"));
}

#[test]
fn supported_s_preserves_running_status_until_execution() {
    let mut app = make_explorer_app(DbType::Sqlite, "");
    select_object(&mut app, ObjectKind::Table);
    app.query_status = QueryStatus::Running;
    app.status_message = "stale action".to_string();

    press(&mut app, KeyCode::Char('s'));

    assert_eq!(app.query_status, QueryStatus::Running);
    assert!(app.pending_query.is_some());
    assert!(app.status_message.is_empty());
}

#[test]
fn s_on_supported_object_clears_previous_action_status() {
    let mut app = make_explorer_app(DbType::Sqlite, "");
    select_object(&mut app, ObjectKind::Index);
    press(&mut app, KeyCode::Char('s'));
    select_object(&mut app, ObjectKind::Table);
    app.query_status = QueryStatus::Error("old query failed".to_string());

    press(&mut app, KeyCode::Char('s'));

    assert_eq!(app.query_status, QueryStatus::Idle);
    assert!(app.status_message.is_empty());
    assert!(!app.status_bar_text().contains("no SELECT"));
    assert!(!app.status_bar_text().contains("old query failed"));
}

#[test]
fn e_switches_to_explorer() {
    let mut app = make_explorer_app(DbType::Sqlite, "");
    app.mode = Mode::QueryNormal;
    app.focused_pane = FocusedPane::Query;

    press(&mut app, KeyCode::Char('e'));

    assert_eq!(app.mode, Mode::Explorer);
    assert_eq!(app.focused_pane, FocusedPane::Explorer);
}
