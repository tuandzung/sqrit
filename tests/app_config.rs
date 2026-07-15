use std::fs;

use sqrit::app::App;
use sqrit::config::AppConfig;
use tempfile::tempdir;

#[test]
fn load_from_returns_empty_when_file_missing() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("does-not-exist.toml");

    let cfg = AppConfig::load_from(&path).unwrap();

    assert!(
        cfg.theme.is_none(),
        "missing file should produce empty config, got {:?}",
        cfg.theme
    );
}

#[test]
fn save_to_roundtrips_through_disk() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let cfg = AppConfig {
        theme: Some("gruvbox".to_string()),
        ..Default::default()
    };

    cfg.save_to(&path).unwrap();
    let reloaded = AppConfig::load_from(&path).unwrap();

    assert_eq!(reloaded.theme.as_deref(), Some("gruvbox"));
}

#[test]
fn set_theme_persists_to_disk() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut cfg = AppConfig::default();

    cfg.set_theme_at("rose-pine", &path).unwrap();
    let reloaded = AppConfig::load_from(&path).unwrap();

    assert_eq!(reloaded.theme.as_deref(), Some("rose-pine"));
    assert_eq!(
        cfg.theme.as_deref(),
        Some("rose-pine"),
        "in-memory state also updated"
    );
}

#[test]
fn load_from_returns_theme_name_when_present() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "theme = \"nord\"\n").unwrap();

    let cfg = AppConfig::load_from(&path).unwrap();

    assert_eq!(cfg.theme.as_deref(), Some("nord"));
}

#[test]
fn load_without_hint_bar_uses_defaults() {
    let toml_str = r#"theme = "tokyo-night""#;
    let cfg: AppConfig = toml::from_str(toml_str).unwrap();

    assert!(cfg.hint_bar.enabled);
    assert!(!cfg.hint_bar.auto_hide_narrow);
}

#[test]
fn load_with_partial_hint_bar_section() {
    let toml_str = r#"
theme = "nord"
[hint_bar]
enabled = false
"#;
    let cfg: AppConfig = toml::from_str(toml_str).unwrap();

    assert!(!cfg.hint_bar.enabled);
    assert!(!cfg.hint_bar.auto_hide_narrow);
}

#[test]
fn app_retains_loaded_hint_bar_config() {
    let dir = tempdir().unwrap();
    let sqrit_dir = dir.path().join(".sqrit");
    fs::create_dir(&sqrit_dir).unwrap();
    fs::write(
        sqrit_dir.join("config.toml"),
        "[hint_bar]\nenabled = false\n",
    )
    .unwrap();

    let original_home = std::env::var_os("HOME");
    std::env::set_var("HOME", dir.path());
    let app = App::new();
    match original_home {
        Some(home) => std::env::set_var("HOME", home),
        None => std::env::remove_var("HOME"),
    }

    assert!(!app.unwrap().app_config.hint_bar.enabled);
}
