mod common;

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use sqrit::app::App;
use sqrit::cell_viewer::ViewMode;
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

fn seed_results(app: &mut App, columns: &[&str], rows: Vec<Vec<Value>>) {
    let typed: Vec<(&str, Option<&str>)> = columns.iter().map(|n| (*n, None)).collect();
    seed_results_with_types(app, &typed, rows);
}

fn seed_results_with_types(app: &mut App, columns: &[(&str, Option<&str>)], rows: Vec<Vec<Value>>) {
    let columns: Vec<ResultColumn> = columns
        .iter()
        .map(|(name, ty)| ResultColumn {
            name: name.to_string(),
            data_type: ty.map(|s| s.to_string()),
        })
        .collect();
    let rows: Vec<HashMap<String, Value>> = rows
        .into_iter()
        .map(|values| {
            columns
                .iter()
                .map(|c| c.name.clone())
                .zip(values)
                .collect::<HashMap<_, _>>()
        })
        .collect();
    app.results = Some(QueryResult {
        columns,
        rows,
        rows_affected: None,
        total_count: None,
    });
    app.mode = Mode::Results;
    app.focused_pane = sqrit::app::FocusedPane::Results;
}

// --- Slice 6: 'v' from Results opens the cell viewer with the selected cell ---

#[test]
fn v_in_results_opens_cell_viewer_on_selected_cell() {
    let mut app = common::test_app();
    seed_results(
        &mut app,
        &["name", "age"],
        vec![vec![Value::Text("alice".to_string()), Value::Integer(30)]],
    );
    app.results_state.selected_col = 1;

    press(&mut app, &[KeyCode::Char('v')]);

    assert_eq!(app.mode, Mode::CellViewer);
    let state = app.cell_viewer.as_ref().expect("cell_viewer state set");
    assert_eq!(state.column, "age");
    assert_eq!(state.value, Value::Integer(30));
    assert_eq!(state.view, ViewMode::Raw);
}

#[test]
fn v_without_results_is_a_noop() {
    let mut app = common::test_app();
    app.mode = Mode::Results;
    // No app.results set.

    press(&mut app, &[KeyCode::Char('v')]);

    assert_eq!(app.mode, Mode::Results, "must stay in Results");
    assert!(app.cell_viewer.is_none());
}

// --- Slice 7: Tab toggles raw <-> formatted, modal stays open ---

#[test]
fn tab_toggles_view_mode_in_cell_viewer() {
    let mut app = common::test_app();
    seed_results(
        &mut app,
        &["payload"],
        vec![vec![Value::Text(r#"{"x":1}"#.to_string())]],
    );

    press(&mut app, &[KeyCode::Char('v'), KeyCode::Tab]);

    let state = app.cell_viewer.as_ref().unwrap();
    assert_eq!(state.view, ViewMode::Formatted);
    assert_eq!(app.mode, Mode::CellViewer, "modal must stay open");

    press(&mut app, &[KeyCode::Tab]);
    let state = app.cell_viewer.as_ref().unwrap();
    assert_eq!(state.view, ViewMode::Raw);
}

#[test]
fn displayed_string_follows_current_view_mode() {
    let mut app = common::test_app();
    seed_results(
        &mut app,
        &["payload"],
        vec![vec![Value::Text(r#"{"x":1}"#.to_string())]],
    );

    press(&mut app, &[KeyCode::Char('v')]);
    let raw = app.cell_viewer.as_ref().unwrap().displayed();
    press(&mut app, &[KeyCode::Tab]);
    let pretty = app.cell_viewer.as_ref().unwrap().displayed();

    assert_eq!(raw, r#"{"x":1}"#);
    assert!(pretty.contains('\n'), "formatted JSON should pretty-print");
}

// --- Slice 8: 'y' copies the currently displayed string ---

#[test]
fn y_in_cell_viewer_surfaces_copy_status() {
    let mut app = common::test_app();
    seed_results(
        &mut app,
        &["name"],
        vec![vec![Value::Text("alice".to_string())]],
    );

    press(&mut app, &[KeyCode::Char('v'), KeyCode::Char('y')]);

    assert!(
        app.status_message.to_lowercase().contains("copied"),
        "status should confirm copy, got: {:?}",
        app.status_message
    );
    assert_eq!(app.mode, Mode::CellViewer, "modal stays open after copy");
}

// --- Slice 9: Esc closes and returns to Results ---

#[test]
fn esc_closes_cell_viewer_and_returns_to_results() {
    let mut app = common::test_app();
    seed_results(&mut app, &["name"], vec![vec![Value::Text("a".into())]]);

    press(&mut app, &[KeyCode::Char('v'), KeyCode::Esc]);

    assert_eq!(app.mode, Mode::Results);
    assert!(app.cell_viewer.is_none());
}

// --- Slice (issue #45): cell viewer routes column data_type to formatter ---

#[test]
fn v_on_timestamptz_column_opens_with_column_type_hint() {
    let mut app = common::test_app();
    seed_results_with_types(
        &mut app,
        &[("ts", Some("timestamptz"))],
        vec![vec![Value::Text("2026-05-21T03:00:00Z".to_string())]],
    );

    press(&mut app, &[KeyCode::Char('v')]);

    let state = app.cell_viewer.as_ref().expect("cell viewer open");
    assert_eq!(state.column_type.as_deref(), Some("timestamptz"));
}

#[test]
fn formatted_view_on_timestamptz_renders_via_chrono() {
    let mut app = common::test_app();
    seed_results_with_types(
        &mut app,
        &[("ts", Some("timestamptz"))],
        vec![vec![Value::Text("2026-05-21T03:00:00Z".to_string())]],
    );

    press(&mut app, &[KeyCode::Char('v'), KeyCode::Tab]);

    let displayed = app.cell_viewer.as_ref().unwrap().displayed();
    assert!(
        !displayed.contains('T'),
        "formatted timestamp should use a space separator, got: {:?}",
        displayed
    );
    assert!(
        displayed.contains('+') || displayed.contains('-'),
        "formatted timestamp should include a numeric offset, got: {:?}",
        displayed
    );
}

#[test]
fn formatted_view_on_text_column_with_iso_string_stays_raw() {
    // Column has no declared type → ISO string text must NOT be re-rendered.
    let mut app = common::test_app();
    seed_results_with_types(
        &mut app,
        &[("note", None)],
        vec![vec![Value::Text("2026-05-21T03:00:00Z".to_string())]],
    );

    press(&mut app, &[KeyCode::Char('v'), KeyCode::Tab]);

    let displayed = app.cell_viewer.as_ref().unwrap().displayed();
    assert_eq!(displayed, "2026-05-21T03:00:00Z");
}

#[test]
fn unmapped_keys_in_cell_viewer_do_not_dismiss() {
    let mut app = common::test_app();
    seed_results(&mut app, &["name"], vec![vec![Value::Text("a".into())]]);

    press(
        &mut app,
        &[
            KeyCode::Char('v'),
            KeyCode::Char('q'),
            KeyCode::Char('e'),
            KeyCode::Char('?'),
        ],
    );

    assert_eq!(app.mode, Mode::CellViewer);
    assert!(app.cell_viewer.is_some());
}
