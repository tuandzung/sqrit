use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, QueryStatus};
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
        action: "Expand or collapse the selected node",
    },
    KeyBinding {
        key: "s",
        action: "SELECT * FROM <ns>.<obj> LIMIT 100 (tables/views/matviews only)",
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
            use crate::db::quote::{quote_mysql, quote_pg, quote_sqlite};

            let items = app.explorer_state.items();
            if let Some(item) = items.get(app.explorer_state.selected) {
                let (namespace, kind, name) = match item {
                    TreeItem::Object { ns, kind, name, .. } => (ns, *kind, name),
                    TreeItem::Column {
                        ns, kind, parent, ..
                    } => (ns, *kind, parent),
                    _ => return,
                };
                if matches!(
                    &app.query_status,
                    QueryStatus::Success | QueryStatus::Error(_)
                ) {
                    app.query_status = QueryStatus::Idle;
                }
                app.status_message.clear();
                if !kind.supports_select_star() {
                    app.status_message = format!(
                        "no SELECT for {} (only tables/views/materialized views)",
                        kind.group_label().to_lowercase()
                    );
                    return;
                }
                let db_type = app
                    .active_connection
                    .as_ref()
                    .and_then(|name| app.config.get_connection(name))
                    .map(|connection| connection.db_type.clone());
                let qualified = match db_type {
                    Some(crate::config::DbType::Postgres) => {
                        if namespace.is_empty() {
                            quote_pg(name)
                        } else {
                            format!("{}.{}", quote_pg(namespace), quote_pg(name))
                        }
                    }
                    Some(crate::config::DbType::Mysql) => {
                        if namespace.is_empty() {
                            quote_mysql(name)
                        } else {
                            format!("{}.{}", quote_mysql(namespace), quote_mysql(name))
                        }
                    }
                    _ => quote_sqlite(name),
                };
                app.results_state.reset_pagination();
                app.pending_query = Some(format!("SELECT * FROM {qualified} LIMIT 100"));
                app.switch_pane(Mode::Results, crate::app::FocusedPane::Results);
            }
        }
        KeyCode::Enter => {
            let items = app.explorer_state.items();
            if let Some(item) = items.get(app.explorer_state.selected) {
                if let Some(key) = item.key() {
                    app.explorer_state.toggle_key(key);
                }
            }
        }
        KeyCode::Char(' ') => {
            app.pending_space = true;
        }
        _ => {}
    }
}
