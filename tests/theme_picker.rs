mod common;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sqrit::config::AppConfig;
use sqrit::mode::theme_picker::ThemePickerState;
use sqrit::mode::Mode;
use sqrit::theme::{ensure_bundled, Theme};
use tempfile::tempdir;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn state_with(available: Vec<&str>) -> ThemePickerState {
    ThemePickerState::open(
        available.into_iter().map(String::from).collect(),
        Theme::default_theme(),
        Mode::QueryNormal,
    )
}

#[test]
fn move_down_advances_selection() {
    let mut s = state_with(vec!["a", "b", "c"]);
    s.selected = 0;
    s.move_down();
    assert_eq!(s.selected, 1);
}

#[test]
fn move_down_clamps_at_last_entry() {
    let mut s = state_with(vec!["a", "b", "c"]);
    s.selected = 2;
    s.move_down();
    assert_eq!(s.selected, 2);
}

#[test]
fn move_up_decreases_selection() {
    let mut s = state_with(vec!["a", "b", "c"]);
    s.selected = 2;
    s.move_up();
    assert_eq!(s.selected, 1);
}

#[test]
fn move_up_clamps_at_zero() {
    let mut s = state_with(vec!["a", "b", "c"]);
    s.selected = 0;
    s.move_up();
    assert_eq!(s.selected, 0);
}

#[test]
fn esc_restores_original_theme_and_closes_picker() {
    let tmp = tempdir().unwrap();
    ensure_bundled(tmp.path()).unwrap();
    let config_path = tmp.path().join("config.toml");

    let mut app = common::test_app();
    app.themes_dir = tmp.path().to_path_buf();
    app.app_config_path = config_path.clone();
    app.theme = Theme::parse(include_str!("../themes/nord.toml")).unwrap();
    app.mode = Mode::QueryNormal;

    app.handle_key_event(key(KeyCode::Char(' ')));
    app.handle_key_event(key(KeyCode::Char('t')));
    // Move around to preview a different theme
    app.handle_key_event(key(KeyCode::Char('j')));
    app.handle_key_event(key(KeyCode::Char('j')));
    let previewed = app.theme.name.clone();
    assert_ne!(previewed, "nord", "preview should have moved off nord");

    app.handle_key_event(key(KeyCode::Esc));

    assert_eq!(app.mode, Mode::QueryNormal, "should return to origin mode");
    assert!(app.theme_picker.is_none(), "picker closed");
    assert_eq!(app.theme.name, "nord", "Esc restored original theme");
    assert!(
        !config_path.exists(),
        "Esc should NOT persist anything to disk"
    );
}

#[test]
fn enter_persists_chosen_theme_and_closes_picker() {
    let tmp = tempdir().unwrap();
    ensure_bundled(tmp.path()).unwrap();
    let config_path = tmp.path().join("config.toml");

    let mut app = common::test_app();
    app.themes_dir = tmp.path().to_path_buf();
    app.app_config_path = config_path.clone();
    app.mode = Mode::QueryNormal;

    app.handle_key_event(key(KeyCode::Char(' ')));
    app.handle_key_event(key(KeyCode::Char('t')));
    app.handle_key_event(key(KeyCode::Char('j')));
    let chosen = app
        .theme_picker
        .as_ref()
        .unwrap()
        .current_name()
        .unwrap()
        .to_string();

    app.handle_key_event(key(KeyCode::Enter));

    assert_eq!(app.mode, Mode::QueryNormal, "should return to origin mode");
    assert!(app.theme_picker.is_none(), "picker closed");
    assert_eq!(app.theme.name, chosen, "selected theme stays active");
    let saved = AppConfig::load_from(&config_path).unwrap();
    assert_eq!(
        saved.theme.as_deref(),
        Some(chosen.as_str()),
        "theme name persisted to config.toml"
    );
}

#[test]
fn j_in_picker_advances_and_live_previews_theme() {
    let tmp = tempdir().unwrap();
    ensure_bundled(tmp.path()).unwrap();

    let mut app = common::test_app();
    app.themes_dir = tmp.path().to_path_buf();
    // Start with default; bundled themes alphabetically: catppuccin, gruvbox, nord, rose-pine, tokyo-night
    app.handle_key_event(key(KeyCode::Char(' ')));
    app.handle_key_event(key(KeyCode::Char('t')));
    let first_name = app
        .theme_picker
        .as_ref()
        .unwrap()
        .current_name()
        .unwrap()
        .to_string();
    let theme_before = app.theme.name.clone();

    app.handle_key_event(key(KeyCode::Char('j')));

    let picker = app.theme_picker.as_ref().expect("picker still open");
    let now_name = picker.current_name().unwrap();
    assert_ne!(
        now_name, first_name,
        "j should advance selection past initial entry"
    );
    assert_eq!(
        app.theme.name, now_name,
        "live preview: app.theme follows selection (was {})",
        theme_before
    );
}

#[test]
fn space_t_opens_theme_picker_and_captures_original() {
    let tmp = tempdir().unwrap();
    ensure_bundled(tmp.path()).unwrap();

    let mut app = common::test_app();
    app.themes_dir = tmp.path().to_path_buf();
    app.theme = Theme::parse(include_str!("../themes/nord.toml")).unwrap();

    app.handle_key_event(key(KeyCode::Char(' ')));
    app.handle_key_event(key(KeyCode::Char('t')));

    assert_eq!(app.mode, Mode::ThemePicker);
    let picker = app.theme_picker.as_ref().expect("picker should be open");
    assert_eq!(picker.original_theme.name, "nord");
    assert_eq!(picker.available.len(), 5, "should list 5 bundled themes");
}
