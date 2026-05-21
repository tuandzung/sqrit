mod common;

use std::collections::HashMap;

use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use sqrit::app::App;
use sqrit::cell_viewer::ViewMode;
use sqrit::db::types::{QueryResult, Value};
use sqrit::mode::cell_viewer::CellViewerState;
use sqrit::mode::Mode;

fn buffer_contains(terminal: &Terminal<TestBackend>, needle: &str) -> bool {
    let buffer = terminal.backend().buffer();
    for y in 0..buffer.area.height {
        let mut row = String::new();
        for x in 0..buffer.area.width {
            row.push_str(buffer[(x, y)].symbol());
        }
        if row.contains(needle) {
            return true;
        }
    }
    false
}

fn open_cell(app: &mut App, column: &str, value: Value, view: ViewMode) {
    let mut row: HashMap<String, Value> = HashMap::new();
    row.insert(column.to_string(), value.clone());
    app.results = Some(QueryResult {
        columns: vec![column.to_string()],
        rows: vec![row],
        rows_affected: None,
        total_count: None,
    });
    app.cell_viewer = Some(CellViewerState {
        origin: Mode::Results,
        column: column.to_string(),
        column_type: None,
        value,
        view,
        scroll: 0,
    });
    app.mode = Mode::CellViewer;
}

#[test]
fn cell_viewer_renders_text_content_in_raw_view() {
    let mut app = common::test_app();
    open_cell(
        &mut app,
        "note",
        Value::Text("hello world".to_string()),
        ViewMode::Raw,
    );

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();

    assert!(buffer_contains(&terminal, "hello world"));
    assert!(buffer_contains(&terminal, "Cell"));
    assert!(buffer_contains(&terminal, "note"));
}

#[test]
fn cell_viewer_renders_json_pretty_in_formatted_view() {
    let mut app = common::test_app();
    open_cell(
        &mut app,
        "payload",
        Value::Text(r#"{"a":1}"#.to_string()),
        ViewMode::Formatted,
    );

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();

    // Pretty-printed key + value should appear on the rendered row
    assert!(buffer_contains(&terminal, "\"a\""));
    assert!(buffer_contains(&terminal, "1"));
}

#[test]
fn cell_viewer_modal_rect_is_about_60pct_wide() {
    let area = Rect::new(0, 0, 80, 24);
    let modal = App::cell_viewer_modal_rect(area, 12);

    // 60% of 80 = 48 — modal should be near that for a generous title.
    assert!(modal.width >= 40 && modal.width <= 60);
    assert!(modal.height >= 10);
    assert!(modal.x + modal.width <= area.width);
    assert!(modal.y + modal.height <= area.height);
}

#[test]
fn cell_viewer_does_not_panic_on_tiny_terminal() {
    let mut app = common::test_app();
    open_cell(&mut app, "x", Value::Text("yo".to_string()), ViewMode::Raw);
    let backend = TestBackend::new(10, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
}
