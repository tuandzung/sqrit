use sqrit::app::{App, FocusedPane};
use sqrit::config::{Config, Connection, DbType};
use sqrit::editor::EditorBuffer;
use sqrit::mode::Mode;
use sqrit::picker::PickerState;

fn make_config(names: &[&str]) -> Config {
    let connections = names
        .iter()
        .enumerate()
        .map(|(i, name)| Connection {
            name: name.to_string(),
            db_type: DbType::Sqlite,
            host: None,
            port: None,
            username: None,
            password: None,
            database: None,
            file_path: Some(format!("/tmp/test_{}.db", i)),
        })
        .collect();
    Config { connections }
}

fn make_app(names: &[&str]) -> App {
    App {
        mode: Mode::Picker,
        config: make_config(names),
        should_quit: false,
        picker: PickerState::new(),
        db: None,
        focused_pane: FocusedPane::Query,
        editor: EditorBuffer::new(),
        status_message: String::new(),
    }
}

// #1 empty config shows no connections
#[test]
fn picker_empty_config_no_filtered_connections() {
    let app = make_app(&[]);
    let indices = app.picker.filtered_indices(&app);
    assert!(indices.is_empty());
}

// #2 lists connections from config
#[test]
fn picker_lists_all_connections() {
    let app = make_app(&["dev", "staging", "prod"]);
    let indices = app.picker.filtered_indices(&app);
    assert_eq!(indices, vec![0, 1, 2]);
}

// #3 arrow keys move selection
#[test]
fn picker_arrow_keys_move_selection() {
    let mut app = make_app(&["dev", "staging", "prod"]);
    assert_eq!(app.picker.selected, 0);

    let count = app.picker.filtered_indices(&app).len();
    app.picker.move_down(count);
    assert_eq!(app.picker.selected, 1);

    let count = app.picker.filtered_indices(&app).len();
    app.picker.move_down(count);
    assert_eq!(app.picker.selected, 2);

    // can't go past end
    let count = app.picker.filtered_indices(&app).len();
    app.picker.move_down(count);
    assert_eq!(app.picker.selected, 2);

    app.picker.move_up();
    assert_eq!(app.picker.selected, 1);

    // can't go before start
    app.picker.move_up();
    app.picker.move_up();
    assert_eq!(app.picker.selected, 0);
}

// #4 type to filter connections by name
#[test]
fn picker_filter_by_name() {
    let mut app = make_app(&["dev-db", "staging-db", "prod-db"]);
    let count = app.picker.filtered_indices(&app).len();
    app.picker.type_char('p', count);
    let filtered = app.picker.filtered_indices(&app);
    assert_eq!(filtered, vec![2]); // only "prod-db"

    app.picker.backspace(app.picker.filtered_indices(&app).len());
    let filtered = app.picker.filtered_indices(&app);
    assert_eq!(filtered.len(), 3); // back to all
}

// #5 select connection transitions mode
#[test]
fn picker_select_transitions_mode() {
    let mut app = make_app(&["test-db"]);

    // Simulate Enter via handle_key
    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);

    assert_eq!(app.mode, Mode::QueryNormal);
    assert!(app.db.is_some());
}
