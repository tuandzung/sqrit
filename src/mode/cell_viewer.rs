use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::cell_viewer::{format_value, ViewMode};
use crate::clipboard;
use crate::db::types::Value;
use crate::mode::{KeyBinding, Mode, ModeHandler};

/// Transient state held while the cell viewer modal is open. Captures the
/// row/column the user pressed `v` on plus the column name (so we can look
/// up the live value) and the column's SQL data type (so the formatter can
/// apply type-aware rendering even after pagination scrolls the cell off
/// screen).
pub struct CellViewerState {
    pub origin: Mode,
    pub column: String,
    pub column_type: Option<String>,
    pub value: Value,
    pub view: ViewMode,
    pub scroll: u16,
}

impl CellViewerState {
    pub fn displayed(&self) -> String {
        format_value(&self.value, self.column_type.as_deref(), self.view)
    }
}

pub struct CellViewerHandler;

const BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: "Tab",
        action: "Toggle raw / formatted view",
    },
    KeyBinding {
        key: "y",
        action: "Copy the displayed text to the clipboard",
    },
    KeyBinding {
        key: "j / k",
        action: "Scroll down / up",
    },
    KeyBinding {
        key: "Esc",
        action: "Close the cell viewer",
    },
];

impl ModeHandler for CellViewerHandler {
    fn dispatch(&self, key: KeyEvent, app: &mut App) {
        handle_key(key, app);
    }

    fn bindings(&self) -> &'static [KeyBinding] {
        BINDINGS
    }
}

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => close(app),
        KeyCode::Tab => toggle_view(app),
        KeyCode::Char('y') => copy_displayed(app),
        KeyCode::Char('j') | KeyCode::Down => scroll_down(app),
        KeyCode::Char('k') | KeyCode::Up => scroll_up(app),
        _ => {}
    }
}

pub fn open(app: &mut App, origin: Mode) {
    let Some(result) = app.results.as_ref() else {
        return;
    };
    let row_idx = app.results_state.selected_row;
    let col_idx = app.results_state.selected_col;
    let Some(column) = result.columns.get(col_idx).cloned() else {
        return;
    };
    let Some(row) = result.rows.get(row_idx) else {
        return;
    };
    let value = row.get(&column).cloned().unwrap_or(Value::Null);
    app.cell_viewer = Some(CellViewerState {
        origin,
        column,
        column_type: None, // column-type lookup hooks in later; the formatter
        // is type-aware but defaults to text-only behavior.
        value,
        view: ViewMode::Raw,
        scroll: 0,
    });
    app.mode = Mode::CellViewer;
}

fn close(app: &mut App) {
    if let Some(state) = app.cell_viewer.take() {
        app.mode = state.origin;
    }
}

fn toggle_view(app: &mut App) {
    if let Some(state) = app.cell_viewer.as_mut() {
        state.view = match state.view {
            ViewMode::Raw => ViewMode::Formatted,
            ViewMode::Formatted => ViewMode::Raw,
        };
        state.scroll = 0;
    }
}

fn copy_displayed(app: &mut App) {
    let Some(state) = app.cell_viewer.as_ref() else {
        return;
    };
    let text = state.displayed();
    let _ = clipboard::copy_to_clipboard(&text);
    app.status_message = format!("Copied cell ({} chars)", text.chars().count());
}

fn scroll_down(app: &mut App) {
    if let Some(state) = app.cell_viewer.as_mut() {
        state.scroll = state.scroll.saturating_add(1);
    }
}

fn scroll_up(app: &mut App) {
    if let Some(state) = app.cell_viewer.as_mut() {
        state.scroll = state.scroll.saturating_sub(1);
    }
}
