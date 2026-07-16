mod common;

use sqrit::app::{App, QueryStatus};
use sqrit::mode::Mode;

fn make_connected_app() -> App {
    common::test_app()
}

fn press(app: &mut App, code: crossterm::event::KeyCode) {
    app.handle_key_event(crossterm::event::KeyEvent::new(
        code,
        crossterm::event::KeyModifiers::NONE,
    ));
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
    app.results_state.page_offset = app.results_state.page_size * 2;

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);

    assert_eq!(app.pending_query.as_deref(), Some("SELECT 1"));
    assert_eq!(app.results_state.page_offset, 0);
}

// T12 #3: Ctrl+Enter in insert mode sets pending_query
#[test]
fn insert_ctrl_enter_sets_pending_query() {
    let mut app = make_connected_app();
    app.mode = Mode::QueryInsert;
    app.editor.insert_str("SELECT 1");
    app.results_state.page_offset = app.results_state.page_size * 2;

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::CONTROL,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);

    assert_eq!(app.pending_query.as_deref(), Some("SELECT 1"));
    assert_eq!(app.results_state.page_offset, 0);
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
    app.execute_pending();
    common::wait_for_query(&mut app, std::time::Duration::from_secs(5)).await;

    assert!(app.results.is_some());
    assert_eq!(app.query_status, QueryStatus::Success);
    assert!(app.pending_query.is_none());

    let results = app.results.unwrap();
    assert_eq!(results.column_names(), vec!["val"]);
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
    app.execute_pending();
    common::wait_for_query(&mut app, std::time::Duration::from_secs(5)).await;

    assert!(app.results.is_none());
    assert!(matches!(app.query_status, QueryStatus::Error(_)));
    assert!(app.pending_query.is_none());
}

#[test]
fn gs_queues_only_the_statement_under_the_cursor() {
    let mut app = make_connected_app();
    app.active_connection = Some("test".to_string());
    app.editor.insert_str("SELECT 1; SELECT 2;");
    app.results_state.page_offset = app.results_state.page_size * 2;
    for _ in 0..10 {
        press(&mut app, crossterm::event::KeyCode::Char('h'));
    }

    press(&mut app, crossterm::event::KeyCode::Char('g'));
    press(&mut app, crossterm::event::KeyCode::Char('s'));

    assert_eq!(app.pending_query.as_deref(), Some("SELECT 1;"));
    let selected = app.selected_statement.as_ref().expect("selected statement");
    assert_eq!((selected.ordinal, selected.total), (1, 2));
    assert!(app.status_message.contains("statement 1/2"));
    assert_eq!(app.results_state.page_offset, 0);
}

#[test]
fn gg_still_moves_to_the_top() {
    let mut app = make_connected_app();
    app.editor.insert_str("one\ntwo");
    press(&mut app, crossterm::event::KeyCode::Char('g'));
    press(&mut app, crossterm::event::KeyCode::Char('g'));
    assert_eq!(app.editor.cursor(), (0, 0));
}

#[test]
fn whole_buffer_keys_clear_statement_selection() {
    let mut app = make_connected_app();
    app.active_connection = Some("test".to_string());
    app.editor.insert_str("SELECT 1; SELECT 2;");
    press(&mut app, crossterm::event::KeyCode::Char('g'));
    press(&mut app, crossterm::event::KeyCode::Char('s'));
    assert!(app.selected_statement.is_some());

    press(&mut app, crossterm::event::KeyCode::Enter);
    assert_eq!(app.pending_query.as_deref(), Some("SELECT 1; SELECT 2;"));
    assert!(app.selected_statement.is_none());
    assert!(app.status_message.is_empty());
}

#[test]
fn insert_ctrl_enter_clears_statement_feedback() {
    let mut app = make_connected_app();
    app.active_connection = Some("test".to_string());
    app.editor.insert_str("SELECT 1; SELECT 2;");
    press(&mut app, crossterm::event::KeyCode::Char('g'));
    press(&mut app, crossterm::event::KeyCode::Char('s'));
    press(&mut app, crossterm::event::KeyCode::Char('i'));
    assert!(app.status_message.starts_with("running statement "));

    app.handle_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::CONTROL,
    ));

    assert_eq!(app.pending_query.as_deref(), Some("SELECT 1; SELECT 2;"));
    assert!(app.selected_statement.is_none());
    assert!(app.status_message.is_empty());
}

#[test]
fn scanner_error_queues_nothing() {
    let mut app = make_connected_app();
    app.active_connection = Some("test".to_string());
    app.query_status = QueryStatus::Running;
    app.editor.insert_str("SELECT 'open");
    press(&mut app, crossterm::event::KeyCode::Char('g'));
    press(&mut app, crossterm::event::KeyCode::Char('s'));
    assert!(app.pending_query.is_none());
    assert_eq!(app.query_status, QueryStatus::Running);
    assert_eq!(app.status_message, "unterminated single-quoted string");
    assert!(app
        .status_bar_text()
        .contains("unterminated single-quoted string"));
}

#[test]
fn idle_scanner_error_stays_out_of_database_status() {
    let mut app = make_connected_app();
    app.active_connection = Some("test".to_string());
    app.editor.insert_str("SELECT 'open");

    press(&mut app, crossterm::event::KeyCode::Char('g'));
    press(&mut app, crossterm::event::KeyCode::Char('s'));

    assert_eq!(app.query_status, QueryStatus::Idle);
    assert_eq!(app.status_message, "unterminated single-quoted string");
}

#[test]
fn empty_or_comment_only_buffer_reports_no_statement() {
    let mut app = make_connected_app();
    app.active_connection = Some("test".to_string());
    app.query_status = QueryStatus::Running;
    app.editor
        .insert_str(" -- only a comment\n/* and another */ ; ");

    press(&mut app, crossterm::event::KeyCode::Char('g'));
    press(&mut app, crossterm::event::KeyCode::Char('s'));

    assert!(app.pending_query.is_none());
    assert_eq!(app.query_status, QueryStatus::Running);
    assert_eq!(app.status_message, "no statement at cursor");
}
