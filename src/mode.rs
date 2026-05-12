pub mod picker;
pub mod editor;
pub mod explorer;
pub mod results;

use crossterm::event::KeyEvent;

use crate::app::App;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Picker,
    Explorer,
    QueryNormal,
    QueryInsert,
    Results,
}

impl Mode {
    pub fn handle_key(&self, key: KeyEvent, app: &mut App) {
        match self {
            Mode::Picker => picker::handle_key(key, app),
            Mode::QueryNormal => editor::normal::handle_key(key, app),
            Mode::QueryInsert => editor::insert::handle_key(key, app),
            Mode::Explorer => explorer::handle_key(key, app),
            Mode::Results => results::handle_key(key, app),
        }
    }
}
