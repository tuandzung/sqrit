# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.1] - 2026-05-20

### Added

- INSERT mode now renders a visible cursor in the query editor via `frame.set_cursor()`, honoring viewport scroll for long lines (V8, T24).
- Explorer pane is now viewport-aware: `ExplorerState` tracks `scroll_offset`, rendering uses `.skip().take()`, and the offset auto-adjusts when the selection moves off-screen — mirrors the `ResultsState::adjust_scroll` pattern (T25).
- Vim-style command mode: `:` from QueryNormal / Explorer / Results enters command mode; `:q`, `:quit`, `:q!`, `:quit!` + Enter quit the app. Esc cancels and restores the previous mode. Unknown commands surface as a status message.

### Fixed

- Autocomplete Tab accept now replaces the typed word prefix instead of appending the suggestion to it (e.g. typing `SEL` + Tab on `SELECT` now yields `SELECT`, not `SELSELECT`). New `EditorBuffer::delete_backwards(n)` removes the prefix in a single undoable step (T26).

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
