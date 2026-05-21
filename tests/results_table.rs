mod common;

use sqrit::app::{App, FocusedPane, QueryStatus};
use sqrit::mode::Mode;
use sqrit::results::ResultsState;

// T13 #1: default state
#[test]
fn default_state() {
    let state = ResultsState::new();
    assert_eq!(state.selected_row, 0);
    assert_eq!(state.selected_col, 0);
    assert_eq!(state.scroll_row, 0);
}

// T13 #2: move down/up clamps to row count
#[test]
fn move_down_up() {
    let mut state = ResultsState::new();

    state.move_down(5);
    assert_eq!(state.selected_row, 1);

    state.move_down(5);
    assert_eq!(state.selected_row, 2);

    // clamp at max
    state.move_down(3);
    assert_eq!(state.selected_row, 2);

    state.move_up();
    assert_eq!(state.selected_row, 1);

    state.move_up();
    assert_eq!(state.selected_row, 0);

    // clamp at 0
    state.move_up();
    assert_eq!(state.selected_row, 0);
}

// T13 #3: move left/right clamps to col count
#[test]
fn move_left_right() {
    let mut state = ResultsState::new();

    state.move_right(3);
    assert_eq!(state.selected_col, 1);

    state.move_right(3);
    assert_eq!(state.selected_col, 2);

    // clamp at max
    state.move_right(3);
    assert_eq!(state.selected_col, 2);

    state.move_left();
    assert_eq!(state.selected_col, 1);

    state.move_left();
    assert_eq!(state.selected_col, 0);

    // clamp at 0
    state.move_left();
    assert_eq!(state.selected_col, 0);
}

// T13 #4: scroll follows selection beyond visible area
#[test]
fn scroll_follows_selection() {
    let mut state = ResultsState::new();
    state.visible_rows = 3;

    // Move past visible area
    for _ in 0..3 {
        state.move_down(100);
    }
    assert_eq!(state.selected_row, 3);
    assert_eq!(state.scroll_row, 1); // scroll adjusted: 3 - 3 + 1 = 1

    // Move back up
    state.move_up();
    state.move_up();
    assert_eq!(state.selected_row, 1);
    assert_eq!(state.scroll_row, 1);

    state.move_up();
    state.move_up();
    assert_eq!(state.selected_row, 0);
    assert_eq!(state.scroll_row, 0); // scroll follows up
}

// T13 #5: empty results — navigation no-op
#[test]
fn empty_results_noop() {
    let mut state = ResultsState::new();
    state.move_down(0);
    assert_eq!(state.selected_row, 0);
    state.move_right(0);
    assert_eq!(state.selected_col, 0);
}

fn make_results_app(rows: usize) -> App {
    let mut app = common::test_app();
    app.mode = Mode::Results;
    app.focused_pane = FocusedPane::Results;
    app.query_status = QueryStatus::Success;

    let mut results = sqrit::db::types::QueryResult::empty();
    results.columns = vec![
        sqrit::db::types::ResultColumn::untyped("a"),
        sqrit::db::types::ResultColumn::untyped("b"),
    ];
    for i in 0..rows {
        let mut row = std::collections::HashMap::new();
        row.insert("a".to_string(), sqrit::db::types::Value::Integer(i as i64));
        row.insert(
            "b".to_string(),
            sqrit::db::types::Value::Text(format!("val{}", i)),
        );
        results.rows.push(row);
    }
    app.results = Some(results);
    app
}

// T13 #6: results mode h/j/k/l dispatches to ResultsState
#[test]
fn results_mode_navigation() {
    let mut app = make_results_app(5);

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('j'),
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);
    assert_eq!(app.results_state.selected_row, 1);

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('l'),
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);
    assert_eq!(app.results_state.selected_col, 1);
}

// T13 #7: results pane renders data from QueryResult
#[test]
fn results_pane_has_data() {
    let app = make_results_app(3);
    let results = app.results.as_ref().unwrap();
    assert_eq!(results.columns.len(), 2);
    assert_eq!(results.rows.len(), 3);
    assert_eq!(results.columns[0].name, "a");
    assert_eq!(results.columns[1].name, "b");
}
