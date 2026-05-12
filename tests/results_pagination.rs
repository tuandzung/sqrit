use sqrit::app::{App, FocusedPane, QueryStatus};
use sqrit::config::{Config, Connection, DbType};
use sqrit::db::sqlite::SqliteAdapter;
use sqrit::editor::EditorBuffer;
use sqrit::mode::Mode;
use sqrit::mode::editor::normal::NormalState;
use sqrit::picker::PickerState;
use sqrit::results::ResultsState;

// T14 #1: page_down advances offset by page_size, resets selection
#[test]
fn page_down_advances_offset() {
    let mut state = ResultsState::new();
    state.page_size = 10;
    state.selected_row = 5;
    state.has_next_page = true;

    state.page_down();

    assert_eq!(state.page_offset, 10);
    assert_eq!(state.selected_row, 0);
    assert_eq!(state.scroll_row, 0);
}

// T14 #2: page_up decreases offset, floor 0
#[test]
fn page_up_decreases_offset() {
    let mut state = ResultsState::new();
    state.page_size = 10;
    state.page_offset = 30;
    state.selected_row = 5;

    state.page_up();

    assert_eq!(state.page_offset, 20);
    assert_eq!(state.selected_row, 0);
    assert_eq!(state.scroll_row, 0);
}

// T14 #3: page_up at offset 0 is no-op
#[test]
fn page_up_at_zero_noop() {
    let mut state = ResultsState::new();
    state.page_offset = 0;
    state.selected_row = 3;

    state.page_up();

    assert_eq!(state.page_offset, 0);
    assert_eq!(state.selected_row, 3);
}

// T14 #4: reset_pagination zeroes offset, selection, has_next_page
#[test]
fn reset_pagination_clears_state() {
    let mut state = ResultsState::new();
    state.page_offset = 100;
    state.selected_row = 5;
    state.selected_col = 2;
    state.scroll_row = 3;
    state.has_next_page = true;

    state.reset_pagination();

    assert_eq!(state.page_offset, 0);
    assert_eq!(state.selected_row, 0);
    assert_eq!(state.selected_col, 0);
    assert_eq!(state.scroll_row, 0);
    assert!(!state.has_next_page);
}

fn make_paginated_app() -> App {
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
        mode: Mode::Results,
        config,
        should_quit: false,
        picker: PickerState::new(),
        db: Some(Box::new(SqliteAdapter::new(":memory:"))),
        focused_pane: FocusedPane::Results,
        editor: EditorBuffer::new(),
        normal_state: NormalState::new(),
        status_message: String::new(),
        results: Some(sqrit::db::types::QueryResult::empty()),
        query_status: QueryStatus::Success,
        pending_query: None,
        last_query: None,
        results_state: ResultsState::new(),
    }
}

// T14 #5: PgDn in results mode advances page and sets pending query
#[test]
fn results_mode_page_down() {
    let mut app = make_paginated_app();
    app.last_query = Some("SELECT * FROM t".to_string());
    app.results_state.has_next_page = true;

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);

    assert_eq!(app.results_state.page_offset, app.results_state.page_size);
    assert_eq!(app.pending_query, Some("SELECT * FROM t".to_string()));
}

// T14 #6: PgUp in results mode goes to previous page
#[test]
fn results_mode_page_up() {
    let mut app = make_paginated_app();
    app.last_query = Some("SELECT * FROM t".to_string());
    app.results_state.page_offset = app.results_state.page_size;
    app.results_state.has_next_page = true;

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::PageUp,
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);

    assert_eq!(app.results_state.page_offset, 0);
    assert_eq!(app.pending_query, Some("SELECT * FROM t".to_string()));
}

// T14 #7: execute_pending stores last_query and uses paginated execution for SELECT
#[tokio::test]
async fn execute_pending_paginates_select() {
    let mut app = make_paginated_app();
    let db = SqliteAdapter::new(":memory:");
    app.db = Some(Box::new(db));
    app.db.as_mut().unwrap().connect().await.unwrap();

    // Create table with 3 rows
    app.db.as_ref().unwrap().execute("CREATE TABLE t (a INTEGER)").await.unwrap();
    app.db.as_ref().unwrap().execute("INSERT INTO t VALUES (1)").await.unwrap();
    app.db.as_ref().unwrap().execute("INSERT INTO t VALUES (2)").await.unwrap();
    app.db.as_ref().unwrap().execute("INSERT INTO t VALUES (3)").await.unwrap();

    app.results_state.page_size = 2;
    app.pending_query = Some("SELECT * FROM t".to_string());
    app.results_state.reset_pagination();

    app.execute_pending().await;

    assert_eq!(app.last_query, Some("SELECT * FROM t".to_string()));
    let result = app.results.as_ref().unwrap();
    assert_eq!(result.rows.len(), 2); // page_size = 2
    assert!(app.results_state.has_next_page); // 3 total, fetched page_size+1=3, got 3 > 2

    // Page down
    app.results_state.page_down();
    app.pending_query = app.last_query.clone();
    app.execute_pending().await;

    let result = app.results.as_ref().unwrap();
    assert_eq!(result.rows.len(), 1); // remaining row
}
