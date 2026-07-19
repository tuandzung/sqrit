mod common;

use ratatui::backend::TestBackend;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;
use sqrit::app::FocusedPane;
use sqrit::sql::statement_at_cursor;

fn render(sql: &str, cursor: (usize, usize)) -> (Terminal<TestBackend>, Color, Color) {
    let mut app = common::test_app();
    app.editor.insert_str(sql);
    let selected = statement_at_cursor(sql, cursor, sqrit::config::DbType::Sqlite)
        .unwrap()
        .unwrap();
    app.selected_statement = Some(selected);
    app.maximized = Some(FocusedPane::Query);
    let selection_bg = app.theme.selection_bg;
    let keyword = app.theme.keyword;
    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    (terminal, selection_bg, keyword)
}

#[test]
fn highlight_covers_only_the_selected_statement_on_one_line() {
    let (terminal, selection_bg, _) = render("SELECT 1; SELECT 2;", (0, 15));
    let buffer = terminal.backend().buffer();
    assert_ne!(buffer[(1, 1)].bg, selection_bg);
    assert_ne!(buffer[(10, 1)].bg, selection_bg);
    assert_eq!(buffer[(11, 1)].bg, selection_bg);
    assert_eq!(buffer[(19, 1)].bg, selection_bg);
}

#[test]
fn highlight_preserves_keyword_foreground() {
    let (terminal, selection_bg, keyword) = render("SELECT 1; SELECT 2;", (0, 15));
    let cell = &terminal.backend().buffer()[(11, 1)];
    assert_eq!(cell.bg, selection_bg);
    assert_eq!(cell.fg, keyword);
    assert!(cell.modifier.contains(Modifier::BOLD));
}

#[test]
fn highlight_spans_multiple_lines_without_coloring_neighbors() {
    let sql = "SELECT 1;\nSELECT\n  2;\nSELECT 3;";
    let (terminal, selection_bg, _) = render(sql, (1, 2));
    let buffer = terminal.backend().buffer();
    assert_ne!(buffer[(1, 1)].bg, selection_bg);
    assert_eq!(buffer[(1, 2)].bg, selection_bg);
    assert_eq!(buffer[(2, 3)].bg, selection_bg);
    assert_ne!(buffer[(1, 4)].bg, selection_bg);
}

#[test]
fn cursor_movement_clears_highlight() {
    let mut app = common::test_app();
    app.active_connection = Some("test".to_string());
    app.editor.insert_str("SELECT 1; SELECT 2;");
    app.handle_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('g'),
        crossterm::event::KeyModifiers::NONE,
    ));
    app.handle_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('s'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(app.selected_statement.is_some());
    app.handle_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('h'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(app.selected_statement.is_none());
}

#[test]
fn editing_selected_statement_clears_highlight_and_feedback() {
    let mut app = common::test_app();
    app.active_connection = Some("test".to_string());
    app.editor.insert_str("SELECT 1; SELECT 2;");
    for code in [
        crossterm::event::KeyCode::Char('g'),
        crossterm::event::KeyCode::Char('s'),
        crossterm::event::KeyCode::Char('i'),
        crossterm::event::KeyCode::Char('x'),
    ] {
        app.handle_key_event(crossterm::event::KeyEvent::new(
            code,
            crossterm::event::KeyModifiers::NONE,
        ));
    }
    assert!(app.selected_statement.is_none());
    assert!(app.status_message.is_empty());
}

#[test]
fn success_and_error_results_keep_the_selected_highlight() {
    let mut app = common::test_app();
    app.editor.insert_str("SELECT 1; SELECT 2;");
    app.selected_statement =
        statement_at_cursor(&app.editor.text(), (0, 15), sqrit::config::DbType::Sqlite).unwrap();
    app.query_id = 4;

    for status in [
        sqrit::app::QueryStatus::Success,
        sqrit::app::QueryStatus::Error("database error".into()),
    ] {
        app.status_message = "running statement 2/2".to_string();
        app.async_tx
            .send(sqrit::app::AsyncResult::QueryDone {
                query_id: 4,
                status,
                result: None,
                has_next_page: false,
            })
            .unwrap();
        app.drain_async_results();
        assert!(app.selected_statement.is_some());
        assert!(app.status_message.is_empty());
    }
}
