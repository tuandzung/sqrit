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

// --- Slice 9: '?' opens the help overlay from QueryNormal ---

#[test]
fn question_mark_opens_help_overlay_from_query_normal() {
    let mut app = common::test_app();
    assert_eq!(app.mode, Mode::QueryNormal);

    press(&mut app, &[KeyCode::Char('?')]);

    assert_eq!(app.mode, Mode::Help);
    assert!(app.help.is_some(), "help state must be set");
    assert_eq!(
        app.help.as_ref().unwrap().origin,
        Mode::QueryNormal,
        "help must remember the origin mode for Esc-revert"
    );
}

// --- Slice 9b: opens from Explorer + Results too ---

#[test]
fn question_mark_opens_help_from_explorer() {
    let mut app = common::test_app();
    app.mode = Mode::Explorer;

    press(&mut app, &[KeyCode::Char('?')]);

    assert_eq!(app.mode, Mode::Help);
    assert_eq!(app.help.as_ref().unwrap().origin, Mode::Explorer);
}

#[test]
fn question_mark_opens_help_from_results() {
    let mut app = common::test_app();
    app.mode = Mode::Results;

    press(&mut app, &[KeyCode::Char('?')]);

    assert_eq!(app.mode, Mode::Help);
    assert_eq!(app.help.as_ref().unwrap().origin, Mode::Results);
}

// --- Slice 10: Esc closes overlay and restores origin ---

#[test]
fn esc_closes_help_and_restores_origin_explorer() {
    let mut app = common::test_app();
    app.mode = Mode::Explorer;

    press(&mut app, &[KeyCode::Char('?'), KeyCode::Esc]);

    assert_eq!(app.mode, Mode::Explorer);
    assert!(app.help.is_none());
}

#[test]
fn question_mark_again_also_closes_help() {
    let mut app = common::test_app();

    press(&mut app, &[KeyCode::Char('?'), KeyCode::Char('?')]);

    assert_eq!(app.mode, Mode::QueryNormal);
    assert!(app.help.is_none());
}

// --- Slice 11: '?' is a literal char in QueryInsert; filters in Picker ---

#[test]
fn question_mark_in_query_insert_inserts_literal() {
    let mut app = common::test_app();
    app.mode = Mode::QueryInsert;

    press(&mut app, &[KeyCode::Char('?')]);

    assert_eq!(app.mode, Mode::QueryInsert, "must stay in Insert");
    assert!(app.help.is_none());
    assert_eq!(app.editor.text(), "?");
}

#[test]
fn question_mark_in_picker_types_into_filter() {
    let mut app = common::test_app();
    app.mode = Mode::Picker;

    press(&mut app, &[KeyCode::Char('?')]);

    assert_eq!(app.mode, Mode::Picker, "must stay in Picker");
    assert!(app.help.is_none());
    assert_eq!(app.picker.filter, "?");
}
