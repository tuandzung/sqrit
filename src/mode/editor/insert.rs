use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::Mode;

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => app.mode = Mode::QueryNormal,
        KeyCode::Char(c) => app.editor.insert_char(c),
        KeyCode::Backspace => app.editor.backspace(),
        KeyCode::Enter => app.editor.insert_newline(),
        KeyCode::Left => app.editor.cursor_left(),
        KeyCode::Right => app.editor.cursor_right(),
        KeyCode::Up => app.editor.cursor_up(),
        KeyCode::Down => app.editor.cursor_down(),
        KeyCode::Home => app.editor.home(),
        KeyCode::End => app.editor.end(),
        _ => {}
    }
}
