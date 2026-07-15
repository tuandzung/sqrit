use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::history::{history_path_for, HistoryEntry, HistoryStore};
use crate::mode::{KeyBinding, Mode, ModeHandler};

pub struct HistoryPickerState {
    pub entries: Vec<HistoryEntry>,
    pub filter: String,
    pub selected: usize,
    pub origin: Mode,
}

impl HistoryPickerState {
    pub fn open(entries: Vec<HistoryEntry>, origin: Mode) -> Self {
        Self {
            entries,
            filter: String::new(),
            selected: 0,
            origin,
        }
    }

    pub fn visible(&self) -> Vec<&HistoryEntry> {
        if self.filter.is_empty() {
            return self.entries.iter().collect();
        }
        let needle = self.filter.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.sql.to_lowercase().contains(&needle))
            .collect()
    }

    fn move_down(&mut self) {
        let len = self.visible().len();
        if len > 0 && self.selected + 1 < len {
            self.selected += 1;
        }
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn current_sql(&self) -> Option<String> {
        self.visible().get(self.selected).map(|e| e.sql.clone())
    }
}

pub struct HistoryPickerHandler;

const BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: "Esc",
        action: "Close the picker without modifying the editor",
    },
    KeyBinding {
        key: "Up / Down",
        action: "Move selection through filtered set",
    },
    KeyBinding {
        key: "Enter",
        action: "Paste selected SQL into the editor (no auto-execute)",
    },
    KeyBinding {
        key: "Type",
        action: "Filter by substring on SQL",
    },
    KeyBinding {
        key: "Backspace",
        action: "Delete one character from the filter",
    },
];

impl ModeHandler for HistoryPickerHandler {
    fn dispatch(&self, key: KeyEvent, app: &mut App) {
        handle_key(key, app);
    }

    fn bindings(&self) -> &'static [KeyBinding] {
        BINDINGS
    }

    fn handle_paste(&self, text: &str, app: &mut App) {
        let Some(p) = app.history_picker.as_mut() else {
            return;
        };
        let first_line = text.split('\n').next().unwrap_or("").trim_end_matches('\r');
        if first_line.is_empty() {
            return;
        }
        p.filter.push_str(first_line);
        p.selected = 0;
    }
}

pub fn open(app: &mut App, origin: Mode) {
    let Some(conn) = app.active_connection.as_ref() else {
        app.status_message = "No active connection — no history to show".to_string();
        return;
    };
    let path = history_path_for(&app.sqrit_dir, conn);
    let entries = match HistoryStore::new(path).load() {
        Ok(mut e) => {
            e.reverse();
            e
        }
        Err(err) => {
            app.status_message = format!("history load failed: {}", err);
            return;
        }
    };
    if entries.is_empty() {
        app.status_message = "History is empty".to_string();
        return;
    }
    app.history_picker = Some(HistoryPickerState::open(entries, origin));
    app.mode = Mode::HistoryPicker;
}

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => close(app),
        KeyCode::Enter => paste(app),
        KeyCode::Down => {
            if let Some(p) = app.history_picker.as_mut() {
                p.move_down();
            }
        }
        KeyCode::Up => {
            if let Some(p) = app.history_picker.as_mut() {
                p.move_up();
            }
        }
        KeyCode::Backspace => {
            if let Some(p) = app.history_picker.as_mut() {
                p.filter.pop();
                p.selected = 0;
            }
        }
        KeyCode::Char(c) => {
            if let Some(p) = app.history_picker.as_mut() {
                p.filter.push(c);
                p.selected = 0;
            }
        }
        _ => {}
    }
}

fn close(app: &mut App) {
    if let Some(state) = app.history_picker.take() {
        app.mode = state.origin;
    }
}

fn paste(app: &mut App) {
    let Some(state) = app.history_picker.take() else {
        return;
    };
    if let Some(sql) = state.current_sql() {
        app.editor.replace_all(&sql);
    }
    app.mode = state.origin;
}
