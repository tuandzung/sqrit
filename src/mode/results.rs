use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::clipboard;
use crate::mode::Mode;

pub fn handle_key(key: KeyEvent, app: &mut App) {
    let total_rows = app.results.as_ref().map(|r| r.rows.len()).unwrap_or(0);
    let total_cols = app.results.as_ref().map(|r| r.columns.len()).unwrap_or(0);

    // Handle pending yank prefix
    if app.results_state.pending_yank {
        app.results_state.pending_yank = false;
        match key.code {
            KeyCode::Char('c') => {
                if let Some(ref result) = app.results {
                    if let Some(text) = clipboard::format_cell(result, app.results_state.selected_row, app.results_state.selected_col) {
                        let _ = clipboard::copy_to_clipboard(&text);
                        app.status_message = format!("Copied cell: {}", text);
                    }
                }
            }
            KeyCode::Char('y') => {
                if let Some(ref result) = app.results {
                    if let Some(text) = clipboard::format_row(result, app.results_state.selected_row) {
                        let _ = clipboard::copy_to_clipboard(&text);
                        app.status_message = format!("Copied row: {}", text);
                    }
                }
            }
            KeyCode::Char('a') => {
                if let Some(ref result) = app.results {
                    let text = clipboard::format_all(result);
                    let _ = clipboard::copy_to_clipboard(&text);
                    app.status_message = format!("Copied all ({} rows)", result.rows.len());
                }
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Char('q') => app.switch_pane(Mode::QueryNormal, crate::app::FocusedPane::Query),
        KeyCode::Char('e') => app.switch_pane(Mode::Explorer, crate::app::FocusedPane::Explorer),
        KeyCode::Char('r') => app.switch_pane(Mode::Results, crate::app::FocusedPane::Results),
        KeyCode::Char('j') | KeyCode::Down => app.results_state.move_down(total_rows),
        KeyCode::Char('k') | KeyCode::Up => app.results_state.move_up(),
        KeyCode::Char('h') | KeyCode::Left => app.results_state.move_left(),
        KeyCode::Char('l') | KeyCode::Right => app.results_state.move_right(total_cols),
        KeyCode::Char('y') => {
            app.results_state.pending_yank = true;
        }
        KeyCode::PageDown => {
            if app.results_state.has_next_page {
                app.results_state.page_down();
                if let Some(ref q) = app.last_query {
                    app.pending_query = Some(q.clone());
                }
            }
        }
        KeyCode::PageUp => {
            if app.results_state.page_offset > 0 {
                app.results_state.page_up();
                if let Some(ref q) = app.last_query {
                    app.pending_query = Some(q.clone());
                }
            }
        }
        _ => {}
    }
}
