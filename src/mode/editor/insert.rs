use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::mode::{KeyBinding, Mode, ModeHandler};

pub struct InsertHandler;

const BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: "Ctrl+Enter",
        action: "Execute the current query",
    },
    KeyBinding {
        key: "Esc",
        action: "Dismiss autocomplete or return to Normal mode",
    },
    KeyBinding {
        key: "Tab",
        action: "Accept autocomplete suggestion",
    },
    KeyBinding {
        key: "<any char>",
        action: "Insert literal character",
    },
    KeyBinding {
        key: "Backspace",
        action: "Delete previous character",
    },
    KeyBinding {
        key: "Enter",
        action: "Insert newline",
    },
    KeyBinding {
        key: "Arrows / Home / End",
        action: "Move cursor",
    },
];

impl ModeHandler for InsertHandler {
    fn dispatch(&self, key: KeyEvent, app: &mut App) {
        handle_key(key, app);
    }

    fn bindings(&self) -> &'static [KeyBinding] {
        BINDINGS
    }

    fn handle_paste(&self, text: &str, app: &mut App) {
        app.editor.insert_str(text);
        update_autocomplete(app);
    }
}

fn update_autocomplete(app: &mut App) {
    app.last_keystroke = Some(Instant::now());
    if app.autocomplete.is_visible() {
        let text = app.editor.text();
        let (row, col) = app.editor.cursor();
        let prefix = crate::autocomplete::current_word_prefix(&text, row, col);
        app.autocomplete.filter(&prefix);
    }
}

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.queue_query(app.editor.text(), None);
        }
        KeyCode::Tab =>
        {
            #[allow(clippy::collapsible_match)]
            if app.autocomplete.is_visible() {
                if let Some(word) = app.autocomplete.accept() {
                    let text = app.editor.text();
                    let (row, col) = app.editor.cursor();
                    let prefix_len =
                        crate::autocomplete::current_word_prefix(&text, row, col).len();
                    app.editor.delete_backwards(prefix_len);
                    app.editor.insert_str(&word);
                }
            }
        }
        KeyCode::Esc => {
            if app.autocomplete.is_visible() {
                app.autocomplete.dismiss();
            } else {
                app.mode = Mode::QueryNormal;
            }
        }
        // Defensive: terminals lacking bracketed paste (older `screen`,
        // raw serial consoles, etc.) deliver pasted LF (0x0A) as Ctrl+J
        // because Ctrl+J == LF in ASCII. Treat that exactly as Enter so
        // multi-line pastes survive even on the fallback path.
        KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.editor.insert_newline();
            update_autocomplete(app);
        }
        KeyCode::Char(c) => {
            app.editor.insert_char(c);
            update_autocomplete(app);
        }
        KeyCode::Backspace => {
            app.editor.backspace();
            update_autocomplete(app);
        }
        KeyCode::Enter => app.editor.insert_newline(),
        KeyCode::Left => app.editor.cursor_left(),
        KeyCode::Right => app.editor.cursor_right(),
        KeyCode::Up => {
            if app.autocomplete.is_visible() {
                app.autocomplete.prev();
            } else {
                app.editor.cursor_up();
            }
        }
        KeyCode::Down => {
            if app.autocomplete.is_visible() {
                app.autocomplete.next();
            } else {
                app.editor.cursor_down();
            }
        }
        KeyCode::Home => app.editor.home(),
        KeyCode::End => app.editor.end(),
        _ => {}
    }
}
