use sqrit::app::{App, FocusedPane, QueryStatus};
use sqrit::mode::Mode;

fn make_app() -> App {
    App {
        mode: Mode::QueryNormal,
        config: sqrit::config::Config::default(),
        should_quit: false,
        picker: sqrit::picker::PickerState::new(),
        db: None,
        focused_pane: FocusedPane::Query,
        editor: sqrit::editor::EditorBuffer::new(),
        normal_state: sqrit::mode::editor::normal::NormalState::new(),
        status_message: String::new(),
        results: None,
        query_status: QueryStatus::Idle,
        pending_query: None,
        results_state: sqrit::results::ResultsState::new(),
        last_query: None,
        explorer_state: sqrit::explorer::ExplorerState::new(),
        pending_space: false,
        autocomplete: sqrit::autocomplete::AutocompleteState::new(),
        active_connection: None,
    }
}

#[test]
fn status_bar_shows_mode_label() {
    let mut app = make_app();

    app.mode = Mode::QueryNormal;
    assert!(app.status_bar_text().contains("NORMAL"));

    app.mode = Mode::QueryInsert;
    assert!(app.status_bar_text().contains("INSERT"));

    app.mode = Mode::Explorer;
    assert!(app.status_bar_text().contains("EXPLORER"));

    app.mode = Mode::Results;
    assert!(app.status_bar_text().contains("RESULTS"));
}

#[test]
fn status_bar_shows_active_connection_name() {
    let mut app = make_app();
    assert!(app.status_bar_text().contains("no connection"));

    app.active_connection = Some("mydb".to_string());
    assert!(app.status_bar_text().contains("mydb"));
    assert!(!app.status_bar_text().contains("no connection"));
}

#[test]
fn status_bar_shows_query_status_idle_when_idle() {
    let app = make_app();
    let text = app.status_bar_text();
    // Idle: empty status section after final pipe
    assert!(text.ends_with("| ") || text.ends_with("|"));
    assert!(!text.contains("running"));
    assert!(!text.contains("ERR"));
}

#[test]
fn status_bar_shows_running_when_query_running() {
    let mut app = make_app();
    app.query_status = QueryStatus::Running;
    assert!(app.status_bar_text().contains("running..."));
}

#[test]
fn status_bar_shows_ok_on_success() {
    let mut app = make_app();
    app.query_status = QueryStatus::Success;
    assert!(app.status_bar_text().contains("ok"));
}

#[test]
fn status_bar_shows_error_message() {
    let mut app = make_app();
    app.query_status = QueryStatus::Error("table not found".to_string());
    let text = app.status_bar_text();
    assert!(text.contains("ERR:"));
    assert!(text.contains("table not found"));
}
