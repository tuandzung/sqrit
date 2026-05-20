use sqrit::app::{App, FocusedPane, QueryStatus};
use sqrit::config::{Config, Connection, DbType};
use sqrit::db::sqlite::SqliteAdapter;
use sqrit::editor::EditorBuffer;
use sqrit::explorer::ExplorerState;
use sqrit::mode::editor::normal::NormalState;
use sqrit::mode::Mode;
use sqrit::picker::PickerState;

pub fn test_app() -> App {
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
        query_id: 0,
        command_buffer: String::new(),
        command_origin: None,
    }
}

#[allow(dead_code)]
pub async fn wait_for_query(app: &mut App, timeout: std::time::Duration) {
    let start = std::time::Instant::now();
    loop {
        app.drain_async_results();
        if app.query_status != QueryStatus::Running {
            break;
        }
        if start.elapsed() >= timeout {
            panic!("Timed out waiting for query completion after {:?}", timeout);
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

#[allow(dead_code)]
pub async fn wait_for_connect(app: &mut App, timeout: std::time::Duration) {
    let start = std::time::Instant::now();
    loop {
        app.drain_async_results();
        if app.explorer_state.schema.is_some() || matches!(app.query_status, QueryStatus::Error(_))
        {
            return;
        }
        if start.elapsed() >= timeout {
            panic!("Timed out waiting for connect result after {:?}", timeout);
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}
