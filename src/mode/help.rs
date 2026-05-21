use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::{KeyBinding, Mode, ModeHandler};

/// Transient state held while the help overlay is open. `origin` is captured
/// at entry so Esc can return the user to the exact mode they came from.
pub struct HelpState {
    pub origin: Mode,
}

pub struct HelpHandler;

const BINDINGS: &[KeyBinding] = &[KeyBinding {
    key: "Esc / ?",
    action: "Close the help overlay",
}];

impl ModeHandler for HelpHandler {
    fn dispatch(&self, key: KeyEvent, app: &mut App) {
        handle_key(key, app);
    }

    fn bindings(&self) -> &'static [KeyBinding] {
        BINDINGS
    }
}

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') => close(app),
        _ => {}
    }
}

pub fn open(app: &mut App, origin: Mode) {
    app.help = Some(HelpState { origin });
    app.mode = Mode::Help;
}

pub fn close(app: &mut App) {
    if let Some(state) = app.help.take() {
        app.mode = state.origin;
    }
}
