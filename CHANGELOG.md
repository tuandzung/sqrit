# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.4.0] - 2026-07-16

### Added

- `gs` in Query Normal executes the statement under the cursor without changing whole-buffer `Enter` or `Ctrl+Enter` behavior.
- Statement scanning honors backend-aware quotes, SQLite brackets, comments, backticks, and PostgreSQL dollar blocks; malformed regions and recognized compound SQLite/MySQL definitions execute nothing.
- The executed source range remains highlighted with the active theme's `selection_bg`, and query history stores only the SQL sent to the adapter.

## [0.3.2] - 2026-07-15

### Added

- Namespace-aware schema browser with collapsible, counted object-kind groups under PostgreSQL schemas, the connected MySQL database, or SQLite's implicit namespace. SQLite shows tables, views, indexes, and triggers; PostgreSQL also shows materialized views, functions, procedures, and sequences; MySQL also shows functions and procedures. Empty groups are hidden, and a lone namespace is omitted from the tree.
- ADR 8: namespace-aware introspection.
- Invariant V11 in `CLAUDE.md`.
- Per-adapter identifier quoting helpers in `src/db/quote.rs`.

### Changed

- `Database::schema_info()` now returns namespace-rooted `SchemaInfo { namespaces: Vec<Namespace> }`. PostgreSQL filters `pg_catalog`, `information_schema`, `pg_toast`, `pg_temp_*`, and `pg_toast_temp_*`; MySQL introspects only the database returned by `DATABASE()`. `ExplorerState::set_schema()` installs new schema data and resets tree state.
- `s` in Explorer now generates a qualified, adapter-quoted `SELECT * FROM <namespace>.<object> LIMIT 100` for tables, views, and materialized views. Other object kinds leave the query unchanged and report a status message.

## [0.3.1] - 2026-07-15

### Added

- Hint bar: single-row, mode-aware keybinding hints above the status bar. Every mode, including Picker and transient modes, lists its top bindings from `ModeHandler::bindings()` on the left. `QueryNormal`, `Explorer`, and `Results` show `<sp> cmd  ? help` on the right, where both shortcuts are active. The status bar takes priority on a one-row terminal.
- `[hint_bar]` config section in `~/.sqrit/config.toml` (`enabled`, `auto_hide_narrow`).
- Optional `[colors].hint_bar_bg`, `[colors].hint_bar_fg`, `[colors].hint_bar_key`, and `[colors].hint_bar_separator` theme keys. Missing keys fall back to existing palette colors. All five bundled themes (Rose Pine, Tokyo Night, Nord, Gruvbox, and Catppuccin Macchiato) define them.
- ADR 7: hint bar layout.
- Invariant V10 in `CLAUDE.md`.

## [0.3.0] - 2026-05-26

### Added

- Added `nucleo-matcher` 0.3 (MPL-2.0) as the fuzzy scoring engine for Results row filtering.

### Changed

- Results filtering now uses fuzzy subsequence scoring across all loaded columns, ranks matches by score, highlights matched characters, and re-ranks each newly loaded page while preserving V6 pagination boundaries.

## [0.2.0] - 2026-05-25

### Added

- Themes: TUI reads colors from a `Theme` loaded from `~/.sqrit/themes/<name>.toml`. Five defaults are bundled and written on first run (Rose Pine, Tokyo Night, Nord, Gruvbox, Catppuccin Macchiato). Active theme persisted in `~/.sqrit/config.toml`. `<space>t` opens a picker modal with live preview (arrow keys), Enter persists, Esc reverts to the pre-modal theme. Malformed or missing theme files fall back to a hardcoded default with a status-bar warning. See [ADR 5](docs/adr/0005-theme-toml-schema.md) (T1).
- Space command palette: from `QueryNormal` / `Explorer` / `Results`, a leading `<space>` arms the palette and the next key dispatches a top-level action — `<space>f` maximize, `<space>t` theme picker, `<space>q` quit, `<space>c` back to connection picker (db preserved), `<space>x` disconnect (clears db/schema/active connection, returns to picker), `<space>z` cancel running query (stub status until T7), `<space>h` query history picker (stub status until T5). Palette is inert in `QueryInsert` (space stays a literal char) and `Picker` (space is typed into the filter). See [CONTEXT.md "Command Palette"](CONTEXT.md) (T2, #32).
- Help overlay: `?` from `QueryNormal` / `Explorer` / `Results` opens a modal listing the active mode's keybindings, themed with the active palette. Esc (or `?` again) dismisses and restores the previous mode. Inactive in `QueryInsert` (literal `?`) and `Picker` (typed into the filter). Internals: a new `ModeHandler { dispatch, bindings }` trait co-locates each mode's key dispatch with its help entries — adding a key without listing it in `bindings()` is now a single-impl-block change that PR review catches. `Mode::handler()` returns the trait object; `Mode::handle_key()` delegates to it. See the [ADR 3 trait-dispatch addendum](docs/adr/0003-mode-dispatch-keybinding.md) (T3, #33).
- Row filter: `/` in `Results` opens a one-line filter prompt at the bottom of the pane. Typing live-filters the loaded rows by case-insensitive substring across **all** columns; Enter locks the filter, Esc cancels and clears it. While the filter is active, the pane title shows `Results (filter: <term>)`, `,c` (comma-c) clears it without re-opening the prompt, and `j`/`k` skip rows hidden by the filter. Pagination (`PgDn`/`PgUp`) re-applies the locked filter to each newly-fetched page — no extra rows are fetched, so V6 stays intact (T6, #36).
- Query history: every executed query (success or error) appends a JSONL record to `~/.sqrit/history/<connection-name>.jsonl` (`ts`, `sql`, `duration_ms`, `status`, `rows`). Each file is capped at 500 entries via ring-buffer rewrite on overflow. `<space>h` opens a modal picker showing entries newest-first; type to substring-filter on the SQL; Enter pastes the selected query into the editor (no auto-execute — destructive safety); Esc cancels without modifying the editor. Replaces T2's `<space>h` stub (T5, #35).
- Cell viewer modal: `v` in Results opens a read-only modal (~60% width × 80% height, centered, themed) showing the full value of the selected cell. `Tab` toggles raw ↔ formatted; `y` copies the currently displayed string to the clipboard; `j`/`k` scroll long content; `Esc` closes. Formatted view pretty-prints JSON text starting with `{`/`[`, hex-dumps blobs (16 bytes/line with an 8-char address column), and routes date / timestamp text (when the column type hint is `date`/`datetime`/`timestamp`/`timestamptz`) through `chrono` for a local-timezone render. NULLs render as `NULL` (raw) and unknown formatted shapes fall back to the raw string rather than mangle content. See [CONTEXT.md "Cell Viewer"](CONTEXT.md) (T4, #34).
- Query cancel: `<space>z` issues a DB-level cancel of the running query via the new `Database::cancel()` trait method — SQLite uses `InterruptHandle`, PostgreSQL fires `pg_cancel_backend()` on a side connection, MySQL issues `KILL QUERY <id>` on a side connection. The cancel future is dispatched via tokio so the UI thread never blocks; the existing `query_id` guard in `App::drain_async_results` discards any stale result returned by the interrupted query. Status bar reports `query cancelled`, or `query cancelled — transaction may need ROLLBACK` when the connection is sitting inside an open transaction (`Database::in_transaction()` checked after cancel lands). Replaces T2's `<space>z` stub. See [ADR 6](docs/adr/0006-per-adapter-query-cancel.md) (T7, #37).
- Active-cell highlight in Results: the active cell now renders with reverse video (`Modifier::REVERSED`) layered over the existing row tint, and the header cell of the active column is also reverse-highlighted. Cursor location is now unambiguous on wide tables — `yc` / `v` always act on the visibly highlighted cell. Selection is persistent across pane focus changes. No theme schema change — reverse video is palette-agnostic across the five bundled themes and any user TOML in `~/.sqrit/themes/` (T8, #52).

### Fixed

- Clipboard copies (`yc` / `yy` / `ya` in Results, and `y` in the Cell Viewer) now actually land in the system clipboard across Linux/X11, Linux/Wayland, macOS, and Windows. The previous implementation constructed an `arboard::Clipboard` inside the copy function and dropped it on return, killing X11's selection-serve thread before the user's clipboard manager could sample the new contents — arboard logged `"Clipboard was dropped very quickly after writing (0ms); clipboard managers may not have seen the contents."`. The new `crate::clipboard::ClipboardWriter` chooses a backend lazily on the first copy: on **Linux/Wayland** with `wl-copy` on `$PATH` it shells out to `wl-copy` (which daemonises itself and holds the selection until something else takes it) — needed because arboard 3.x's Wayland path is silent on several modern compositors (verified on niri with `examples/clipboard_repro`: `set_text()` returns Ok but the compositor never sees the selection); on **everything else** it owns a single `arboard::Clipboard` for the app's lifetime so X11's selection-serve thread persists between copies. The "one backend per writer" invariant is locked by a regression test in `tests/clipboard_writer.rs`. The free `clipboard::copy_to_clipboard` is deprecated; call sites now use `app.clipboard_writer.copy(...)`.
- Pasting from the system clipboard into Insert mode on Linux no longer renders each newline as a literal `j`. Bracketed paste is enabled at startup (`EnableBracketedPaste`) and disabled at shutdown — including via a panic hook that restores the terminal so a crash never leaves the user's shell in bracketed mode. Multi-line clipboard payloads arrive as a single `Event::Paste(String)` and route through a new `ModeHandler::handle_paste` trait method (default no-op): `QueryInsert` inserts the verbatim text (autocomplete refreshes); `Picker` / `HistoryPicker` append the first line to the filter and discard the rest. The original symptom was `ASCII LF (0x0A) → KeyEvent { code: Char('j'), modifiers: CONTROL }` because `Ctrl+J == LF`; a defensive Ctrl+J → newline mapping in Insert mode is kept as a belt-and-suspenders fallback for terminals that don't support bracketed paste (older `screen`, raw serial). See V9 in CLAUDE.md (T9, #50).

### Removed

- Vim-style `:` command mode. The sole `:q` / `:quit` action it carried is fully covered by `<space>q` from the new command palette, and there are no multi-arg `:` commands planned. `:` is now a no-op outside `QueryInsert` (where it remains a literal character). `Mode::Command`, `App::command_buffer`, and `App::command_origin` were removed; see the 2026-05-21 addendum in [ADR 3](docs/adr/0003-mode-dispatch-keybinding.md).

## [0.1.1] - 2026-05-20

### Added

- INSERT mode now renders a visible cursor in the query editor via `frame.set_cursor()`, honoring viewport scroll for long lines (V8, T24).
- Explorer pane is now viewport-aware: `ExplorerState` tracks `scroll_offset`, rendering uses `.skip().take()`, and the offset auto-adjusts when the selection moves off-screen — mirrors the `ResultsState::adjust_scroll` pattern (T25).
- Vim-style command mode: `:` from QueryNormal / Explorer / Results enters command mode; `:q`, `:quit`, `:q!`, `:quit!` + Enter quit the app. Esc cancels and restores the previous mode. Unknown commands surface as a status message.
- Local integration runner: `justfile` + `docker-compose.yml` at the repo root expose `just it`, `just it-pg`, `just it-mysql`, `just db-up`/`db-down`, plus dev shortcuts (`just check`, `just fmt`, `just lint`). Ports/credentials match CI Services. See `docs/adr/0004-local-integration-runner.md`.

### Fixed

- Autocomplete Tab accept now replaces the typed word prefix instead of appending the suggestion to it (e.g. typing `SEL` + Tab on `SELECT` now yields `SELECT`, not `SELSELECT`). New `EditorBuffer::delete_backwards(n)` removes the prefix in a single undoable step (T26).
- MySQL adapter: columns declared `BOOLEAN` / `BOOL` now decode to `Value::Boolean` instead of rendering the literal `<unsupported mysql type: BOOLEAN>`. sqlx-mysql reports `"BOOLEAN"` as the type name for `TINYINT(1)` columns, which previously fell through to the unsupported-type branch (#27).

## [0.1.0] - 2026-05-14

### Added

- Project scaffold: Cargo.toml, module skeleton, ratatui event loop with crossterm
- `Database` trait: connect, disconnect, execute, list_tables, list_columns, list_views
- SQLite adapter via `rusqlite` + `tokio::task::spawn_blocking`
- PostgreSQL adapter via `sqlx::PgPool`
- MySQL adapter via `sqlx::MySqlPool`
- Connection config: TOML structs, load/save `~/.sqrit/connections.toml`
- Connection picker: list saved connections, filter by name, select to connect
- 3-pane layout: Explorer sidebar, Query editor, Results table, status bar
- Query editor insert mode: raw text input, backspace, arrow keys, home/end, multiline
- Query editor normal mode: `i`/insert, `h/j/k/l`, `w/b`, `0/$`, `gg/G`, `dd`, `yy`, `p`, `x`, `u`
- SQL syntax highlighting: tokenize query, color keywords/types/strings/comments
- Query execution: `Enter` (normal) and `Ctrl+Enter` (insert) wired to `Database::execute`
- Results table: scrollable table with column headers, `h/j/k/l` cell navigation
- Results pagination: fetch page-size chunks, `PgDn`/`PgUp` load next/prev page
- Results copy/export: `yc` copy cell, `yy` copy row, `ya` copy all, export to CSV/JSON
- Explorer tree: schema tree (tables/columns, views/columns), expand/collapse with Enter
- Explorer actions: `s` runs `SELECT * FROM <table> LIMIT 100`, toggle with `<space>e`
- Autocomplete popup: suggestion list below cursor, `Tab` accept, `Esc` dismiss, `Up/Down` navigate
- Autocomplete engine: keyword list + table/column suggestions from current schema
- Status bar: mode, connection name, query status (idle/running/error), error messages
- Pane focus: `e`/`q`/`r` keys switch focus to Explorer/Query/Results
- Maximize toggle: `<space>f` expands focused pane to full screen
- Integration tests: cross-adapter tests with SQLite file, PostgreSQL and MySQL in Docker

### Fixed

- Query execution no longer blocks UI thread — dispatched via `tokio::spawn` + `mpsc` channel
- Pagination races guarded by `query_id` — stale results discarded
- Connection picker calls `connect()` on adapter before schema load
- Schema load errors surfaced through channel instead of silently swallowed
