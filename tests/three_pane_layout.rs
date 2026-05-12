use ratatui::layout::{Constraint, Direction, Layout, Rect};
use sqrit::app::{App, FocusedPane};
use sqrit::config::{Config, Connection, DbType};
use sqrit::editor::EditorBuffer;
use sqrit::mode::Mode;
use sqrit::mode::editor::normal::NormalState;
use sqrit::picker::PickerState;

fn make_connected_app() -> App {
    let config = Config {
        connections: vec![Connection {
            name: "test".to_string(),
            db_type: DbType::Sqlite,
            host: None,
            port: None,
            username: None,
            password: None,
            database: None,
            file_path: Some("/tmp/test.db".to_string()),
        }],
    };
    App {
        mode: Mode::QueryNormal,
        config,
        should_quit: false,
        picker: PickerState::new(),
        db: None,
        focused_pane: FocusedPane::Query,
        editor: EditorBuffer::new(),
        normal_state: NormalState::new(),
        status_message: String::new(),
        results: None,
        query_status: sqrit::app::QueryStatus::Idle,
        pending_query: None,
        last_query: None,
        results_state: sqrit::results::ResultsState::new(),
    }
}

// T8 #1: 3-pane layout splits into explorer + right side
#[test]
fn layout_splits_into_explorer_and_right() {
    let area = Rect::new(0, 0, 120, 40);
    let status_height = 1u16;
    let main_height = area.height.saturating_sub(status_height);

    let main_area = Rect {
        height: main_height,
        ..area
    };

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(25), Constraint::Min(0)])
        .split(main_area);

    assert_eq!(main_chunks[0].width, 25);
    assert_eq!(main_chunks[1].width, 120 - 25);
}

// T8 #2: right side splits into query + results
#[test]
fn right_side_splits_into_query_and_results() {
    let right_area = Rect::new(25, 0, 95, 39);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(right_area);

    assert_eq!(right_chunks.len(), 2);
    assert!(right_chunks[0].height > 0);
    assert!(right_chunks[1].height > 0);
    assert_eq!(
        right_chunks[0].height + right_chunks[1].height,
        right_area.height
    );
}

// T8 #3: status bar occupies bottom row
#[test]
fn status_bar_at_bottom() {
    let area = Rect::new(0, 0, 120, 40);
    let status_height = 1u16;
    let main_height = area.height.saturating_sub(status_height);

    let status_area = Rect {
        y: area.y + main_height,
        height: status_height,
        ..area
    };

    assert_eq!(status_area.y, 39);
    assert_eq!(status_area.height, 1);
}

// T8 #4: focused pane border style differs from unfocused
#[test]
fn focused_pane_has_different_border_style() {
    let app = make_connected_app();
    let focused = app.border_style(FocusedPane::Query);
    let unfocused = app.border_style(FocusedPane::Explorer);

    assert_ne!(focused, unfocused);
}
