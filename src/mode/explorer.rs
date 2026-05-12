use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::explorer::TreeItem;
use crate::mode::Mode;

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('q') => app.mode = Mode::QueryNormal,
        KeyCode::Char('j') | KeyCode::Down => app.explorer_state.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.explorer_state.move_up(),
        KeyCode::Enter => {
            let items = app.explorer_state.items();
            if let Some(item) = items.get(app.explorer_state.selected) {
                match item {
                    TreeItem::Table { name, .. } => app.explorer_state.toggle(name),
                    TreeItem::View { name, .. } => app.explorer_state.toggle(name),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}
