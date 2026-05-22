use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::config::DbType;
use crate::db::mysql::MySqlAdapter;
use crate::db::postgres::PgAdapter;
use crate::db::sqlite::SqliteAdapter;
use crate::mode::{KeyBinding, Mode, ModeHandler};

pub struct PickerHandler;

const BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: "Up / Down",
        action: "Move selection",
    },
    KeyBinding {
        key: "Enter",
        action: "Connect to selected database",
    },
    KeyBinding {
        key: "a-z, 0-9, …",
        action: "Type into the filter",
    },
    KeyBinding {
        key: "Backspace",
        action: "Delete last filter character",
    },
    KeyBinding {
        key: "Esc",
        action: "Clear filter",
    },
    KeyBinding {
        key: "q",
        action: "Quit",
    },
];

impl ModeHandler for PickerHandler {
    fn dispatch(&self, key: KeyEvent, app: &mut App) {
        handle_key(key, app);
    }

    fn bindings(&self) -> &'static [KeyBinding] {
        BINDINGS
    }

    fn handle_paste(&self, text: &str, app: &mut App) {
        // Pickers are single-line filters — drop everything after the
        // first newline so multi-line clipboards don't smear into the
        // filter input. Push the whole first line in one shot and
        // recompute filtered_indices once, not per char (matters on
        // large connection lists).
        let first_line = text.split('\n').next().unwrap_or("");
        if first_line.is_empty() {
            return;
        }
        app.picker.filter.push_str(first_line);
        let count = app.picker.filtered_indices(app).len();
        app.picker.clamp_selected(count);
    }
}

fn build_url(
    scheme: &str,
    user: &str,
    password: &str,
    host: &str,
    port: u16,
    database: &str,
) -> String {
    let user = urlencoding::encode(user);
    let database = urlencoding::encode(database);
    if password.is_empty() {
        format!("{}://{}@{}:{}/{}", scheme, user, host, port, database)
    } else {
        let password = urlencoding::encode(password);
        format!(
            "{}://{}:{}@{}:{}/{}",
            scheme, user, password, host, port, database
        )
    }
}

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Up => {
            app.picker.move_up();
        }
        KeyCode::Down => {
            let count = app.picker.filtered_indices(app).len();
            app.picker.move_down(count);
        }
        KeyCode::Enter => {
            if let Some(idx) = app.picker.selected_connection(app) {
                let conn = &app.config.connections[idx];
                let db: Box<dyn crate::db::Database> = match conn.db_type {
                    DbType::Sqlite => {
                        let path = conn.file_path.clone().unwrap_or_default();
                        Box::new(SqliteAdapter::new(&path))
                    }
                    DbType::Postgres => {
                        let url = build_url(
                            "postgresql",
                            conn.username.as_deref().unwrap_or("postgres"),
                            conn.password.as_deref().unwrap_or(""),
                            conn.host.as_deref().unwrap_or("localhost"),
                            conn.port.unwrap_or(5432),
                            conn.database.as_deref().unwrap_or("postgres"),
                        );
                        Box::new(PgAdapter::new(&url))
                    }
                    DbType::Mysql => {
                        let url = build_url(
                            "mysql",
                            conn.username.as_deref().unwrap_or("root"),
                            conn.password.as_deref().unwrap_or(""),
                            conn.host.as_deref().unwrap_or("localhost"),
                            conn.port.unwrap_or(3306),
                            conn.database.as_deref().unwrap_or("mysql"),
                        );
                        Box::new(MySqlAdapter::new(&url))
                    }
                };
                app.db = Some(db);
                app.active_connection = Some(conn.name.clone());
                app.mode = Mode::QueryNormal;
                app.pending_schema_load = true;
            }
        }
        KeyCode::Backspace => {
            let count = app.picker.filtered_indices(app).len();
            app.picker.backspace(count);
        }
        KeyCode::Esc => {
            app.picker.clear_filter();
        }
        KeyCode::Char(c) => {
            let count = app.picker.filtered_indices(app).len();
            app.picker.type_char(c, count);
        }
        _ => {}
    }
}
