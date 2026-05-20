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
    restore_origin_or_default(app);
}

fn execute(app: &mut App) {
    let cmd = std::mem::take(&mut app.command_buffer);
    let trimmed = cmd.trim();
    match trimmed {
        "q" | "quit" | "q!" | "quit!" => {
            app.should_quit = true;
            app.command_origin = None;
        }
        "" => cancel(app),
        other => {
            app.status_message = format!("Not a command: {}", other);
            restore_origin_or_default(app);
        }
    }
}

/// Restore the mode that was active when command mode was entered.
/// Falls back to `Mode::QueryNormal` if no origin was recorded — should
/// never happen in practice, since `enter()` always sets one.
fn restore_origin_or_default(app: &mut App) {
    app.mode = app.command_origin.take().unwrap_or(Mode::QueryNormal);
}

/// Enter command mode from `origin`. Used by `:` keybinding in non-insert modes.
/// Asserts no prior origin is recorded — command mode is intentionally single-level;
/// `:` from within command mode would be a literal char, so a non-None origin here
/// signals a missing cleanup path.
pub fn enter(app: &mut App, origin: Mode) {
    debug_assert!(
        app.command_origin.is_none(),
        "command::enter called with existing origin {:?}",
        app.command_origin,
    );
    app.command_origin = Some(origin);
    app.command_buffer.clear();
    app.mode = Mode::Command;
}
