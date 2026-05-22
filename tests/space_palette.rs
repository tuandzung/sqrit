mod common;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use sqrit::app::App;
use sqrit::mode::Mode;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn press(app: &mut App, codes: &[KeyCode]) {
    for c in codes {
        app.handle_key_event(key(*c));
    }
}

// --- Slice 1: <space>q quits from QueryNormal ---

#[test]
fn space_q_quits_from_query_normal() {
    let mut app = common::test_app();
    assert!(!app.should_quit);

    press(&mut app, &[KeyCode::Char(' '), KeyCode::Char('q')]);

    assert!(app.should_quit, "space-q should set should_quit");
}

// --- Slice 2: <space>q from Explorer and Results ---

#[test]
fn space_q_quits_from_explorer() {
    let mut app = common::test_app();
    app.mode = Mode::Explorer;

    press(&mut app, &[KeyCode::Char(' '), KeyCode::Char('q')]);

    assert!(app.should_quit);
}

#[test]
fn space_q_quits_from_results() {
    let mut app = common::test_app();
    app.mode = Mode::Results;

    press(&mut app, &[KeyCode::Char(' '), KeyCode::Char('q')]);

    assert!(app.should_quit);
}

// --- Slice 3: <space>c switches to Picker without disconnecting ---

#[test]
fn space_c_goes_to_picker_and_preserves_db() {
    let mut app = common::test_app();
    app.active_connection = Some("test".to_string());
    assert!(app.db.is_some());

    press(&mut app, &[KeyCode::Char(' '), KeyCode::Char('c')]);

    assert_eq!(app.mode, Mode::Picker);
    assert!(app.db.is_some(), "space-c must not disconnect");
    assert_eq!(app.active_connection.as_deref(), Some("test"));
}

// --- Slice 4: <space>x disconnects and returns to Picker ---

#[test]
fn space_x_disconnects_and_returns_to_picker() {
    use sqrit::db::types::SchemaInfo;
    let mut app = common::test_app();
    app.active_connection = Some("test".to_string());
    app.explorer_state.schema = Some(SchemaInfo {
        tables: vec![],
        views: vec![],
    });
    assert!(app.db.is_some());

    press(&mut app, &[KeyCode::Char(' '), KeyCode::Char('x')]);

    assert_eq!(app.mode, Mode::Picker);
    assert!(app.db.is_none(), "space-x should clear db");
    assert!(app.active_connection.is_none());
    assert!(app.explorer_state.schema.is_none());
}

// Slice 5 (`<space>z` cancel) is covered end-to-end by
// `v2_async_query::cancel_sets_status_and_drops_stale_result`, which exercises
// the full async flow (db.cancel + in_transaction + drain). The earlier stub
// test asserted only that something was written to status_message; the real
// flow needs a tokio runtime + drain, so we let the integration test own it.

// --- Slice 6: <space>h stub for query history ---

#[test]
fn space_h_sets_history_stub_status() {
    let mut app = common::test_app();

    press(&mut app, &[KeyCode::Char(' '), KeyCode::Char('h')]);

    assert!(
        app.status_message.to_lowercase().contains("history"),
        "expected history stub status, got: {:?}",
        app.status_message
    );
}

// --- Slice 7: in Picker mode, space types filter char (no palette) ---

#[test]
fn space_in_picker_is_filter_char_not_palette() {
    let mut app = common::test_app();
    app.mode = Mode::Picker;

    press(&mut app, &[KeyCode::Char(' ')]);

    assert!(!app.pending_space, "picker must not arm palette");
    assert_eq!(app.picker.filter, " ", "space should land in filter");
    assert_eq!(app.mode, Mode::Picker);
}

// Follow-up: a palette letter after space in Picker stays in filter — no palette dispatch.
// Use 't' (theme picker palette key) because picker has no 't' binding of its own.
#[test]
fn space_then_t_in_picker_does_not_open_theme_picker() {
    let mut app = common::test_app();
    app.mode = Mode::Picker;

    press(&mut app, &[KeyCode::Char(' '), KeyCode::Char('t')]);

    assert_eq!(
        app.mode,
        Mode::Picker,
        "picker must not transition into ThemePicker"
    );
    assert!(app.theme_picker.is_none());
    assert_eq!(app.picker.filter, " t");
}

// --- Slice 8: in QueryInsert, space inserts literal char (no palette) ---

#[test]
fn space_in_query_insert_inserts_literal() {
    let mut app = common::test_app();
    app.mode = Mode::QueryInsert;

    press(&mut app, &[KeyCode::Char(' ')]);

    assert!(!app.pending_space);
    assert_eq!(app.editor.text(), " ");
    assert_eq!(app.mode, Mode::QueryInsert);
}

#[test]
fn space_then_q_in_query_insert_does_not_quit() {
    let mut app = common::test_app();
    app.mode = Mode::QueryInsert;

    press(&mut app, &[KeyCode::Char(' '), KeyCode::Char('q')]);

    assert!(!app.should_quit);
    assert_eq!(app.editor.text(), " q");
}

// --- Unknown palette combo falls through to the mode handler ---

#[test]
fn space_then_unknown_key_passes_through() {
    let mut app = common::test_app();
    let initial_mode = app.mode;
    let initial_status = app.status_message.clone();

    // 'm' is unbound in QueryNormal and not part of the palette.
    press(&mut app, &[KeyCode::Char(' '), KeyCode::Char('m')]);

    assert!(!app.pending_space, "pending_space must be cleared");
    assert!(!app.should_quit);
    assert_eq!(app.status_message, initial_status);
    assert_eq!(app.mode, initial_mode);
}

// --- Modifier-bearing keys do not dispatch palette actions ---

#[test]
fn space_then_modified_key_does_not_dispatch_palette() {
    let mut app = common::test_app();

    // Press space (arms palette), then Ctrl+q (modified).
    app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    app.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));

    assert!(
        !app.should_quit,
        "Ctrl+q after space must not trigger palette quit"
    );
    assert!(!app.pending_space);
}
