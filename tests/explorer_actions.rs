mod common;

use sqrit::app::{App, FocusedPane};
use sqrit::db::types::{ColumnInfo, SchemaInfo, TableInfo};
use sqrit::mode::Mode;

fn make_schema() -> SchemaInfo {
    SchemaInfo {
        tables: vec![TableInfo {
            name: "users".to_string(),
            columns: vec![ColumnInfo {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                is_primary_key: true,
            }],
        }],
        views: vec![],
    }
}

fn make_explorer_app() -> App {
    let mut app = common::test_app();
    app.mode = Mode::Explorer;
    app.focused_pane = FocusedPane::Explorer;
    app.explorer_state.schema = Some(make_schema());
    app
}

// T17 #1: s on table sets pending_query with SELECT * FROM
#[test]
fn s_on_table_sets_query() {
    let mut app = make_explorer_app();

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('s'),
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);

    assert_eq!(
        app.pending_query,
        Some("SELECT * FROM users LIMIT 100".to_string())
    );
    assert_eq!(app.mode, Mode::Results);
}

// T17 #2: s on column uses parent table
#[test]
fn s_on_column_uses_parent_table() {
    let mut app = make_explorer_app();
    // Expand users table
    app.explorer_state.toggle("users");
    // Move to column (index 1 = id column)
    app.explorer_state.move_down();

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('s'),
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);

    assert_eq!(
        app.pending_query,
        Some("SELECT * FROM users LIMIT 100".to_string())
    );
}

// T22: e from QueryNormal switches to Explorer (was space+e, now bare e)
#[test]
fn e_switches_to_explorer() {
    let mut app = make_explorer_app();
    app.mode = Mode::QueryNormal;
    app.focused_pane = FocusedPane::Query;

    let e_key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('e'),
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(e_key, &mut app);

    assert_eq!(app.mode, Mode::Explorer);
    assert_eq!(app.focused_pane, FocusedPane::Explorer);
}
