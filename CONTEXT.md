# sqrit — Domain Context

## Glossary

### Connection
Saved database connection configuration stored in `~/.sqrit/connections.toml`.
Contains: name, database type (sqlite/postgres/mysql), host, port, database name, credentials (plaintext in baseline).
No keyring integration until v0.2.

### Backend (Adapter)
Database-specific implementation of the `Database` trait.
Three backends in baseline: SQLite (`rusqlite` via `spawn_blocking`), PostgreSQL (`sqlx`), MySQL (`sqlx`).
All equally first-class. Each backend handles: connect, execute query, schema introspection, disconnect.

### Query
SQL text edited in the query pane. Executed on `Enter` in Normal mode or `Ctrl+Enter` in Insert mode.

### Results
Output from query execution. Rendered as a paginated, scrollable table.
Pagination: fetch N rows at a time. Navigation: `h/j/k/l`.
Copy: cell (`yc`), row (`yy`), all (`ya`). Export: CSV, JSON.
Selection is two-axis: the active row is tinted with `selection_bg`, and the active cell is layered on top with reverse video (`Modifier::REVERSED`); the header cell of the active column is also reverse-highlighted. Selection is persistent — focus changes (`<space>e/q/r`) recolor the border but never clear the cell highlight.

### Explorer
Left sidebar schema browser. Tree hierarchy: connection → tables → columns, views → columns.
`Enter` expands/collapses. `s` runs `SELECT * FROM <table> LIMIT 100`.
Toggleable with `<space>e`.

### Mode
Current input handling state. Three categories:
- **Edit modes** (query pane): Normal, Insert
- **Focus states** (which pane is active): Explorer, Query, Results
- **Modal overlays**: ThemePicker (live-preview picker; tracks the pre-modal theme for Esc-revert)

Each mode has its own `handle_key()` method. Main event loop dispatches to active mode.

### Pane
One of three UI areas in the 3-pane layout:
- **Explorer** (left sidebar, toggleable)
- **Query** (top-right, SQL editor)
- **Results** (bottom-right, query output)

`<space>f` maximizes focused pane.

### Autocomplete
LSP-style auto-triggered suggestions. Appears after idle on word boundary.
Suggests: SQL keywords, table names, column names (from current schema).
No alias resolution in baseline.

### Status Bar
Fixed bar at bottom. Shows: current mode, connection name, query status (idle/running/error), error messages.

### Theme (v0.2)
Visual palette applied across the TUI. Distributed as TOML files in `~/.sqrit/themes/`; five defaults (Rose Pine, Tokyo Night, Nord, Gruvbox, Catppuccin Macchiato) are embedded in the binary and written to that directory on first run (idempotent — existing files are not overwritten). The active theme name is persisted in `~/.sqrit/config.toml`. Switched via `<space>t`, which opens a picker modal with live preview; Enter applies and persists, Esc reverts. Malformed or missing theme files fall back to a hardcoded default with a status-bar warning. See [ADR 5](docs/adr/0005-theme-toml-schema.md).

### Command Palette (v0.2)
Single-letter actions reached via the `<space>` prefix from non-Insert, non-Picker modes:
- `<space>f` — maximize focused pane (existing in v0.1)
- `<space>q` — quit
- `<space>c` — back to the connection picker (change connection)
- `<space>x` — disconnect current connection, return to picker
- `<space>z` — cancel running query (see [Cancel](#cancel-v02))
- `<space>t` — open theme picker
- `<space>h` — open query history picker

Inactive in QueryInsert (`<space>` is literal text) and Picker (`<space>` types into the filter).

### Help Overlay (v0.2)
Press `?` (no prefix) from any mode to toggle a modal listing the active mode's keybindings. Content is sourced from each mode handler's `bindings()` method (auto-generated, never drifts). Esc dismisses.

### Cell Viewer (v0.2)
Press `v` on a selected cell in Results to open a read-only modal with the full value. Long text is scrollable; blobs render as hex. `Tab` toggles between **raw** and **formatted** views (JSON pretty-print for text starting with `{` or `[`; chrono-formatted local time for date/timestamp column types). `y` copies the currently displayed form to the clipboard. Esc closes. No in-place editing in v0.2 — DML generation remains deferred.

### Query History (v0.2)
Per-connection ring of executed queries stored at `~/.sqrit/history/<connection-name>.jsonl` (append-only, capped at 500 entries, rotated on overflow). Each entry: `ts` (ISO 8601 UTC), `sql`, `duration_ms`, `status` (`ok`/`error`), `rows`. Accessed via `<space>h`, which opens a picker modal: newest-first, type to substring-filter on the SQL text, Enter pastes the selected query into the editor (never auto-executes — destructive-query safety), Esc cancels.

### Filter (v0.3)
Client-side **fuzzy** row filter on the **current results page**. `/` in Results mode opens a filter prompt at the bottom of the pane; live-filters as the user types using `nucleo-matcher`'s subsequence scorer (case-insensitive, smart-normalised). All columns are scored independently per row; row score is the sum of column scores. Rows render in descending score order, ties broken by original row order. Matched characters render bold + underlined in the theme accent color. Enter persists the filter — subsequent `PgDn`/`PgUp` page loads re-rank each new page against it. Esc cancels and clears. Filter operates only on rows already loaded (respects invariant V6 — never the full result set in memory).

### Cancel (v0.2)
DB-level cancel of a running query, exposed as `async fn cancel(&self)` on the `Database` trait. Each adapter uses its native mechanism: SQLite via `rusqlite`'s `InterruptHandle`; PostgreSQL via `SELECT pg_cancel_backend($pid)` executed on a side connection (PID captured at connect); MySQL via `KILL QUERY <conn_id>` similarly. Triggered by `<space>z`. Stale results from the cancelled query are discarded by the existing `query_id` guard in `App::drain_async_results`. See [ADR 6](docs/adr/0006-per-adapter-query-cancel.md).

### Paste (v0.2)
Bracketed paste is enabled at startup so terminals deliver clipboard payloads as a single `Event::Paste(String)` rather than a stream of `KeyCode::Char('j') + CONTROL` events (which is how a raw LF byte decodes — `Ctrl+J == LF` in ASCII, hence the historical "every newline became `j`" bug). Paste events route through a new `ModeHandler::handle_paste` trait method (default no-op): `QueryInsert` inserts the text verbatim and refreshes autocomplete; `Picker` / `HistoryPicker` append the first line of the pasted text to the filter and drop the rest. Pasted leading whitespace bypasses the `<space>` command-palette dispatcher. A defensive `Ctrl+J → newline` mapping in Insert mode keeps multi-line paste working on terminals that do not support bracketed paste (older `screen`, raw serial). See V9 in CLAUDE.md.

## Baseline Scope (v0.1)

- TUI only, no CLI mode
- 3 backends: SQLite, PostgreSQL, MySQL
- Connection picker on launch (`sqrit` with no args)
- Saved connections in TOML (plaintext)
- Vim-lite editing (Normal + Insert, basic motions/operators)
- SQL syntax highlighting
- Schema explorer (tables, views, columns)
- Autocomplete (keywords, tables, columns)
- Results table with pagination, copy, export
- 3-pane layout with maximize toggle
- Status bar for mode/status/errors
- Async-first: `tokio` + `sqlx` (PG/MySQL) + `rusqlite` (`spawn_blocking`)

## v0.2 Scope ("Polish")

Power-user polish, no new system dependencies, no CLI mode. Tracked by milestone `v0.2-polish`.

- **Themes** — external TOML in `~/.sqrit/themes/`, 5 defaults bundled (Rose Pine / Tokyo Night / Nord / Gruvbox / Catppuccin Macchiato). See ADR 5.
- **Space command palette** — `<space>c/x/z/t/q/h/f` for actions; `?` for help overlay.
- **Help overlay** — `?` toggles a modal of the active mode's keybindings, sourced from each handler's `bindings()` method. Requires the trait-based dispatch refinement in the [ADR 3 addendum](docs/adr/0003-mode-dispatch-keybinding.md).
- **Cell viewer** — `v` opens a read-only modal with `Tab` raw↔formatted toggle (JSON / date / hex).
- **Query history** — per-connection JSONL ring at `~/.sqrit/history/<conn>.jsonl`, 500 entries, `<space>h` picker pastes into editor (never auto-executes).
- **Row filter** — `/` triggers a client-side, case-insensitive substring filter on the current results page; respects V6. Fuzzy scoring deferred.
- **Cancel running query** — `<space>z` invokes per-adapter native cancel via a new `Database::cancel()` trait method. See ADR 6.

## Deferred to v0.3+

- OS keyring for passwords
- CLI query mode (inline `-q` / file `-f` / CSV/JSON output)
- SSH tunnels (password + key auth)
- Docker auto-discovery
- Full vim engine (text objects, f/t, marks, registers)
- DML generation from selected row (editable cell viewer)
- Alias-aware autocomplete
- Schema browser extension: procedures, indexes, triggers, sequences
- External-author theme TOMLs (user-supplied beyond the 4 bundled defaults — already supported by ADR 5, but no marketplace/discovery UX)
- Cloud-CLI integration (Azure / AWS / GCP)
