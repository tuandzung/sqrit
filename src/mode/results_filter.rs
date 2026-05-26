use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::{KeyBinding, Mode, ModeHandler};

pub struct ResultsFilterHandler;

const BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: "Type",
        action: "Fuzzy-filter loaded rows (subsequence, any column)",
    },
    KeyBinding {
        key: "Backspace",
        action: "Delete one character from the filter",
    },
    KeyBinding {
        key: "Enter",
        action: "Lock the filter and return to Results navigation",
    },
    KeyBinding {
        key: "Esc",
        action: "Cancel and clear the filter",
    },
];

impl ModeHandler for ResultsFilterHandler {
    fn dispatch(&self, key: KeyEvent, app: &mut App) {
        handle_key(key, app);
    }

    fn bindings(&self) -> &'static [KeyBinding] {
        BINDINGS
    }

    fn handle_paste(&self, text: &str, app: &mut App) {
        let first_line = text.split('\n').next().unwrap_or("").trim_end_matches('\r');
        if first_line.is_empty() {
            return;
        }
        if let Some(f) = app.results_state.filter.as_mut() {
            f.push_str(first_line);
        }
        recompute(app);
    }
}

pub fn open(app: &mut App) {
    app.results_state.filter = Some(String::new());
    recompute(app);
    app.mode = Mode::ResultsFilter;
}

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => {
            app.results_state.filter = None;
            app.results_state.filter_hits.clear();
            app.mode = Mode::Results;
        }
        KeyCode::Enter => {
            if let Some(f) = app.results_state.filter.as_ref() {
                if f.is_empty() {
                    app.results_state.filter = None;
                }
            }
            app.mode = Mode::Results;
            recompute(app);
        }
        KeyCode::Backspace => {
            if let Some(f) = app.results_state.filter.as_mut() {
                f.pop();
            }
            recompute(app);
        }
        KeyCode::Char(c) => {
            if let Some(f) = app.results_state.filter.as_mut() {
                f.push(c);
            }
            recompute(app);
        }
        _ => {}
    }
}

fn recompute(app: &mut App) {
    if let Some(result) = app.results.as_ref() {
        let query = app.results_state.filter.as_deref().unwrap_or("");
        app.results_state.filter_hits = app.fuzzy_filter.rank(result, query);
        app.results_state.snap_selection_to_visible(result);
    }
}
