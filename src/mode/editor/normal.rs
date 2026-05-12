use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::Mode;

pub struct NormalState {
    pub pending_g: bool,
    pub yank_register: Option<String>,
}

impl NormalState {
    pub fn new() -> Self {
        Self {
            pending_g: false,
            yank_register: None,
        }
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
            _ => return,
        }
    }

    match key.code {
        // Execute query
        KeyCode::Enter => {
            app.pending_query = Some(app.editor.text());
        }

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

        _ => {}
    }
}
