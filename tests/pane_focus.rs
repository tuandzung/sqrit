mod common;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use sqrit::app::{App, FocusedPane};
use sqrit::mode::Mode;

fn make_app() -> App {
    common::test_app()
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

// --- Slice 1: QueryNormal `e` → Explorer ---

#[test]
fn query_normal_e_switches_to_explorer() {
    let mut app = make_app();
    assert_eq!(app.mode, Mode::QueryNormal);
    assert_eq!(app.focused_pane, FocusedPane::Query);

    app.handle_key_event(key(KeyCode::Char('e')));

    assert_eq!(app.mode, Mode::Explorer);
    assert_eq!(app.focused_pane, FocusedPane::Explorer);
}

// --- Slice 2: QueryNormal `r` → Results ---

#[test]
fn query_normal_r_switches_to_results() {
    let mut app = make_app();

    app.handle_key_event(key(KeyCode::Char('r')));

    assert_eq!(app.mode, Mode::Results);
    assert_eq!(app.focused_pane, FocusedPane::Results);
}

// --- Slice 3: Explorer `r` → Results ---

#[test]
fn explorer_r_switches_to_results() {
    let mut app = make_app();
    app.mode = Mode::Explorer;
    app.focused_pane = FocusedPane::Explorer;

    app.handle_key_event(key(KeyCode::Char('r')));

    assert_eq!(app.mode, Mode::Results);
    assert_eq!(app.focused_pane, FocusedPane::Results);
}

// --- Slice 4: Results `e` → Explorer ---

#[test]
fn results_e_switches_to_explorer() {
    let mut app = make_app();
    app.mode = Mode::Results;
    app.focused_pane = FocusedPane::Results;

    app.handle_key_event(key(KeyCode::Char('e')));

    assert_eq!(app.mode, Mode::Explorer);
    assert_eq!(app.focused_pane, FocusedPane::Explorer);
}

// --- Slice 5: Existing `q` from Explorer/Results ---

#[test]
fn explorer_q_switches_to_query_and_focuses_query_pane() {
    let mut app = make_app();
    app.mode = Mode::Explorer;
    app.focused_pane = FocusedPane::Explorer;

    app.handle_key_event(key(KeyCode::Char('q')));

    assert_eq!(app.mode, Mode::QueryNormal);
    assert_eq!(app.focused_pane, FocusedPane::Query);
}

#[test]
fn results_q_switches_to_query_and_focuses_query_pane() {
    let mut app = make_app();
    app.mode = Mode::Results;
    app.focused_pane = FocusedPane::Results;

    app.handle_key_event(key(KeyCode::Char('q')));

    assert_eq!(app.mode, Mode::QueryNormal);
    assert_eq!(app.focused_pane, FocusedPane::Query);
}

// --- Slice 6: Insert mode does NOT switch pane ---

#[test]
fn insert_mode_e_types_character_not_switch_pane() {
    let mut app = make_app();
    app.mode = Mode::QueryInsert;

    app.handle_key_event(key(KeyCode::Char('e')));

    assert_eq!(app.mode, Mode::QueryInsert);
    assert_eq!(app.focused_pane, FocusedPane::Query);
    assert_eq!(app.editor.text(), "e");
}

#[test]
fn insert_mode_q_types_character_not_switch_pane() {
    let mut app = make_app();
    app.mode = Mode::QueryInsert;

    app.handle_key_event(key(KeyCode::Char('q')));

    assert_eq!(app.mode, Mode::QueryInsert);
    assert_eq!(app.editor.text(), "q");
}

#[test]
fn insert_mode_r_types_character_not_switch_pane() {
    let mut app = make_app();
    app.mode = Mode::QueryInsert;

    app.handle_key_event(key(KeyCode::Char('r')));

    assert_eq!(app.mode, Mode::QueryInsert);
    assert_eq!(app.focused_pane, FocusedPane::Query);
    assert_eq!(app.editor.text(), "r");
}
