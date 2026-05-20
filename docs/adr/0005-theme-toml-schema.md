# ADR 5: Themes as external TOML, embedded defaults, first-run write

**Status:** Accepted (planned for v0.2)

## Context

v0.2 introduces theming. Every TUI surface (borders, status bar, results table, syntax-highlighted SQL, autocomplete popup, modals) currently reads `Color::X` literals scattered across `src/app.rs` and the mode renderers. We need a single source of truth for colors, a way to ship five named presets (Rose Pine, Tokyo Night, Nord, Gruvbox, Catppuccin Macchiato), and a path for users to add their own themes without recompiling.

## Decision

- **Schema.** A theme is a TOML file with a fixed set of named colors. Required fields:

  ```toml
  name = "tokyo-night"

  [colors]
  bg              = "#1a1b26"
  fg              = "#c0caf5"
  border_focused  = "#7aa2f7"
  border_unfocused = "#414868"
  selection_bg    = "#283457"
  keyword         = "#bb9af7"
  string          = "#9ece6a"
  comment         = "#565f89"
  number          = "#ff9e64"
  type            = "#7dcfff"
  error           = "#f7768e"
  ```

  Hex strings only; parsed via `ratatui::style::Color::from_str` (RGB hex), which is sufficient for terminal output and avoids a custom color parser. Missing required fields are an error.

- **Storage.** Themes live in `~/.sqrit/themes/<name>.toml`. Default file names: `rose-pine.toml`, `tokyo-night.toml`, `nord.toml`, `gruvbox.toml`, `catppuccin-macchiato.toml`. The active theme is selected by name in `~/.sqrit/config.toml`:

  ```toml
  # ~/.sqrit/config.toml
  theme = "tokyo-night"
  ```

  `config.toml` is a new file separate from `connections.toml` so the connection store stays minimal and the new theming domain doesn't leak in.

- **First-run write.** The five default themes are embedded in the binary via `include_str!`. On startup, sqrit ensures `~/.sqrit/themes/` exists and writes any default file that is not already present (idempotent — existing files are never overwritten, so user edits survive upgrades).

- **Loading + fallback.** At startup, sqrit loads the file matching the `theme` name from `config.toml`. On error (file missing, malformed TOML, missing required field), it falls back to a hardcoded `Theme::default()` baked into the binary and shows a one-line status-bar warning naming the failure. No crash, ever.

- **Switching.** `<space>t` opens a picker modal listing every `.toml` file in `~/.sqrit/themes/` (scanned at modal-open time, not cached, so dropped-in themes appear without a restart). Arrow keys preview the theme live (UI re-renders with the candidate `Theme`). Enter applies and persists the choice to `config.toml`. Esc reverts to the pre-modal theme.

## Rationale

- **External TOML over hardcoded structs**: chosen against the recommendation precisely because the cost of letting users add their own themes is small (just allow more files in a directory we already scan) and the win is large for a polish-themed release.
- **Embedded + first-run write**, instead of asking users to download themes: zero-config UX (`sqrit` Just Works after install with a sensible default), but the files exist on disk so users can edit them in place to tweak.
- **`Color::from_str` (hex only)** over a richer color spec (24-bit / 256-color / named ANSI): ratatui handles terminal color quantization downstream. Hex strings are unambiguous, easy to copy from existing color schemes online, and won't grow a parser bug.
- **Fixed required field list** over a free-form map: TUI surfaces are known and stable. Missing a field is a real error (e.g. forgetting `border_focused` would render invisible panes). Catching it at load time beats discovering it during use.
- **No `Theme::default()` from a user file**: the hardcoded default in the binary is the floor. A user's broken file can never break sqrit's ability to start.
- **`config.toml` separate from `connections.toml`**: keeps the connection store narrow and avoids version-skew migrations when adding future global settings (history retention, default editor mode, etc.).

## Consequences

- Adds `~/.sqrit/config.toml` to the file footprint. New file → new doc entry in README under Configuration, and likely a v0.3 ADR if more global settings land.
- First-run side effect: sqrit writes 5 files into `~/.sqrit/themes/`. Documented in CHANGELOG and Configuration; safe because writes are gated on file-not-exists.
- Live-preview-on-arrow-keys means the picker modal must hold a snapshot of the prior `Theme` to restore on Esc. State adds one field to the picker's `ModalState`.
- Future external themes (community-shared, marketplace) are already supported by the schema. v0.2 ships only the 5 defaults and the loader; discovery UX is deferred to v0.3+.
- Color quantization on 16-color terminals is delegated to ratatui — themes may look subtly different in a low-color terminal. Not a sqrit concern.
