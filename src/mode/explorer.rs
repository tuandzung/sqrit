use crossterm::event::KeyEvent;

use crate::app::App;
use crate::mode::Mode;

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        crossterm::event::KeyCode::Char('q') => app.mode = Mode::QueryNormal,
        _ => {}
    }
}
