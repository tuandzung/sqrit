use ratatui::style::Color;
use serde::Deserialize;

/// Bundled default themes embedded at compile time. Tuple is `(filename, toml_content)`.
/// Written to `~/.sqrit/themes/` on first run by `ensure_bundled`.
pub const BUNDLED: &[(&str, &str)] = &[
    ("rose-pine.toml", include_str!("../themes/rose-pine.toml")),
    (
        "tokyo-night.toml",
        include_str!("../themes/tokyo-night.toml"),
    ),
    ("nord.toml", include_str!("../themes/nord.toml")),
    ("gruvbox.toml", include_str!("../themes/gruvbox.toml")),
    (
        "catppuccin-macchiato.toml",
        include_str!("../themes/catppuccin-macchiato.toml"),
    ),
];

#[derive(Debug)]
pub enum ThemeError {
    Toml(toml::de::Error),
    InvalidHex(String),
}

impl std::fmt::Display for ThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeError::Toml(e) => write!(f, "invalid TOML: {}", e),
            ThemeError::InvalidHex(s) => write!(f, "invalid hex color {:?} (expected #rrggbb)", s),
        }
    }
}

impl std::error::Error for ThemeError {}

impl From<toml::de::Error> for ThemeError {
    fn from(e: toml::de::Error) -> Self {
        ThemeError::Toml(e)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub bg: Color,
    pub fg: Color,
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub selection_bg: Color,
    pub keyword: Color,
    pub string: Color,
    pub comment: Color,
    pub number: Color,
    pub type_: Color,
    pub error: Color,
    pub hint_bar_bg: Color,
    pub hint_bar_fg: Color,
    pub hint_bar_key: Color,
    pub hint_bar_separator: Color,
}

#[derive(Deserialize)]
struct RawTheme {
    name: String,
    colors: RawColors,
}

#[derive(Deserialize)]
struct RawColors {
    bg: String,
    fg: String,
    border_focused: String,
    border_unfocused: String,
    selection_bg: String,
    keyword: String,
    string: String,
    comment: String,
    number: String,
    #[serde(rename = "type")]
    type_: String,
    error: String,
    #[serde(default)]
    hint_bar_bg: Option<String>,
    #[serde(default)]
    hint_bar_fg: Option<String>,
    #[serde(default)]
    hint_bar_key: Option<String>,
    #[serde(default)]
    hint_bar_separator: Option<String>,
}

impl Theme {
    /// Hardcoded fallback used when the active theme can't be loaded.
    /// Independent of any file on disk — sqrit must always be able to render.
    pub fn default_theme() -> Self {
        Theme {
            name: "default".to_string(),
            bg: Color::Rgb(0x1a, 0x1b, 0x26),
            fg: Color::Rgb(0xc0, 0xca, 0xf5),
            border_focused: Color::Rgb(0x7a, 0xa2, 0xf7),
            border_unfocused: Color::Rgb(0x41, 0x48, 0x68),
            selection_bg: Color::Rgb(0x28, 0x34, 0x57),
            keyword: Color::Rgb(0xbb, 0x9a, 0xf7),
            string: Color::Rgb(0x9e, 0xce, 0x6a),
            comment: Color::Rgb(0x56, 0x5f, 0x89),
            number: Color::Rgb(0xff, 0x9e, 0x64),
            type_: Color::Rgb(0x7d, 0xcf, 0xff),
            error: Color::Rgb(0xf7, 0x76, 0x8e),
            hint_bar_bg: Color::Rgb(0x1a, 0x1b, 0x26),
            hint_bar_fg: Color::Rgb(0xc0, 0xca, 0xf5),
            hint_bar_key: Color::Rgb(0x7a, 0xa2, 0xf7),
            hint_bar_separator: Color::Rgb(0x41, 0x48, 0x68),
        }
    }

    pub fn parse(toml_str: &str) -> Result<Self, ThemeError> {
        let raw: RawTheme = toml::from_str(toml_str)?;
        let c = raw.colors;
        let bg = parse_hex(&c.bg)?;
        let fg = parse_hex(&c.fg)?;
        let border_focused = parse_hex(&c.border_focused)?;
        let border_unfocused = parse_hex(&c.border_unfocused)?;
        let hint_bar_bg = parse_hex_opt(&c.hint_bar_bg)?.unwrap_or(bg);
        let hint_bar_fg = parse_hex_opt(&c.hint_bar_fg)?.unwrap_or(fg);
        let hint_bar_key = parse_hex_opt(&c.hint_bar_key)?.unwrap_or(border_focused);
        let hint_bar_separator = parse_hex_opt(&c.hint_bar_separator)?.unwrap_or(border_unfocused);
        Ok(Theme {
            name: raw.name,
            bg,
            fg,
            border_focused,
            border_unfocused,
            selection_bg: parse_hex(&c.selection_bg)?,
            keyword: parse_hex(&c.keyword)?,
            string: parse_hex(&c.string)?,
            comment: parse_hex(&c.comment)?,
            number: parse_hex(&c.number)?,
            type_: parse_hex(&c.type_)?,
            error: parse_hex(&c.error)?,
            hint_bar_bg,
            hint_bar_fg,
            hint_bar_key,
            hint_bar_separator,
        })
    }
}

/// List theme names (filename without `.toml` extension) available in `themes_dir`.
/// Returns an empty vec if the directory doesn't exist or contains no `.toml` files.
/// Order is OS-dependent — callers that need a stable order must sort.
pub fn list_available(themes_dir: &std::path::Path) -> Vec<String> {
    let entries = match std::fs::read_dir(themes_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                return None;
            }
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .collect()
}

/// Load the named theme from `themes_dir`, falling back to `Theme::default_theme()` on any
/// error. Returns the theme plus an optional warning string (None if loaded cleanly, or if
/// no theme was selected — first-run users shouldn't see a warning for not picking one yet).
///
/// Failure modes that produce a warning:
/// - `theme_name = Some(name)` but `<themes_dir>/<name>.toml` does not exist
/// - Read or parse error on the file
pub fn load_active(
    themes_dir: &std::path::Path,
    theme_name: Option<&str>,
) -> (Theme, Option<String>) {
    let name = match theme_name {
        Some(n) if !n.is_empty() => n,
        _ => return (Theme::default_theme(), None),
    };
    let path = themes_dir.join(format!("{}.toml", name));
    if !path.exists() {
        return (
            Theme::default_theme(),
            Some(format!("theme '{}' not found at {}", name, path.display())),
        );
    }
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            return (
                Theme::default_theme(),
                Some(format!("read theme '{}' failed: {}", name, e)),
            );
        }
    };
    match Theme::parse(&contents) {
        Ok(t) => (t, None),
        Err(e) => (
            Theme::default_theme(),
            Some(format!("theme '{}' invalid: {}", name, e)),
        ),
    }
}

/// Write bundled default themes into `themes_dir`, creating the directory if needed.
/// Idempotent: existing files are preserved (user edits survive upgrades).
pub fn ensure_bundled(themes_dir: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(themes_dir)?;
    for (filename, contents) in BUNDLED {
        let path = themes_dir.join(filename);
        if !path.exists() {
            std::fs::write(&path, contents)?;
        }
    }
    Ok(())
}

fn parse_hex(s: &str) -> Result<Color, ThemeError> {
    if !s.starts_with('#') || s.len() != 7 {
        return Err(ThemeError::InvalidHex(s.to_string()));
    }
    let r = u8::from_str_radix(&s[1..3], 16).map_err(|_| ThemeError::InvalidHex(s.to_string()))?;
    let g = u8::from_str_radix(&s[3..5], 16).map_err(|_| ThemeError::InvalidHex(s.to_string()))?;
    let b = u8::from_str_radix(&s[5..7], 16).map_err(|_| ThemeError::InvalidHex(s.to_string()))?;
    Ok(Color::Rgb(r, g, b))
}

fn parse_hex_opt(s: &Option<String>) -> Result<Option<Color>, ThemeError> {
    match s {
        Some(v) => Ok(Some(parse_hex(v)?)),
        None => Ok(None),
    }
}
