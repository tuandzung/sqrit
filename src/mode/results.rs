use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::Mode;

pub fn handle_key(key: KeyEvent, app: &mut App) {
    let total_rows = app.results.as_ref().map(|r| r.rows.len()).unwrap_or(0);
    let total_cols = app.results.as_ref().map(|r| r.columns.len()).unwrap_or(0);

    match key.code {
        KeyCode::Char('q') => app.mode = Mode::QueryNormal,
        KeyCode::Char('j') | KeyCode::Down => app.results_state.move_down(total_rows),
        KeyCode::Char('k') | KeyCode::Up => app.results_state.move_up(),
        KeyCode::Char('h') | KeyCode::Left => app.results_state.move_left(),
        KeyCode::Char('l') | KeyCode::Right => app.results_state.move_right(total_cols),
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
