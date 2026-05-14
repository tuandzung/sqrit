use sqrit::app::{App, FocusedPane, QueryStatus};
use sqrit::config::{Config, Connection, DbType};
use sqrit::db::sqlite::SqliteAdapter;
use sqrit::db::types::{ColumnInfo, SchemaInfo, TableInfo, ViewInfo};
use sqrit::editor::EditorBuffer;
use sqrit::explorer::{ExplorerState, TreeItem};
use sqrit::mode::Mode;
use sqrit::mode::editor::normal::NormalState;
use sqrit::picker::PickerState;
use sqrit::results::ResultsState;

fn make_schema() -> SchemaInfo {
    SchemaInfo {
        tables: vec![
            TableInfo {
                name: "users".to_string(),
                columns: vec![
                    ColumnInfo { name: "id".to_string(), data_type: "INTEGER".to_string(), nullable: false, is_primary_key: true },
                    ColumnInfo { name: "name".to_string(), data_type: "TEXT".to_string(), nullable: false, is_primary_key: false },
                ],
            },
            TableInfo {
                name: "orders".to_string(),
                columns: vec![
                    ColumnInfo { name: "id".to_string(), data_type: "INTEGER".to_string(), nullable: false, is_primary_key: true },
                ],
            },
        ],
        views: vec![],
    }
}

// T16 #1: items() returns table names collapsed
#[test]
fn items_shows_collapsed_tables() {
    let mut state = ExplorerState::new();
    state.schema = Some(make_schema());

    let items = state.items();
    assert_eq!(items.len(), 2);
    assert!(matches!(&items[0], TreeItem::Table { name, expanded } if name == "users" && !expanded));
    assert!(matches!(&items[1], TreeItem::Table { name, expanded } if name == "orders" && !expanded));
}

// T16 #2: toggle expands table, items shows columns
#[test]
fn toggle_expand_shows_columns() {
    let mut state = ExplorerState::new();
    state.schema = Some(make_schema());
    state.toggle("users");

    let items = state.items();
    assert_eq!(items.len(), 4); // users + 2 cols + orders
    assert!(matches!(&items[0], TreeItem::Table { name, expanded } if name == "users" && *expanded));
    assert!(matches!(&items[1], TreeItem::Column { name, .. } if name == "id"));
    assert!(matches!(&items[2], TreeItem::Column { name, .. } if name == "name"));
    assert!(matches!(&items[3], TreeItem::Table { name, .. } if name == "orders"));
}

// T16 #3: toggle collapse hides columns
#[test]
fn toggle_collapse_hides_columns() {
    let mut state = ExplorerState::new();
    state.schema = Some(make_schema());
    state.toggle("users");
    state.toggle("users");

    let items = state.items();
    assert_eq!(items.len(), 2);
    assert!(matches!(&items[0], TreeItem::Table { expanded, .. } if !expanded));
}

// T16 #4: move_down/move_up navigates selection
#[test]
fn move_down_up_navigates() {
    let mut state = ExplorerState::new();
    state.schema = Some(make_schema());

    assert_eq!(state.selected, 0);
    state.move_down();
    assert_eq!(state.selected, 1);
    state.move_down();
    assert_eq!(state.selected, 1); // clamp at last item

    state.move_up();
    assert_eq!(state.selected, 0);
    state.move_up();
    assert_eq!(state.selected, 0); // clamp at 0
}

fn make_explorer_app() -> App {
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
    let mut explorer_state = ExplorerState::new();
    explorer_state.schema = Some(make_schema());
    let (async_tx, async_rx) = tokio::sync::mpsc::unbounded_channel();
    App {
        mode: Mode::Explorer,
        config,
        should_quit: false,
        picker: PickerState::new(),
        db: Some(Box::new(SqliteAdapter::new(":memory:"))),
        focused_pane: FocusedPane::Explorer,
        editor: EditorBuffer::new(),
        normal_state: NormalState::new(),
        status_message: String::new(),
        results: None,
        query_status: QueryStatus::Idle,
        pending_query: None,
        last_query: None,
        results_state: ResultsState::new(),
        explorer_state,
        pending_space: false,
            maximized: None,
            autocomplete: sqrit::autocomplete::AutocompleteState::new(),
            active_connection: None,
        last_keystroke: None,
            pending_schema_load: false,
        async_rx,
        async_tx,
    }
}

// T16 #5: Enter toggles expand on selected table
#[test]
fn enter_toggles_expand() {
    let mut app = make_explorer_app();

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);

    let items = app.explorer_state.items();
    assert_eq!(items.len(), 4); // users expanded (2 cols) + orders
}
