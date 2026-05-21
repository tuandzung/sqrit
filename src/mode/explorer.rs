use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::explorer::TreeItem;
use crate::mode::{KeyBinding, Mode, ModeHandler};

pub struct ExplorerHandler;

const BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: "j / k",
        action: "Move selection down / up",
    },
    KeyBinding {
        key: "Enter",
        action: "Expand or collapse the table / view",
    },
    KeyBinding {
        key: "s",
        action: "SELECT * FROM <table> LIMIT 100",
    },
    KeyBinding {
        key: "q / r / e",
        action: "Focus Query / Results / Explorer pane",
    },
    KeyBinding {
        key: "<space>",
        action: "Open command palette",
    },
];

impl ModeHandler for ExplorerHandler {
    fn dispatch(&self, key: KeyEvent, app: &mut App) {
        handle_key(key, app);
    }

    fn bindings(&self) -> &'static [KeyBinding] {
        BINDINGS
    }
}

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
        KeyCode::Char(' ') => {
            app.pending_space = true;
        }
        _ => {}
    }
}
