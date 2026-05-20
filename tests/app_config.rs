use std::fs;

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
