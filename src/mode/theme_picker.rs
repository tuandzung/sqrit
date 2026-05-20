use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::Mode;
use crate::theme::Theme;

/// Transient state held while the theme picker modal is open.
/// `original_theme` is captured at entry so Esc can revert when the user
/// previewed something then cancelled.
pub struct ThemePickerState {
    pub available: Vec<String>,
    pub selected: usize,
    pub original_theme: Theme,
    pub origin_mode: Mode,
}

impl ThemePickerState {
    pub fn open(available: Vec<String>, original_theme: Theme, origin_mode: Mode) -> Self {
        // Position the cursor on the current theme if it's in the list, else first entry.
        let selected = available
            .iter()
            .position(|n| n == &original_theme.name)
            .unwrap_or(0);
        Self {
            available,
            selected,
            original_theme,
            origin_mode,
        }
    }

    pub fn current_name(&self) -> Option<&str> {
        self.available.get(self.selected).map(|s| s.as_str())
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.available.len() {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
}

/// Open the picker from `origin`. Lists available themes from `app.themes_dir`,
/// captures the current `app.theme` as the revert target.
pub fn enter(app: &mut App, origin: Mode) {
    let mut available = crate::theme::list_available(&app.themes_dir);
    available.sort();
    let original = app.theme.clone();
    app.theme_picker = Some(ThemePickerState::open(available, original, origin));
    app.mode = Mode::ThemePicker;
}

pub fn handle_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(p) = app.theme_picker.as_mut() {
                p.move_down();
            }
            preview_current(app);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Some(p) = app.theme_picker.as_mut() {
                p.move_up();
            }
            preview_current(app);
        }
        KeyCode::Enter => apply(app),
        KeyCode::Esc => cancel(app),
        _ => {}
    }
}

fn cancel(app: &mut App) {
    let Some(picker) = app.theme_picker.take() else {
        return;
    };
    app.theme = picker.original_theme;
    app.mode = picker.origin_mode;
}

fn apply(app: &mut App) {
    let Some(picker) = app.theme_picker.take() else {
        return;
    };
    let Some(name) = picker.current_name().map(str::to_string) else {
        app.mode = picker.origin_mode;
        return;
    };
    let mut cfg = crate::config::AppConfig::load_from(&app.app_config_path).unwrap_or_default();
    match cfg.set_theme_at(&name, &app.app_config_path) {
        Ok(()) => {
            app.status_message = format!("theme '{}' saved", name);
        }
        Err(e) => {
            // Persistence failed; revert the previewed theme so the in-memory
            // state matches what's on disk. Surfacing the failure prevents the
            // "looks saved but isn't" trap.
            app.theme = picker.original_theme.clone();
            app.status_message = format!("failed to persist theme '{}': {}", name, e);
        }
    }
    app.mode = picker.origin_mode;
}

/// Load and apply the picker's currently selected theme to `app.theme`.
/// On load failure, `load_active` already returns `Theme::default_theme()`, so
/// `app.theme` becomes the safe fallback; we just forward its warning to the
/// status bar verbatim.
fn preview_current(app: &mut App) {
    let Some(name) = app
        .theme_picker
        .as_ref()
        .and_then(|p| p.current_name().map(str::to_string))
    else {
        return;
    };
    let (theme, warning) = crate::theme::load_active(&app.themes_dir, Some(&name));
    app.theme = theme;
    if let Some(w) = warning {
        app.status_message = w;
    }
}
