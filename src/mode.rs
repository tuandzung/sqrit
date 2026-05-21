pub mod editor;
pub mod explorer;
pub mod help;
pub mod picker;
pub mod results;
pub mod theme_picker;

use crossterm::event::KeyEvent;

use crate::app::App;

/// One row in the help overlay: the key (or chord) the user presses,
/// and a one-line label of what it does.
#[derive(Debug, Clone, Copy)]
pub struct KeyBinding {
    pub key: &'static str,
    pub action: &'static str,
}

/// Per-mode key-handling contract. Each implementer owns BOTH the dispatch
/// (`dispatch`) and the human-facing description of its bindings
/// (`bindings`). Co-locating the two makes the help overlay impossible to
/// drift out of sync with the handler — adding a key without listing it is a
/// PR-visible mistake on a single line of source.
pub trait ModeHandler {
    fn dispatch(&self, key: KeyEvent, app: &mut App);
    fn bindings(&self) -> &'static [KeyBinding];
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Picker,
    Explorer,
    QueryNormal,
    QueryInsert,
    Results,
    ThemePicker,
    Help,
}

impl Mode {
    /// Return the trait object that owns this mode's dispatch + bindings.
    pub fn handler(&self) -> &'static dyn ModeHandler {
        match self {
            Mode::Picker => &picker::PickerHandler,
            Mode::Explorer => &explorer::ExplorerHandler,
            Mode::QueryNormal => &editor::normal::NormalHandler,
            Mode::QueryInsert => &editor::insert::InsertHandler,
            Mode::Results => &results::ResultsHandler,
            Mode::ThemePicker => &theme_picker::ThemePickerHandler,
            Mode::Help => &help::HelpHandler,
        }
    }

    pub fn handle_key(&self, key: KeyEvent, app: &mut App) {
        self.handler().dispatch(key, app);
    }

    pub fn label(&self) -> &'static str {
        match self {
            Mode::Picker => "PICKER",
            Mode::Explorer => "EXPLORER",
            Mode::QueryNormal => "NORMAL",
            Mode::QueryInsert => "INSERT",
            Mode::Results => "RESULTS",
            Mode::ThemePicker => "THEME",
            Mode::Help => "HELP",
        }
    }
}
