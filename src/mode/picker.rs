use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::config::DbType;
use crate::db::mysql::MySqlAdapter;
use crate::db::postgres::PgAdapter;
use crate::db::sqlite::SqliteAdapter;
use crate::mode::Mode;

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
