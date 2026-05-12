use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::config::DbType;
use crate::db::sqlite::SqliteAdapter;
use crate::mode::Mode;

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
                    DbType::Postgres | DbType::Mysql => {
                        return;
                    }
                };
                app.db = Some(db);
                app.mode = Mode::QueryNormal;
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
