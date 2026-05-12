use sqrit::app::{App, FocusedPane, QueryStatus};
use sqrit::config::{Config, Connection, DbType};
use sqrit::db::sqlite::SqliteAdapter;
use sqrit::editor::EditorBuffer;
use sqrit::mode::Mode;
use sqrit::mode::editor::normal::NormalState;
use sqrit::picker::PickerState;

fn make_connected_app() -> App {
    let config = Config {
        connections: vec![Connection {
            name: "test".to_string(),
            db_type: DbType::Sqlite,
            host: None,
            port: None,
            username: None,
            password: None,
            database: None,
            file_path: Some(":memory:".to_string()),
        }],
    };
    App {
        mode: Mode::QueryNormal,
        config,
        should_quit: false,
        picker: PickerState::new(),
        db: Some(Box::new(SqliteAdapter::new(":memory:"))),
        focused_pane: FocusedPane::Query,
        editor: EditorBuffer::new(),
        normal_state: NormalState::new(),
        status_message: String::new(),
        results: None,
        query_status: QueryStatus::Idle,
        pending_query: None,
    }
}

// T12 #1: default state
#[test]
fn default_state() {
    let app = make_connected_app();
    assert!(app.results.is_none());
    assert_eq!(app.query_status, QueryStatus::Idle);
    assert!(app.pending_query.is_none());
}

// T12 #2: normal Enter sets pending_query
#[test]
fn normal_enter_sets_pending_query() {
    let mut app = make_connected_app();
    app.editor.insert_str("SELECT 1");

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);

    assert_eq!(app.pending_query.as_deref(), Some("SELECT 1"));
}

// T12 #3: Ctrl+Enter in insert mode sets pending_query
#[test]
fn insert_ctrl_enter_sets_pending_query() {
    let mut app = make_connected_app();
    app.mode = Mode::QueryInsert;
    app.editor.insert_str("SELECT 1");

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::CONTROL,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);

    assert_eq!(app.pending_query.as_deref(), Some("SELECT 1"));
}

// T12 #4: regular Enter in insert mode does NOT set pending_query
#[test]
fn insert_enter_does_not_set_pending_query() {
    let mut app = make_connected_app();
    app.mode = Mode::QueryInsert;
    app.editor.insert_str("hello");

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);

    assert!(app.pending_query.is_none());
    assert_eq!(app.editor.text(), "hello\n");
}

// T12 #5: execute with SQLite stores results
#[tokio::test]
async fn execute_stores_results() {
    let mut app = make_connected_app();
    if let Some(ref mut db) = app.db {
        db.connect().await.unwrap();
    }

    app.editor.insert_str("SELECT 1 AS val");
    app.pending_query = Some(app.editor.text());
    app.execute_pending().await;

    assert!(app.results.is_some());
    assert_eq!(app.query_status, QueryStatus::Success);
    assert!(app.pending_query.is_none());

    let results = app.results.unwrap();
    assert_eq!(results.columns, vec!["val".to_string()]);
    assert_eq!(results.rows.len(), 1);
}

// T12 #6: execute invalid SQL stores error
#[tokio::test]
async fn execute_invalid_sql_stores_error() {
    let mut app = make_connected_app();
    if let Some(ref mut db) = app.db {
        db.connect().await.unwrap();
    }

    app.pending_query = Some("INVALID SQL !!@@".to_string());
    app.execute_pending().await;

    assert!(app.results.is_none());
    assert!(matches!(app.query_status, QueryStatus::Error(_)));
    assert!(app.pending_query.is_none());
}
