use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Global application config stored at `~/.sqrit/config.toml`.
/// Distinct from `Config` (`connections.toml`) so connection storage stays narrow
/// and future global settings (themes, history retention, default editor mode) don't
/// pollute it. See ADR 5.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default)]
    pub hint_bar: HintBarConfig,
}

/// Hint-bar layout knobs. Missing fields → `Default` (enabled = true,
/// auto_hide_narrow = false). Loading is forward-additive: an `AppConfig`
/// written by v0.3.0 (no `[hint_bar]` section) deserialises with defaults.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HintBarConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub auto_hide_narrow: bool,
}

impl Default for HintBarConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_hide_narrow: false,
        }
    }
}

fn default_true() -> bool {
    true
}

impl AppConfig {
    /// Root sqrit config directory (`~/.sqrit/`). Errors if `$HOME` is unresolvable
    /// rather than silently falling back to a relative path (CWD).
    pub fn sqrit_dir() -> anyhow::Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home directory"))?;
        let dir = home.join(".sqrit");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn config_path() -> anyhow::Result<PathBuf> {
        Ok(Self::sqrit_dir()?.join("config.toml"))
    }

    /// Directory holding per-theme TOML files (`~/.sqrit/themes/`).
    pub fn themes_dir() -> anyhow::Result<PathBuf> {
        let dir = Self::sqrit_dir()?.join("themes");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn load_from(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&content)?;
        Ok(cfg)
    }

    pub fn load() -> anyhow::Result<Self> {
        Self::load_from(&Self::config_path()?)
    }

    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to(&Self::config_path()?)
    }

    /// Update the active theme name in-memory and persist to `path`.
    /// Used by tests and the theme picker; production callers go through `set_theme`.
    pub fn set_theme_at(&mut self, name: &str, path: &Path) -> anyhow::Result<()> {
        self.theme = Some(name.to_string());
        self.save_to(path)
    }

    pub fn set_theme(&mut self, name: &str) -> anyhow::Result<()> {
        self.theme = Some(name.to_string());
        self.save()
    }
}
