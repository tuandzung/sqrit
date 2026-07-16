use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, QueryStatus};
use crate::mode::{KeyBinding, Mode, ModeHandler};

#[derive(Default)]
pub struct NormalState {
    pub pending_g: bool,
    pub yank_register: Option<String>,
}

impl NormalState {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct NormalHandler;

const BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: "i",
        action: "Enter Insert mode",
    },
    KeyBinding {
        key: "Enter",
        action: "Execute query",
    },
    KeyBinding {
        key: "gs",
        action: "Execute statement under cursor",
    },
    KeyBinding {
        key: "h / j / k / l",
        action: "Move cursor left / down / up / right",
    },
    KeyBinding {
        key: "w / b",
        action: "Word forward / backward",
    },
    KeyBinding {
        key: "0 / $",
        action: "Line start / end",
    },
    KeyBinding {
        key: "gg / G",
        action: "Top / bottom of buffer",
    },
    KeyBinding {
        key: "x",
        action: "Delete char at cursor",
    },
    KeyBinding {
        key: "dd",
        action: "Delete line",
    },
    KeyBinding {
        key: "yy",
        action: "Yank line",
    },
    KeyBinding {
        key: "p",
        action: "Paste below",
    },
    KeyBinding {
        key: "u",
        action: "Undo",
    },
    KeyBinding {
        key: "e / r",
        action: "Focus Explorer / Results pane",
    },
    KeyBinding {
        key: "<space>",
        action: "Open command palette",
    },
];

impl ModeHandler for NormalHandler {
    fn dispatch(&self, key: KeyEvent, app: &mut App) {
        handle_key(key, app);
    }

    fn bindings(&self) -> &'static [KeyBinding] {
        BINDINGS
    }
}

pub fn handle_key(key: KeyEvent, app: &mut App) {
    let state = &mut app.normal_state;

    // Handle pending 'g' prefix
    if state.pending_g {
        state.pending_g = false;
        match key.code {
            KeyCode::Char('g') => {
                app.editor.go_top();
                return;
            }
            KeyCode::Char('s') => {
                let backend = app
                    .active_connection
                    .as_ref()
                    .and_then(|name| app.config.get_connection(name))
                    .map(|connection| connection.db_type.clone());
                let Some(backend) = backend else {
                    app.query_status = QueryStatus::Error("No database connection".to_string());
                    return;
                };
                let text = app.editor.text();
                match crate::sql::statement_at_cursor(&text, app.editor.cursor(), backend) {
                    Ok(Some(statement)) => {
                        let query = text[statement.range.clone()].to_string();
                        app.status_message = format!(
                            "running statement {}/{}",
                            statement.ordinal, statement.total
                        );
                        app.queue_query(query, Some(statement));
                    }
                    Ok(None) => {
                        app.query_status = QueryStatus::Idle;
                        app.status_message = "no statement at cursor".to_string();
                    }
                    Err(error) => {
                        app.query_status = QueryStatus::Error(error.to_string());
                    }
                }
                return;
            }
            _ => return,
        }
    }

    match key.code {
        // Execute query
        KeyCode::Enter => app.queue_query(app.editor.text(), None),

        // Mode switch
        KeyCode::Char('i') => app.mode = Mode::QueryInsert,

        // Movement
        KeyCode::Char('h') => app.editor.cursor_left(),
        KeyCode::Char('j') => app.editor.cursor_down(),
        KeyCode::Char('k') => app.editor.cursor_up(),
        KeyCode::Char('l') => app.editor.cursor_right(),
        KeyCode::Char('w') => app.editor.word_forward(),
        KeyCode::Char('b') => app.editor.word_backward(),
        KeyCode::Char('0') => app.editor.home(),
        KeyCode::Char('$') => app.editor.end(),

        // Line operations
        KeyCode::Char('G') => app.editor.go_bottom(),
        KeyCode::Char('g') => {
            state.pending_g = true;
        }

        // Delete char at cursor
        KeyCode::Char('x') => app.editor.delete_char(),

        // Delete line (dd)
        KeyCode::Char('d') => {
            if let Some(line) = app.editor.delete_line() {
                state.yank_register = Some(line);
            }
        }

        // Yank line (yy)
        KeyCode::Char('y') => {
            state.yank_register = Some(app.editor.yank_line());
        }

        // Paste below (p)
        KeyCode::Char('p') => {
            if let Some(ref line) = state.yank_register {
                app.editor.paste_below(line);
            }
        }

        // Undo
        KeyCode::Char('u') => app.editor.undo(),

        // Pane focus
        KeyCode::Char('e') => app.switch_pane(Mode::Explorer, crate::app::FocusedPane::Explorer),
        KeyCode::Char('r') => app.switch_pane(Mode::Results, crate::app::FocusedPane::Results),

        // Space prefix
        KeyCode::Char(' ') => {
            app.pending_space = true;
        }

        _ => {}
    }
}
