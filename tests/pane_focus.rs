use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use sqrit::app::{App, FocusedPane};
use sqrit::mode::Mode;

fn make_app() -> App {
    let (async_tx, async_rx) = tokio::sync::mpsc::unbounded_channel();
    App {
        mode: Mode::QueryNormal,
        should_quit: false,
        config: sqrit::config::Config { connections: vec![] },
        picker: sqrit::picker::PickerState::new(),
        db: None,
        focused_pane: FocusedPane::Query,
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
    }
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
