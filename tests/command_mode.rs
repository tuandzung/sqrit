mod common;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use sqrit::app::App;
use sqrit::mode::Mode;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn type_str(app: &mut App, s: &str) {
    for c in s.chars() {
        app.handle_key_event(key(KeyCode::Char(c)));
    }
}

// --- Slice 1: `:` from QueryNormal enters Command mode ---

#[test]
fn colon_from_query_normal_enters_command_mode() {
    let mut app = common::test_app();
    app.mode = Mode::QueryNormal;

    app.handle_key_event(key(KeyCode::Char(':')));

    assert_eq!(app.mode, Mode::Command);
    assert_eq!(app.command_buffer, "");
    assert_eq!(app.command_origin, Some(Mode::QueryNormal));
}

// --- Slice 2: `:q` + Enter sets should_quit ---

#[test]
fn colon_q_enter_quits() {
    let mut app = common::test_app();
    app.mode = Mode::QueryNormal;
    assert!(!app.should_quit);

    app.handle_key_event(key(KeyCode::Char(':')));
    type_str(&mut app, "q");
    app.handle_key_event(key(KeyCode::Enter));

    assert!(app.should_quit);
}

// --- Slice 3: `:quit` + Enter sets should_quit ---

#[test]
fn colon_quit_long_form_quits() {
    let mut app = common::test_app();
    app.mode = Mode::QueryNormal;

    app.handle_key_event(key(KeyCode::Char(':')));
    type_str(&mut app, "quit");
    app.handle_key_event(key(KeyCode::Enter));

    assert!(app.should_quit);
}

// --- Slice 4: `:q!` and `:quit!` also quit ---

#[test]
fn colon_q_bang_quits() {
    let mut app = common::test_app();
    app.mode = Mode::QueryNormal;

    app.handle_key_event(key(KeyCode::Char(':')));
    type_str(&mut app, "q!");
    app.handle_key_event(key(KeyCode::Enter));

    assert!(app.should_quit);
}

// --- Slice 5: Esc cancels command mode, restores origin ---

#[test]
fn esc_cancels_command_mode() {
    let mut app = common::test_app();
    app.mode = Mode::QueryNormal;
    app.handle_key_event(key(KeyCode::Char(':')));
    type_str(&mut app, "qu");

    app.handle_key_event(key(KeyCode::Esc));

    assert_eq!(app.mode, Mode::QueryNormal);
    assert_eq!(app.command_buffer, "");
    assert_eq!(app.command_origin, None);
    assert!(!app.should_quit);
}

// --- Slice 6: Backspace removes chars; empty buffer Backspace cancels ---

#[test]
fn backspace_pops_then_cancels() {
    let mut app = common::test_app();
    app.mode = Mode::QueryNormal;
    app.handle_key_event(key(KeyCode::Char(':')));
    type_str(&mut app, "qu");

    app.handle_key_event(key(KeyCode::Backspace));
    assert_eq!(app.command_buffer, "q");
    assert_eq!(app.mode, Mode::Command);

    app.handle_key_event(key(KeyCode::Backspace));
    assert_eq!(app.command_buffer, "");
    assert_eq!(app.mode, Mode::Command);

    // One more Backspace on empty buffer cancels
    app.handle_key_event(key(KeyCode::Backspace));
    assert_eq!(app.mode, Mode::QueryNormal);
}

// --- Slice 7: unknown command sets error, returns to origin ---

#[test]
fn unknown_command_returns_to_origin_with_error() {
    let mut app = common::test_app();
    app.mode = Mode::QueryNormal;
    app.handle_key_event(key(KeyCode::Char(':')));
    type_str(&mut app, "bogus");
    app.handle_key_event(key(KeyCode::Enter));

    assert_eq!(app.mode, Mode::QueryNormal);
    assert!(!app.should_quit);
    assert!(app.status_message.contains("Not a command"));
    assert!(app.status_message.contains("bogus"));
}

// --- Slice 8: `:` from Explorer enters command mode, origin = Explorer ---

#[test]
fn colon_from_explorer_quits() {
    let mut app = common::test_app();
    app.mode = Mode::Explorer;

    app.handle_key_event(key(KeyCode::Char(':')));
    assert_eq!(app.mode, Mode::Command);
    assert_eq!(app.command_origin, Some(Mode::Explorer));

    type_str(&mut app, "q");
    app.handle_key_event(key(KeyCode::Enter));
    assert!(app.should_quit);
}

// --- Slice 9: `:` from Results enters command mode, origin = Results ---

#[test]
fn colon_from_results_quits() {
    let mut app = common::test_app();
    app.mode = Mode::Results;

    app.handle_key_event(key(KeyCode::Char(':')));
    assert_eq!(app.mode, Mode::Command);
    assert_eq!(app.command_origin, Some(Mode::Results));

    type_str(&mut app, "q");
    app.handle_key_event(key(KeyCode::Enter));
    assert!(app.should_quit);
}

// --- Slice 10: `:` in QueryInsert mode is a literal char, not command entry ---

#[test]
fn colon_in_insert_mode_is_literal() {
    let mut app = common::test_app();
    app.mode = Mode::QueryInsert;

    app.handle_key_event(key(KeyCode::Char(':')));

    assert_eq!(app.mode, Mode::QueryInsert);
    assert_eq!(app.editor.text(), ":");
}

// --- Slice 11: status bar shows `:buffer` while in Command mode ---

#[test]
fn status_bar_shows_command_buffer() {
    let mut app = common::test_app();
    app.mode = Mode::QueryNormal;
    app.handle_key_event(key(KeyCode::Char(':')));
    type_str(&mut app, "qu");

    let status = app.status_bar_text();
    assert_eq!(status, ":qu");
}

// --- Slice 12: empty command (just `:` + Enter) cancels without error ---

#[test]
fn empty_command_cancels_silently() {
    let mut app = common::test_app();
    app.mode = Mode::QueryNormal;
    app.handle_key_event(key(KeyCode::Char(':')));
    app.handle_key_event(key(KeyCode::Enter));

    assert_eq!(app.mode, Mode::QueryNormal);
    assert!(!app.should_quit);
    assert!(!app.status_message.contains("Not a command"));
}
