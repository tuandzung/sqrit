use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::clipboard;
use crate::mode::{KeyBinding, Mode, ModeHandler};

pub struct ResultsHandler;

const BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: "/",
        action: "Fuzzy-filter loaded rows (live, all columns)",
    },
    KeyBinding {
        key: "h / j / k / l",
        action: "Move selection left / down / up / right",
    },
    KeyBinding {
        key: "v",
        action: "Open the selected cell in the viewer modal",
    },
    KeyBinding {
        key: "yc / yy / ya",
        action: "Copy cell / row / all to clipboard",
    },
    KeyBinding {
        key: "PgDn / PgUp",
        action: "Next / previous page",
    },
    KeyBinding {
        key: ",c",
        action: "Clear an active row filter",
    },
    KeyBinding {
        key: "q / e / r",
        action: "Focus Query / Explorer / Results pane",
    },
    KeyBinding {
        key: "<space>",
        action: "Open command palette",
    },
];

impl ModeHandler for ResultsHandler {
    fn dispatch(&self, key: KeyEvent, app: &mut App) {
        handle_key(key, app);
    }

    fn bindings(&self) -> &'static [KeyBinding] {
        BINDINGS
    }
}

pub fn handle_key(key: KeyEvent, app: &mut App) {
    let total_rows = app.results.as_ref().map(|r| r.rows.len()).unwrap_or(0);
    let total_cols = app.results.as_ref().map(|r| r.columns.len()).unwrap_or(0);

    // Handle pending comma prefix (e.g. `,c` clears a locked filter).
    if app.results_state.pending_comma {
        app.results_state.pending_comma = false;
        if let KeyCode::Char('c') = key.code {
            app.results_state.filter = None;
            app.results_state.filter_hits.clear();
            if let Some(result) = app.results.as_ref() {
                app.results_state.snap_selection_to_visible(result);
            }
        }
        return;
    }

    // Handle pending yank prefix
    if app.results_state.pending_yank {
        app.results_state.pending_yank = false;
        match key.code {
            KeyCode::Char('c') => {
                let payload = app.results.as_ref().and_then(|r| {
                    clipboard::format_cell(
                        r,
                        app.results_state.selected_row,
                        app.results_state.selected_col,
                    )
                });
                if let Some(text) = payload {
                    let _ = app.clipboard_writer.copy(&text);
                    app.status_message = format!("Copied cell: {}", text);
                }
            }
            KeyCode::Char('y') => {
                let payload = app
                    .results
                    .as_ref()
                    .and_then(|r| clipboard::format_row(r, app.results_state.selected_row));
                if let Some(text) = payload {
                    let _ = app.clipboard_writer.copy(&text);
                    app.status_message = format!("Copied row: {}", text);
                }
            }
            KeyCode::Char('a') => {
                let payload = app
                    .results
                    .as_ref()
                    .map(|r| (clipboard::format_all(r), r.rows.len()));
                if let Some((text, row_count)) = payload {
                    let _ = app.clipboard_writer.copy(&text);
                    app.status_message = format!("Copied all ({} rows)", row_count);
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
        KeyCode::Char('j') | KeyCode::Down => match app.results.as_ref() {
            Some(result) => app.results_state.move_down_visible(result),
            None => app.results_state.move_down(total_rows),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.results.as_ref() {
            Some(result) => app.results_state.move_up_visible(result),
            None => app.results_state.move_up(),
        },
        KeyCode::Char('h') | KeyCode::Left => app.results_state.move_left(),
        KeyCode::Char('l') | KeyCode::Right => app.results_state.move_right(total_cols),
        KeyCode::Char('y') => {
            app.results_state.pending_yank = true;
        }
        KeyCode::Char('v') => {
            crate::mode::cell_viewer::open(app, Mode::Results);
        }
        KeyCode::PageDown =>
        {
            #[allow(clippy::collapsible_match)]
            if app.results_state.has_next_page {
                app.results_state.page_down();
                if let Some(q) = app.last_query.clone() {
                    app.queue_query_page(q);
                }
            }
        }
        KeyCode::PageUp =>
        {
            #[allow(clippy::collapsible_match)]
            if app.results_state.page_offset > 0 {
                app.results_state.page_up();
                if let Some(q) = app.last_query.clone() {
                    app.queue_query_page(q);
                }
            }
        }
        KeyCode::Char(' ') => {
            app.pending_space = true;
        }
        KeyCode::Char('/') => {
            crate::mode::results_filter::open(app);
        }
        KeyCode::Char(',') => {
            app.results_state.pending_comma = true;
        }
        _ => {}
    }
}
