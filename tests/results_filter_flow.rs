mod common;

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use sqrit::app::{App, FocusedPane};
use sqrit::db::types::{QueryResult, ResultColumn, Value};
use sqrit::mode::Mode;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn press(app: &mut App, codes: &[KeyCode]) {
    for c in codes {
        app.handle_key_event(key(*c));
    }
}

fn seed_three_rows(app: &mut App) {
    let columns = vec![ResultColumn::untyped("name"), ResultColumn::untyped("city")];
    let rows = vec![
        {
            let mut r = HashMap::new();
            r.insert("name".into(), Value::Text("alice".into()));
            r.insert("city".into(), Value::Text("Paris".into()));
            r
        },
        {
            let mut r = HashMap::new();
            r.insert("name".into(), Value::Text("bob".into()));
            r.insert("city".into(), Value::Text("Berlin".into()));
            r
        },
        {
            let mut r = HashMap::new();
            r.insert("name".into(), Value::Text("carol".into()));
            r.insert("city".into(), Value::Text("Madrid".into()));
            r
        },
    ];
    app.results = Some(QueryResult {
        columns,
        rows,
        rows_affected: None,
        total_count: None,
    });
    app.mode = Mode::Results;
    app.focused_pane = FocusedPane::Results;
}

#[test]
fn slash_in_results_opens_filter_prompt() {
    let mut app = common::test_app();
    seed_three_rows(&mut app);

    press(&mut app, &[KeyCode::Char('/')]);

    assert_eq!(app.mode, Mode::ResultsFilter);
    assert_eq!(app.results_state.filter.as_deref(), Some(""));
}

#[test]
fn slash_in_results_recomputes_empty_filter_hits() {
    let mut app = common::test_app();
    seed_three_rows(&mut app);

    press(
        &mut app,
        &[KeyCode::Char('/'), KeyCode::Char('b'), KeyCode::Enter],
    );
    assert_eq!(app.results_state.filter.as_deref(), Some("b"));
    let result = app.results.as_ref().expect("results should be loaded");
    assert_eq!(
        app.results_state.visible_row_indices(result),
        vec![1],
        "visible rows should reflect non-empty filter hits"
    );
    assert!(
        app.results_state
            .filter_hits
            .iter()
            .any(|hit| !hit.matches.is_empty()),
        "non-empty fuzzy filter should cache highlight ranges"
    );

    press(&mut app, &[KeyCode::Char('/')]);

    assert_eq!(app.mode, Mode::ResultsFilter);
    assert_eq!(app.results_state.filter.as_deref(), Some(""));
    assert_eq!(
        app.results_state.filter_hits.len(),
        app.results.as_ref().unwrap().rows.len(),
        "empty prompt should cache all loaded rows"
    );
    let result = app.results.as_ref().expect("results should be loaded");
    assert_eq!(
        app.results_state.visible_row_indices(result),
        vec![0, 1, 2],
        "visible rows should reflect empty filter hits (all rows visible)"
    );
    assert!(
        app.results_state
            .filter_hits
            .iter()
            .all(|hit| hit.matches.is_empty()),
        "opening an empty prompt must clear stale fuzzy highlight ranges"
    );
}

#[test]
fn typing_in_filter_mode_live_filters_visible_rows() {
    let mut app = common::test_app();
    seed_three_rows(&mut app);

    press(
        &mut app,
        &[KeyCode::Char('/'), KeyCode::Char('b'), KeyCode::Char('o')],
    );

    let result = app.results.as_ref().unwrap();
    let visible = app.results_state.visible_row_indices(result);
    assert_eq!(visible, vec![1], "only 'bob' matches 'bo'");
}

#[test]
fn global_shortcut_keys_remain_literal_in_filter_mode() {
    let mut app = common::test_app();
    seed_three_rows(&mut app);

    press(
        &mut app,
        &[KeyCode::Char('/'), KeyCode::Char('?'), KeyCode::Char(' ')],
    );

    assert_eq!(app.mode, Mode::ResultsFilter);
    assert_eq!(app.results_state.filter.as_deref(), Some("? "));
    assert!(app.help.is_none());
    assert!(!app.pending_space);
}

#[test]
fn enter_locks_filter_and_returns_to_results_mode() {
    let mut app = common::test_app();
    seed_three_rows(&mut app);

    press(
        &mut app,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('a'),
            KeyCode::Char('r'),
            KeyCode::Enter,
        ],
    );

    assert_eq!(app.mode, Mode::Results);
    assert_eq!(app.results_state.filter.as_deref(), Some("ar"));
}

#[test]
fn esc_in_filter_mode_cancels_and_clears_filter_hits() {
    let mut app = common::test_app();
    seed_three_rows(&mut app);

    press(&mut app, &[KeyCode::Char('/'), KeyCode::Char('a')]);
    assert!(
        !app.results_state.filter_hits.is_empty(),
        "typing a filter should populate fuzzy highlight hits"
    );

    press(&mut app, &[KeyCode::Esc]);

    assert_eq!(app.mode, Mode::Results);
    assert_eq!(app.results_state.filter, None);
    assert!(
        app.results_state.filter_hits.is_empty(),
        "canceling the filter must clear stale fuzzy highlight hits"
    );
}

#[test]
fn comma_c_clears_a_locked_filter() {
    let mut app = common::test_app();
    seed_three_rows(&mut app);

    press(
        &mut app,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('a'),
            KeyCode::Enter,
            KeyCode::Char(','),
            KeyCode::Char('c'),
        ],
    );

    assert_eq!(app.mode, Mode::Results);
    assert_eq!(app.results_state.filter, None);
}

#[test]
fn backspace_in_filter_mode_removes_last_char() {
    let mut app = common::test_app();
    seed_three_rows(&mut app);

    press(
        &mut app,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('a'),
            KeyCode::Char('b'),
            KeyCode::Backspace,
        ],
    );

    assert_eq!(app.results_state.filter.as_deref(), Some("a"));
}

#[test]
fn selection_snaps_to_first_filtered_row_when_current_is_excluded() {
    let mut app = common::test_app();
    seed_three_rows(&mut app);
    app.results_state.selected_row = 2; // carol

    press(
        &mut app,
        &[KeyCode::Char('/'), KeyCode::Char('b'), KeyCode::Char('o')],
    );

    let result = app.results.as_ref().unwrap();
    let visible = app.results_state.visible_row_indices(result);
    assert_eq!(visible, vec![1]);
    assert_eq!(
        app.results_state.selected_row, 1,
        "selection snaps to first filtered row (bob)"
    );
}

#[test]
fn filter_persists_across_page_change() {
    let mut app = common::test_app();
    seed_three_rows(&mut app);

    press(
        &mut app,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('l'),
            KeyCode::Char('i'),
            KeyCode::Enter,
        ],
    );
    assert_eq!(app.results_state.filter.as_deref(), Some("li"));

    app.last_query = Some("SELECT 1".to_string());
    press(&mut app, &[KeyCode::PageDown]);

    assert_eq!(
        app.results_state.filter.as_deref(),
        Some("li"),
        "filter must survive PgDn"
    );
}

#[test]
fn navigation_j_skips_filtered_out_rows() {
    let mut app = common::test_app();
    seed_three_rows(&mut app);

    // Filter "li" matches alice (li) + Berlin (li); carol/Madrid are hidden.
    press(
        &mut app,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('l'),
            KeyCode::Char('i'),
            KeyCode::Enter,
        ],
    );
    let result = app.results.as_ref().unwrap();
    assert_eq!(app.results_state.visible_row_indices(result), vec![0, 1]);

    app.results_state.selected_row = 0;
    press(&mut app, &[KeyCode::Char('j')]);
    assert_eq!(
        app.results_state.selected_row, 1,
        "j moves to next visible row (skipping the hidden one)"
    );
}

#[test]
fn snap_clamps_scroll_row_within_filtered_set() {
    let mut app = common::test_app();
    seed_three_rows(&mut app);
    app.results_state.visible_rows = 5;
    app.results_state.scroll_row = 2;
    app.results_state.selected_row = 2;

    // Apply a filter that yields a single visible row — scroll_row=2 would
    // render an empty table if not clamped.
    press(
        &mut app,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('b'),
            KeyCode::Char('o'),
            KeyCode::Enter,
        ],
    );

    let result = app.results.as_ref().unwrap();
    let visible = app.results_state.visible_row_indices(result);
    assert_eq!(visible, vec![1]);
    assert_eq!(
        app.results_state.scroll_row, 0,
        "scroll_row must clamp to max(0, visible.len() - visible_rows) when the filtered set shrinks"
    );
}

#[test]
fn typed_and_pasted_controls_do_not_change_filter_matches() {
    let mut typed = common::test_app();
    seed_three_rows(&mut typed);
    press(
        &mut typed,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('b'),
            KeyCode::Char('\u{7}'),
            KeyCode::Char('o'),
        ],
    );
    let typed_rows = typed
        .results_state
        .visible_row_indices(typed.results.as_ref().unwrap());

    let mut pasted = common::test_app();
    seed_three_rows(&mut pasted);
    press(&mut pasted, &[KeyCode::Char('/')]);
    Mode::ResultsFilter
        .handler()
        .handle_paste("b\u{7}o", &mut pasted);
    let pasted_rows = pasted
        .results_state
        .visible_row_indices(pasted.results.as_ref().unwrap());

    assert_eq!([typed_rows, pasted_rows], [vec![1], vec![1]]);
}
