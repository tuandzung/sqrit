use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::{KeyBinding, Mode, ModeHandler};

pub struct ResultsFilterHandler;

const BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: "Type",
        action: "Substring-filter loaded rows (case-insensitive, any column)",
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
}

pub fn open(app: &mut App) {
    app.results_state.filter = Some(String::new());
    app.mode = Mode::ResultsFilter;
}

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => {
            app.results_state.filter = None;
            app.mode = Mode::Results;
        }
        KeyCode::Enter => {
            if let Some(f) = app.results_state.filter.as_ref() {
                if f.is_empty() {
                    app.results_state.filter = None;
                }
            }
            app.mode = Mode::Results;
            if let Some(result) = app.results.as_ref() {
                let result = result.clone();
                app.results_state.snap_selection_to_visible(&result);
            }
        }
        KeyCode::Backspace => {
            if let Some(f) = app.results_state.filter.as_mut() {
                f.pop();
            }
            snap(app);
        }
        KeyCode::Char(c) => {
            if let Some(f) = app.results_state.filter.as_mut() {
                f.push(c);
            }
            snap(app);
        }
        _ => {}
    }
}

fn snap(app: &mut App) {
    if let Some(result) = app.results.as_ref() {
        let result = result.clone();
        app.results_state.snap_selection_to_visible(&result);
    }
}
