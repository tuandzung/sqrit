use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::explorer::TreeItem;
use crate::mode::Mode;

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('q') => app.switch_pane(Mode::QueryNormal, crate::app::FocusedPane::Query),
        KeyCode::Char('r') => app.switch_pane(Mode::Results, crate::app::FocusedPane::Results),
        KeyCode::Char('e') => app.switch_pane(Mode::Explorer, crate::app::FocusedPane::Explorer),
        KeyCode::Char('j') | KeyCode::Down => app.explorer_state.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.explorer_state.move_up(),
        KeyCode::Char('s') => {
            let items = app.explorer_state.items();
            if let Some(item) = items.get(app.explorer_state.selected) {
                let name = match item {
                    TreeItem::Table { name, .. } => Some(name.clone()),
                    TreeItem::View { name, .. } => Some(name.clone()),
                    TreeItem::Column { table, .. } => Some(table.clone()),
                    TreeItem::ViewColumn { view, .. } => Some(view.clone()),
                };
                if let Some(name) = name {
                    app.pending_query = Some(format!("SELECT * FROM {} LIMIT 100", name));
                    app.switch_pane(Mode::Results, crate::app::FocusedPane::Results);
                }
            }
        }
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
