use std::fs;

use ratatui::style::Color;
use sqrit::theme::{ensure_bundled, list_available, load_active, Theme};
use tempfile::tempdir;

const TOKYO_NIGHT_TOML: &str = r##"
name = "tokyo-night"

[colors]
bg = "#1a1b26"
fg = "#c0caf5"
border_focused = "#7aa2f7"
border_unfocused = "#414868"
selection_bg = "#283457"
keyword = "#bb9af7"
string = "#9ece6a"
comment = "#565f89"
number = "#ff9e64"
type = "#7dcfff"
error = "#f7768e"
"##;

#[test]
fn list_available_returns_theme_names_from_dir() {
    let dir = tempdir().unwrap();
    ensure_bundled(dir.path()).unwrap();
    // User-supplied theme in addition to the bundled ones
    fs::write(
        dir.path().join("custom.toml"),
        include_str!("../themes/nord.toml"),
    )
    .unwrap();
    // Non-toml file should be ignored
    fs::write(dir.path().join("README.md"), "not a theme").unwrap();

    let mut names = list_available(dir.path());
    names.sort();

    let expected = vec![
        "catppuccin-macchiato",
        "custom",
        "gruvbox",
        "nord",
        "rose-pine",
        "tokyo-night",
    ];
    assert_eq!(names, expected, "unexpected theme list: {:?}", names);
}

#[test]
fn list_available_empty_dir_returns_empty_vec() {
    let dir = tempdir().unwrap();
    let names = list_available(dir.path());
    assert!(names.is_empty(), "expected empty list, got {:?}", names);
}

#[test]
fn list_available_missing_dir_returns_empty_vec() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("does-not-exist");
    let names = list_available(&missing);
    assert!(names.is_empty(), "expected empty list, got {:?}", names);
}

#[test]
fn load_active_returns_named_theme() {
    let dir = tempdir().unwrap();
    ensure_bundled(dir.path()).unwrap();

    let (theme, warning) = load_active(dir.path(), Some("nord"));

    assert_eq!(theme.name, "nord");
    assert!(warning.is_none(), "expected no warning, got {:?}", warning);
}

#[test]
fn load_active_no_selection_returns_default_silently() {
    let dir = tempdir().unwrap();

    let (theme, warning) = load_active(dir.path(), None);

    assert_eq!(theme.name, Theme::default_theme().name);
    assert!(
        warning.is_none(),
        "no selection should not warn, got {:?}",
        warning
    );
}

#[test]
fn load_active_missing_theme_returns_default_with_warning() {
    let dir = tempdir().unwrap();
    // Themes dir empty; user selected a name that doesn't exist
    let (theme, warning) = load_active(dir.path(), Some("does-not-exist"));

    assert_eq!(theme.name, Theme::default_theme().name);
    let w = warning.expect("missing theme should produce a warning");
    assert!(
        w.contains("does-not-exist"),
        "warning should name the missing theme, got: {w}"
    );
}

#[test]
fn load_active_malformed_theme_returns_default_with_warning() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("broken.toml"), "not valid = = toml").unwrap();

    let (theme, warning) = load_active(dir.path(), Some("broken"));

    assert_eq!(theme.name, Theme::default_theme().name);
    let w = warning.expect("malformed theme should produce a warning");
    assert!(
        w.contains("broken"),
        "warning should name the broken theme, got: {w}"
    );
}

#[test]
fn ensure_bundled_writes_all_when_dir_empty() {
    let dir = tempdir().unwrap();
    let themes_dir = dir.path();

    ensure_bundled(themes_dir).expect("ensure_bundled should succeed");

    for (filename, _) in sqrit::theme::BUNDLED {
        let path = themes_dir.join(filename);
        assert!(path.exists(), "expected {} to be written", filename);
        let contents = fs::read_to_string(&path).unwrap();
        assert!(
            contents.contains("[colors]"),
            "{} written but missing [colors] section",
            filename
        );
    }
}

#[test]
fn ensure_bundled_preserves_existing_files() {
    let dir = tempdir().unwrap();
    let themes_dir = dir.path();
    let nord_path = themes_dir.join("nord.toml");

    // Pre-existing user-edited file
    fs::write(&nord_path, "# user customised\nname = \"my-nord\"\n").unwrap();

    ensure_bundled(themes_dir).expect("ensure_bundled should succeed");

    let preserved = fs::read_to_string(&nord_path).unwrap();
    assert!(
        preserved.contains("my-nord"),
        "existing nord.toml was overwritten: {preserved}"
    );
    // Other bundled themes still written
    assert!(themes_dir.join("rose-pine.toml").exists());
    assert!(themes_dir.join("tokyo-night.toml").exists());
}

#[test]
fn all_bundled_defaults_parse() {
    let bundled = sqrit::theme::BUNDLED;
    assert_eq!(
        bundled.len(),
        5,
        "expected 5 bundled themes, got {}",
        bundled.len()
    );

    let expected_names: Vec<&str> = vec![
        "rose-pine.toml",
        "tokyo-night.toml",
        "nord.toml",
        "gruvbox.toml",
        "catppuccin-macchiato.toml",
    ];
    for name in &expected_names {
        assert!(
            bundled.iter().any(|(f, _)| f == name),
            "missing bundled theme file: {}",
            name
        );
    }
    for (filename, toml) in bundled {
        let theme = Theme::parse(toml)
            .unwrap_or_else(|e| panic!("bundled theme {} failed to parse: {}", filename, e));
        assert!(
            !theme.name.is_empty(),
            "bundled theme {} has empty name",
            filename
        );
    }
}

#[test]
fn bundled_themes_define_hint_bar_palettes() {
    let expected = [
        ("rose-pine.toml", [0x26233a, 0x908caa, 0xc4a7e7, 0x403d52]),
        ("tokyo-night.toml", [0x1a1b26, 0x9aa5ce, 0x7aa2f7, 0x414868]),
        ("nord.toml", [0x2e3440, 0xd8dee9, 0x88c0d0, 0x4c566a]),
        ("gruvbox.toml", [0x282828, 0xbdae93, 0xfabd2f, 0x504945]),
        (
            "catppuccin-macchiato.toml",
            [0x1e2030, 0xa5adcb, 0x8aadf4, 0x363a4f],
        ),
    ];
    let rgb = |hex: u32| Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8);

    for (filename, expected_palette) in expected {
        let toml = sqrit::theme::BUNDLED
            .iter()
            .find_map(|(name, toml)| (*name == filename).then_some(*toml))
            .unwrap_or_else(|| panic!("missing bundled theme: {filename}"));
        let theme = Theme::parse(toml).unwrap();

        assert_eq!(
            (
                theme.hint_bar_bg,
                theme.hint_bar_fg,
                theme.hint_bar_key,
                theme.hint_bar_separator,
            ),
            (
                rgb(expected_palette[0]),
                rgb(expected_palette[1]),
                rgb(expected_palette[2]),
                rgb(expected_palette[3]),
            ),
            "wrong hint bar palette for {filename}"
        );
    }
}

#[test]
fn parse_missing_required_field_errors() {
    // Omit `error` field
    let toml = r##"
name = "broken"

[colors]
bg = "#000000"
fg = "#ffffff"
border_focused = "#000000"
border_unfocused = "#000000"
selection_bg = "#000000"
keyword = "#000000"
string = "#000000"
comment = "#000000"
number = "#000000"
type = "#000000"
"##;
    let err = Theme::parse(toml).expect_err("missing field should error");
    let msg = err.to_string();
    assert!(
        msg.contains("error"),
        "error message should mention the missing field, got: {msg}"
    );
}

#[test]
fn parse_malformed_toml_errors() {
    let err = Theme::parse("this is not toml = =").expect_err("malformed TOML should error");
    let msg = err.to_string();
    assert!(
        msg.contains("TOML") || msg.contains("toml") || msg.contains("invalid"),
        "should report TOML parse error, got: {msg}"
    );
}

#[test]
fn parse_invalid_hex_errors() {
    let toml = r##"
name = "bad-hex"

[colors]
bg = "not-a-color"
fg = "#ffffff"
border_focused = "#000000"
border_unfocused = "#000000"
selection_bg = "#000000"
keyword = "#000000"
string = "#000000"
comment = "#000000"
number = "#000000"
type = "#000000"
error = "#000000"
"##;
    let err = Theme::parse(toml).expect_err("bad hex should error");
    let msg = err.to_string();
    assert!(
        msg.contains("hex") || msg.contains("not-a-color"),
        "should mention hex problem, got: {msg}"
    );
}

#[test]
fn parse_valid_toml_yields_all_fields() {
    let theme = Theme::parse(TOKYO_NIGHT_TOML).expect("valid TOML should parse");

    assert_eq!(theme.name, "tokyo-night");
    assert_eq!(theme.bg, Color::Rgb(0x1a, 0x1b, 0x26));
    assert_eq!(theme.fg, Color::Rgb(0xc0, 0xca, 0xf5));
    assert_eq!(theme.border_focused, Color::Rgb(0x7a, 0xa2, 0xf7));
    assert_eq!(theme.border_unfocused, Color::Rgb(0x41, 0x48, 0x68));
    assert_eq!(theme.selection_bg, Color::Rgb(0x28, 0x34, 0x57));
    assert_eq!(theme.keyword, Color::Rgb(0xbb, 0x9a, 0xf7));
    assert_eq!(theme.string, Color::Rgb(0x9e, 0xce, 0x6a));
    assert_eq!(theme.comment, Color::Rgb(0x56, 0x5f, 0x89));
    assert_eq!(theme.number, Color::Rgb(0xff, 0x9e, 0x64));
    assert_eq!(theme.type_, Color::Rgb(0x7d, 0xcf, 0xff));
    assert_eq!(theme.error, Color::Rgb(0xf7, 0x76, 0x8e));
}

#[test]
fn theme_without_hint_bar_section_falls_back_to_palette() {
    let toml_str = r##"
name = "minimal"
[colors]
bg = "#000000"
fg = "#ffffff"
border_focused = "#aaaaaa"
border_unfocused = "#444444"
selection_bg = "#222222"
keyword = "#ff00ff"
string = "#00ff00"
comment = "#666666"
number = "#0000ff"
type = "#00ffff"
error = "#ff0000"
"##;
    let t = Theme::parse(toml_str).unwrap();
    assert_eq!(t.hint_bar_bg, t.bg);
    assert_eq!(t.hint_bar_fg, t.fg);
    assert_eq!(t.hint_bar_key, t.border_focused);
    assert_eq!(t.hint_bar_separator, t.border_unfocused);
}

#[test]
fn theme_with_partial_hint_bar_section_overrides_only_present_fields() {
    let toml_str = r##"
name = "partial"
[colors]
bg = "#000000"
fg = "#ffffff"
border_focused = "#aaaaaa"
border_unfocused = "#444444"
selection_bg = "#222222"
keyword = "#ff00ff"
string = "#00ff00"
comment = "#666666"
number = "#0000ff"
type = "#00ffff"
error = "#ff0000"
hint_bar_key = "#abcdef"
"##;
    let t = Theme::parse(toml_str).unwrap();
    assert_eq!(t.hint_bar_key, Color::Rgb(0xab, 0xcd, 0xef));
    assert_eq!(t.hint_bar_bg, t.bg);
}
