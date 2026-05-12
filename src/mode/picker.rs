use crossterm::event::KeyEvent;

use crate::app::App;

pub fn handle_key(key: KeyEvent, app: &mut App) {
    if let crossterm::event::KeyCode::Char('q') = key.code {
        app.should_quit = true;
    }
}
