mod common;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use sqrit::mode::Mode;

// Cycle 1: Insert mode's handle_paste places the entire pasted string
// (including newlines) into the editor buffer.
#[test]
fn insert_handle_paste_inserts_multi_line_text() {
    let mut app = common::test_app();
    app.mode = Mode::QueryInsert;
    Mode::QueryInsert
        .handler()
        .handle_paste("SELECT *\nFROM users\nWHERE id = 1", &mut app);

    assert_eq!(app.editor.text(), "SELECT *\nFROM users\nWHERE id = 1");
    // 3 lines of text — the editor should have advanced through newlines.
    let line_count = app.editor.text().lines().count();
    assert_eq!(line_count, 3);
}

#[test]
fn app_paste_path_clears_statement_highlight_and_feedback() {
    let mut app = common::test_app();
    app.mode = Mode::QueryInsert;
    app.editor.insert_str("SELECT 1; SELECT 2;");
    app.selected_statement =
        sqrit::sql::statement_at_cursor(&app.editor.text(), (0, 15), sqrit::config::DbType::Sqlite)
            .unwrap();
    app.status_message = "running statement 2/2".to_string();

    app.handle_paste_event(" -- edited");

    assert!(app.editor.text().ends_with(" -- edited"));
    assert!(app.selected_statement.is_none());
    assert!(app.status_message.is_empty());
}

// Cycle 2: Defensive fallback — Ctrl+J in Insert mode inserts a newline,
// not literal 'j'. Locks the bug at the key-event layer for terminals
// that don't support bracketed paste.
#[test]
fn insert_ctrl_j_inserts_newline_not_literal_j() {
    let mut app = common::test_app();
    app.mode = Mode::QueryInsert;
    Mode::QueryInsert.handle_key(
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
        &mut app,
    );
    assert_eq!(app.editor.text(), "\n");
}

// Cycle 3: Plain 'j' (no modifier) still inserts a literal 'j'. Guards
// against overcorrecting cycle 2.
#[test]
fn insert_plain_j_still_inserts_j() {
    let mut app = common::test_app();
    app.mode = Mode::QueryInsert;
    Mode::QueryInsert.handle_key(
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
        &mut app,
    );
    assert_eq!(app.editor.text(), "j");
}

// Cycle 4: Connection picker paste appends the first line of the pasted
// text to the filter; subsequent lines are dropped.
#[test]
fn picker_handle_paste_appends_first_line_to_filter() {
    let mut app = common::test_app();
    app.mode = Mode::Picker;
    Mode::Picker
        .handler()
        .handle_paste("prod\nstaging", &mut app);
    assert_eq!(app.picker.filter, "prod");
}
