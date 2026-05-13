use crossterm::event::{KeyCode, KeyModifiers, KeyEvent};

use crate::app::App;
use crate::mode::Mode;

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.pending_query = Some(app.editor.text());
        }
        KeyCode::Tab => {
            if app.autocomplete.is_visible() {
                if let Some(word) = app.autocomplete.accept() {
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
        KeyCode::Char(c) => app.editor.insert_char(c),
        KeyCode::Backspace => app.editor.backspace(),
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
