use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use sqrit::app::{App, FocusedPane};
use sqrit::mode::Mode;

fn make_app() -> App {
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
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn press_space_then_f(app: &mut App) {
    app.handle_key_event(key(KeyCode::Char(' ')));
    app.handle_key_event(key(KeyCode::Char('f')));
}

// --- Slice 1: space+f toggles maximize on from QueryNormal ---

#[test]
fn space_f_maximizes_focused_pane() {
    let mut app = make_app();
    assert_eq!(app.maximized, None);

    press_space_then_f(&mut app);

    assert_eq!(app.maximized, Some(FocusedPane::Query));
}

// --- Slice 2: space+f toggles back off ---

#[test]
fn space_f_toggles_maximize_off() {
    let mut app = make_app();
    press_space_then_f(&mut app);
    assert_eq!(app.maximized, Some(FocusedPane::Query));

    press_space_then_f(&mut app);

    assert_eq!(app.maximized, None);
}

// --- Slice 3: works from Explorer mode ---

#[test]
fn space_f_maximizes_from_explorer_mode() {
    let mut app = make_app();
    app.mode = Mode::Explorer;
    app.focused_pane = FocusedPane::Explorer;

    press_space_then_f(&mut app);

    assert_eq!(app.maximized, Some(FocusedPane::Explorer));
}

// --- Slice 4: works from Results mode ---

#[test]
fn space_f_maximizes_from_results_mode() {
    let mut app = make_app();
    app.mode = Mode::Results;
    app.focused_pane = FocusedPane::Results;

    press_space_then_f(&mut app);

    assert_eq!(app.maximized, Some(FocusedPane::Results));
}

// --- Slice 5: focus preserved across toggle ---

#[test]
fn focus_preserved_across_maximize_toggle() {
    let mut app = make_app();
    app.focused_pane = FocusedPane::Explorer;

    press_space_then_f(&mut app);
    assert_eq!(app.focused_pane, FocusedPane::Explorer);

    // Switch focus while maximized
    app.focused_pane = FocusedPane::Query;

    // Toggle off
    press_space_then_f(&mut app);
    assert_eq!(app.maximized, None);
    assert_eq!(app.focused_pane, FocusedPane::Query);
}

// --- Slice 6: space alone does not maximize ---

#[test]
fn space_alone_does_not_maximize() {
    let mut app = make_app();
    app.handle_key_event(key(KeyCode::Char(' ')));
    assert_eq!(app.maximized, None);
    assert!(app.pending_space);
}

// --- Slice 7: maximized tracks current focused pane, not stale ---

#[test]
fn maximize_captures_current_focused_pane() {
    let mut app = make_app();
    app.focused_pane = FocusedPane::Results;
    press_space_then_f(&mut app);
    assert_eq!(app.maximized, Some(FocusedPane::Results));
}
