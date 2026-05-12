use sqrit::clipboard::{format_cell, format_row, format_all, format_csv, format_json};
use sqrit::app::{App, FocusedPane, QueryStatus};
use sqrit::config::{Config, Connection, DbType};
use sqrit::db::sqlite::SqliteAdapter;
use sqrit::editor::EditorBuffer;
use sqrit::mode::Mode;
use sqrit::mode::editor::normal::NormalState;
use sqrit::picker::PickerState;
use sqrit::results::ResultsState;
use sqrit::explorer::ExplorerState;
use sqrit::db::types::{QueryResult, Value};

fn make_result() -> QueryResult {
    let mut result = QueryResult::empty();
    result.columns = vec!["name".to_string(), "age".to_string()];
    let mut r1 = std::collections::HashMap::new();
    r1.insert("name".to_string(), Value::Text("alice".to_string()));
    r1.insert("age".to_string(), Value::Integer(30));
    result.rows.push(r1);
    let mut r2 = std::collections::HashMap::new();
    r2.insert("name".to_string(), Value::Text("bob".to_string()));
    r2.insert("age".to_string(), Value::Integer(25));
    result.rows.push(r2);
    result
}

// T15 #1: format_cell returns value at row/col
#[test]
fn format_cell_returns_value() {
    let result = make_result();
    assert_eq!(format_cell(&result, 0, 0), Some("alice".to_string()));
    assert_eq!(format_cell(&result, 0, 1), Some("30".to_string()));
    assert_eq!(format_cell(&result, 1, 0), Some("bob".to_string()));
}

// T15 #2: format_cell returns None for out of bounds
#[test]
fn format_cell_out_of_bounds() {
    let result = make_result();
    assert_eq!(format_cell(&result, 5, 0), None);
    assert_eq!(format_cell(&result, 0, 5), None);
}

// T15 #3: format_row returns tab-separated values
#[test]
fn format_row_returns_tsv() {
    let result = make_result();
    assert_eq!(format_row(&result, 0), Some("alice\t30".to_string()));
    assert_eq!(format_row(&result, 1), Some("bob\t25".to_string()));
}

// T15 #4: format_all returns header + rows as TSV
#[test]
fn format_all_returns_tsv() {
    let result = make_result();
    let text = format_all(&result);
    assert_eq!(text, "name\tage\nalice\t30\nbob\t25");
}

// T15 #5: format_csv returns CSV with header
#[test]
fn format_csv_returns_csv() {
    let result = make_result();
    let text = format_csv(&result);
    assert_eq!(text, "name,age\nalice,30\nbob,25");
}

// T15 #6: format_csv escapes commas and quotes
#[test]
fn format_csv_escapes() {
    let mut result = QueryResult::empty();
    result.columns = vec!["name".to_string()];
    let mut r = std::collections::HashMap::new();
    r.insert("name".to_string(), Value::Text("a,b".to_string()));
    result.rows.push(r);
    let mut r2 = std::collections::HashMap::new();
    r2.insert("name".to_string(), Value::Text("he said \"hi\"".to_string()));
    result.rows.push(r2);
    let text = format_csv(&result);
    assert_eq!(text, "name\n\"a,b\"\n\"he said \"\"hi\"\"\"");
}

// T15 #7: format_json returns JSON array of objects
#[test]
fn format_json_returns_json() {
    let result = make_result();
    let text = format_json(&result);
    assert_eq!(text, "[{\"name\":\"alice\",\"age\":\"30\"},{\"name\":\"bob\",\"age\":\"25\"}]");

    // Verify valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed[0]["name"], "alice");
    assert_eq!(parsed[1]["age"], "25");
}

fn make_copy_app() -> App {
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
    let mut results = make_result();
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
        results: Some(results),
        query_status: QueryStatus::Success,
        pending_query: None,
        last_query: None,
        explorer_state: ExplorerState::new(),
        pending_space: false,
        results_state: ResultsState::new(),
    }
}

fn press(app: &mut App, code: crossterm::event::KeyCode) {
    let key = crossterm::event::KeyEvent::new(code, crossterm::event::KeyModifiers::NONE);
    let mode = app.mode;
    mode.handle_key(key, app);
}

// T15 #8: yc copies cell to status message
#[test]
fn yc_copies_cell() {
    let mut app = make_copy_app();
    press(&mut app, crossterm::event::KeyCode::Char('y'));
    press(&mut app, crossterm::event::KeyCode::Char('c'));
    assert!(app.status_message.contains("alice"));
}

// T15 #9: yy copies row to status message
#[test]
fn yy_copies_row() {
    let mut app = make_copy_app();
    press(&mut app, crossterm::event::KeyCode::Char('y'));
    press(&mut app, crossterm::event::KeyCode::Char('y'));
    assert!(app.status_message.contains("alice"));
    assert!(app.status_message.contains("30"));
}

// T15 #10: ya copies all to status message
#[test]
fn ya_copies_all() {
    let mut app = make_copy_app();
    press(&mut app, crossterm::event::KeyCode::Char('y'));
    press(&mut app, crossterm::event::KeyCode::Char('a'));
    assert!(app.status_message.contains("2 rows"));
}
