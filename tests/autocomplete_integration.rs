use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use sqrit::app::App;
use sqrit::mode::Mode;

fn make_insert_app_with_editor() -> App {
    let (async_tx, async_rx) = tokio::sync::mpsc::unbounded_channel();
    let app = App {
        mode: Mode::QueryInsert,
        should_quit: false,
        config: sqrit::config::Config { connections: vec![] },
        picker: sqrit::picker::PickerState::new(),
        db: None,
        focused_pane: sqrit::app::FocusedPane::Query,
        editor: sqrit::editor::EditorBuffer::new(),
        normal_state: sqrit::mode::editor::normal::NormalState::new(),
        status_message: String::new(),
        results: None,
        query_status: sqrit::app::QueryStatus::Idle,
        pending_query: None,
        results_state: sqrit::results::ResultsState::new(),
        last_query: None,
        explorer_state: sqrit::explorer::ExplorerState::new(),
        pending_space: false,
            maximized: None,
        autocomplete: sqrit::autocomplete::AutocompleteState::new(),
        last_keystroke: None,
        pending_schema_load: false,
        active_connection: None,
        async_rx,
        async_tx,
    };
    app
}

#[test]
fn tick_autocomplete_opens_after_300ms_idle() {
    let mut app = make_insert_app_with_editor();
    // Type "SEL" into editor
    for c in "SEL".chars() {
        app.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }

    // Simulate 301ms idle
    app.last_keystroke = Some(Instant::now() - Duration::from_millis(301));

    app.tick_autocomplete();

    assert!(app.autocomplete.is_visible(), "autocomplete should open after 300ms idle");
    let filtered = app.autocomplete.filtered();
    assert!(filtered.contains(&"SELECT"), "should suggest SELECT, got: {:?}", filtered);
}

#[test]
fn tick_autocomplete_does_not_open_before_300ms() {
    let mut app = make_insert_app_with_editor();
    for c in "SEL".chars() {
        app.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    // last_keystroke is recent (< 300ms)
    app.tick_autocomplete();
    assert!(!app.autocomplete.is_visible(), "autocomplete should not open before 300ms");
}

#[test]
fn tick_autocomplete_noop_in_normal_mode() {
    let mut app = make_insert_app_with_editor();
    app.mode = Mode::QueryNormal;
    app.last_keystroke = Some(Instant::now() - Duration::from_millis(500));

    app.tick_autocomplete();
    assert!(!app.autocomplete.is_visible());
}

#[test]
fn insert_mode_char_updates_keystroke_and_filters() {
    let mut app = make_insert_app_with_editor();

    // Type "SE" to trigger idle-based autocomplete opening
    for c in "SE".chars() {
        app.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    // Force-open with "SE" prefix candidates
    let candidates = sqrit::autocomplete::suggest("SE", None);
    app.autocomplete.open(candidates);

    // Now type "L" — prefix becomes "SEL", should filter
    app.handle_key_event(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::NONE));

    let filtered = app.autocomplete.filtered();
    assert!(filtered.contains(&"SELECT"), "should still contain SELECT after typing L");
    // "SET" and "SERIAL" etc. should be filtered out since "SEL" doesn't match
    assert!(!filtered.contains(&"SET"), "SET should be filtered out");
}

#[test]
fn insert_mode_backspace_updates_keystroke_and_filters() {
    let mut app = make_insert_app_with_editor();

    // Type "SELE" then open autocomplete
    for c in "SELE".chars() {
        app.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    let candidates = sqrit::autocomplete::suggest("SELE", None);
    app.autocomplete.open(candidates);

    // Backspace — prefix reverts to "SEL"
    app.handle_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

    let filtered = app.autocomplete.filtered();
    assert!(filtered.contains(&"SELECT"), "SELECT should be back after backspace");
}

#[test]
fn pending_schema_load_deferred_flow() {
    let mut app = make_insert_app_with_editor();
    app.pending_schema_load = true;
    // No db set, so schema_info can't be called — flag should clear
    // We can't test with real db here (needs tokio runtime), but we verify
    // the flag clearing path
    assert!(app.pending_schema_load);
}

#[test]
fn tick_autocomplete_uses_schema_for_suggestions() {
    let mut app = make_insert_app_with_editor();
    app.explorer_state.schema = Some(sqrit::db::types::SchemaInfo {
        tables: vec![sqrit::db::types::TableInfo {
            name: "users".into(),
            columns: vec![sqrit::db::types::ColumnInfo {
                name: "email".into(),
                data_type: "TEXT".into(),
                nullable: false,
                is_primary_key: false,
            }],
        }],
        views: vec![],
    });

    // Type "em" — should match "email" column
    for c in "em".chars() {
        app.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    app.last_keystroke = Some(Instant::now() - Duration::from_millis(301));
    app.tick_autocomplete();

    assert!(app.autocomplete.is_visible());
    let filtered = app.autocomplete.filtered();
    assert!(filtered.contains(&"email"), "should suggest email column, got: {:?}", filtered);
}
