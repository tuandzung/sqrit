use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::Mode;

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => cancel(app),
        KeyCode::Enter => execute(app),
        KeyCode::Backspace if app.command_buffer.pop().is_none() => cancel(app),
        KeyCode::Backspace => {}
        KeyCode::Char(c) => app.command_buffer.push(c),
        _ => {}
    }
}

fn cancel(app: &mut App) {
    app.command_buffer.clear();
    if let Some(origin) = app.command_origin.take() {
        app.mode = origin;
    } else {
        app.mode = Mode::QueryNormal;
    }
}

fn execute(app: &mut App) {
    let cmd = std::mem::take(&mut app.command_buffer);
    let trimmed = cmd.trim();
    match trimmed {
        "q" | "quit" | "q!" | "quit!" => {
            app.should_quit = true;
            app.command_origin = None;
        }
        "" => {
            cancel(app);
        }
        other => {
            app.status_message = format!("Not a command: {}", other);
            if let Some(origin) = app.command_origin.take() {
                app.mode = origin;
            } else {
                app.mode = Mode::QueryNormal;
            }
        }
    }
}

/// Enter command mode from `origin`. Used by `:` keybinding in non-insert modes.
pub fn enter(app: &mut App, origin: Mode) {
    app.command_origin = Some(origin);
    app.command_buffer.clear();
    app.mode = Mode::Command;
}
