use crossterm::event::KeyEvent;

use crate::app::App;
use crate::mode::Mode;

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        crossterm::event::KeyCode::Char('i') => app.mode = Mode::QueryInsert,
        crossterm::event::KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}
