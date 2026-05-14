use sqrit::app::{App, FocusedPane, QueryStatus};
use sqrit::config::{Config, Connection, DbType};
use sqrit::db::sqlite::SqliteAdapter;
use sqrit::editor::EditorBuffer;
use sqrit::mode::Mode;
use sqrit::mode::editor::normal::NormalState;
use sqrit::picker::PickerState;
use sqrit::explorer::ExplorerState;

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
    let (async_tx, async_rx) = tokio::sync::mpsc::unbounded_channel();
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
        last_query: None,
        explorer_state: ExplorerState::new(),
        pending_space: false,
        maximized: None,
        autocomplete: sqrit::autocomplete::AutocompleteState::new(),
        active_connection: None,
        results_state: sqrit::results::ResultsState::new(),
        last_keystroke: None,
        pending_schema_load: false,
        async_rx,
        async_tx,
    }
}

// V2: execute_pending must not block — spawns task, returns immediately
#[tokio::test]
async fn execute_pending_returns_immediately() {
    let mut app = make_connected_app();
    if let Some(ref mut db) = app.db {
        db.connect().await.unwrap();
    }

    app.editor.insert_str("SELECT 1 AS val");
    app.pending_query = Some(app.editor.text());

    // execute_pending is sync — spawns task and returns immediately
    app.execute_pending();
    assert!(app.pending_query.is_none());
    assert_eq!(app.query_status, QueryStatus::Running);
    // Results not yet available — task is running
    assert!(app.results.is_none());

    // Yield to let the spawned task complete
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    app.drain_async_results();

    assert_eq!(app.query_status, QueryStatus::Success);
    assert!(app.results.is_some());
    let result = app.results.unwrap();
    assert_eq!(result.columns, vec!["val".to_string()]);
    assert_eq!(result.rows.len(), 1);
}

// V2: query status transitions Running → Error on bad SQL
#[tokio::test]
async fn execute_pending_error_via_channel() {
    let mut app = make_connected_app();
    if let Some(ref mut db) = app.db {
        db.connect().await.unwrap();
    }

    app.pending_query = Some("INVALID SQL !!@@".to_string());
    app.execute_pending();

    assert_eq!(app.query_status, QueryStatus::Running);
    assert!(app.results.is_none());

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    app.drain_async_results();

    assert!(matches!(app.query_status, QueryStatus::Error(_)));
    assert!(app.results.is_none());
}

// V2: schema load via async task
#[tokio::test]
async fn schema_load_via_async_task() {
    let mut app = make_connected_app();
    if let Some(ref mut db) = app.db {
        db.connect().await.unwrap();
        // Create a table so schema is non-empty
        db.execute("CREATE TABLE test_v2 (id INTEGER PRIMARY KEY, name TEXT)").await.unwrap();
    }

    app.pending_schema_load = true;

    // Simulate the event loop schema load path
    if app.pending_schema_load {
        if let Some(ref db) = app.db {
            let db = db.clone_box();
            let tx = app.async_tx.clone();
            app.pending_schema_load = false;
            tokio::spawn(async move {
                if let Ok(schema) = db.schema_info().await {
                    let _ = tx.send(sqrit::app::AsyncResult::SchemaLoaded(schema));
                }
            });
        }
    }

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    app.drain_async_results();

    assert!(app.explorer_state.schema.is_some());
    let schema = app.explorer_state.schema.unwrap();
    assert!(schema.tables.iter().any(|t| t.name == "test_v2"));
}
