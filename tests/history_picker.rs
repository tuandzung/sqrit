mod common;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use sqrit::history::{history_path_for, HistoryEntry, HistoryStatus, HistoryStore};
use sqrit::mode::Mode;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[must_use = "drop the returned tempdir at end of test to clean up the history directory"]
fn seed_history(app: &mut sqrit::app::App, sqls: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    app.sqrit_dir = dir.path().to_path_buf();
    app.active_connection = Some("test".to_string());
    let store = HistoryStore::new(history_path_for(&app.sqrit_dir, "test"));
    for sql in sqls {
        store
            .append(&HistoryEntry {
                ts: "2026-05-21T08:13:02Z".into(),
                sql: sql.to_string(),
                duration_ms: 1,
                status: HistoryStatus::Ok,
                rows: Some(0),
            })
            .unwrap();
    }
    dir
}

#[test]
fn space_h_opens_history_picker_with_entries_newest_first() {
    let mut app = common::test_app();
    let _dir = seed_history(&mut app, &["SELECT 1", "SELECT 2", "SELECT 3"]);

    app.handle_key_event(key(KeyCode::Char(' ')));
    app.handle_key_event(key(KeyCode::Char('h')));

    assert_eq!(app.mode, Mode::HistoryPicker);
    let state = app.history_picker.as_ref().expect("picker state set");
    assert_eq!(
        state
            .entries
            .iter()
            .map(|e| e.sql.as_str())
            .collect::<Vec<_>>(),
        vec!["SELECT 3", "SELECT 2", "SELECT 1"],
        "newest-first ordering"
    );
    assert_eq!(state.selected, 0, "newest preselected");
}

#[test]
fn esc_closes_history_picker_without_modifying_editor() {
    let mut app = common::test_app();
    let _dir = seed_history(&mut app, &["SELECT 1"]);
    let before = app.editor.text();

    app.handle_key_event(key(KeyCode::Char(' ')));
    app.handle_key_event(key(KeyCode::Char('h')));
    app.handle_key_event(key(KeyCode::Esc));

    assert_eq!(app.mode, Mode::QueryNormal);
    assert!(app.history_picker.is_none());
    assert_eq!(app.editor.text(), before);
}

#[test]
fn enter_pastes_selected_sql_into_editor_and_closes() {
    let mut app = common::test_app();
    let _dir = seed_history(&mut app, &["SELECT old", "SELECT new"]);

    app.handle_key_event(key(KeyCode::Char(' ')));
    app.handle_key_event(key(KeyCode::Char('h')));
    app.handle_key_event(key(KeyCode::Enter));

    assert_eq!(app.mode, Mode::QueryNormal);
    assert!(app.history_picker.is_none());
    assert_eq!(app.editor.text(), "SELECT new");
    assert!(
        app.pending_query.is_none(),
        "history paste must NOT auto-execute"
    );
}

#[test]
fn typing_substring_filters_entries() {
    let mut app = common::test_app();
    let _dir = seed_history(
        &mut app,
        &[
            "SELECT * FROM users",
            "SELECT * FROM orders",
            "UPDATE users",
        ],
    );

    app.handle_key_event(key(KeyCode::Char(' ')));
    app.handle_key_event(key(KeyCode::Char('h')));
    app.handle_key_event(key(KeyCode::Char('u')));
    app.handle_key_event(key(KeyCode::Char('s')));
    app.handle_key_event(key(KeyCode::Char('e')));
    app.handle_key_event(key(KeyCode::Char('r')));
    app.handle_key_event(key(KeyCode::Char('s')));

    let state = app.history_picker.as_ref().unwrap();
    let visible: Vec<&str> = state.visible().iter().map(|e| e.sql.as_str()).collect();
    assert_eq!(visible, vec!["UPDATE users", "SELECT * FROM users"]);
}

#[test]
fn down_arrow_moves_selection_through_filtered_set() {
    let mut app = common::test_app();
    let _dir = seed_history(&mut app, &["SELECT 1", "SELECT 2", "SELECT 3"]);

    app.handle_key_event(key(KeyCode::Char(' ')));
    app.handle_key_event(key(KeyCode::Char('h')));
    app.handle_key_event(key(KeyCode::Down));

    let state = app.history_picker.as_ref().unwrap();
    assert_eq!(state.selected, 1);
}
