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

### Namespace (v0.3)
A backend-specific container for schema objects. PostgreSQL exposes each user schema and filters `pg_catalog`, `information_schema`, `pg_toast`, `pg_temp_*`, and `pg_toast_temp_*`; MySQL exposes only the selected database returned by `DATABASE()`; SQLite exposes one implicit namespace with an empty name.

Explorer omits the namespace row when exactly one namespace exists.

### Object Kind (v0.3)
A schema object's category. Explorer uses it to group objects, decide whether `s` can run `SELECT *`, and name status messages.

| Kind | SQLite | PostgreSQL | MySQL | `s` |
|------|--------|------------|-------|-----|
| Table | ✓ | ✓ | ✓ | ✓ |
| View | ✓ | ✓ | ✓ | ✓ |
| Materialized View | — | ✓ | — | ✓ |
| Index | ✓ | ✓ | ✓ | — |
| Trigger | ✓ | ✓ | ✓ | — |
| Function | — | ✓ | ✓ | — |
| Procedure | — | ✓ | ✓ | — |
| Sequence | — | ✓ | — | — |

### Query
SQL text edited in the query pane. Executed on `Enter` in Normal mode or `Ctrl+Enter` in Insert mode.

### Results
Output from query execution. Rendered as a paginated, scrollable table.
Pagination: fetch N rows at a time. Navigation: `h/j/k/l`.
Copy: cell (`yc`), row (`yy`), all (`ya`). Export: CSV, JSON.
Selection is two-axis: the active row is tinted with `selection_bg`, and the active cell is layered on top with reverse video (`Modifier::REVERSED`); the header cell of the active column is also reverse-highlighted. Selection is persistent — focus changes (`<space>e/q/r`) recolor the border but never clear the cell highlight.

### Explorer
Left sidebar schema browser. Hierarchy: `Namespace → Group (Object Kind) → Object → Column`; columns appear only under tables, views, and materialized views. Empty groups are hidden. When introspection returns one namespace, Explorer omits the namespace row and starts with object-kind groups.

`Enter` toggles the selected expandable node. `j` and `k` walk visible items.

On a table, view, materialized view, or one of its columns, `s` runs `SELECT * FROM <namespace>.<object> LIMIT 100`. Explorer quotes identifiers per backend: `"namespace"."object"` for PostgreSQL, `` `namespace`.`object` `` for MySQL, and `"object"` for SQLite's empty namespace. On other object kinds, `s` leaves the query unchanged and writes a status-bar message. See [ADR 8](docs/adr/0008-namespace-aware-introspection.md).

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
Fixed bar at the bottom row. Shows: current mode, connection name, query status (idle/running/error), error messages. The hint bar (v0.3) renders one row above when enabled — see "Hint Bar".

### Theme (v0.2)
Visual palette applied across the TUI. Distributed as TOML files in `~/.sqrit/themes/`; five defaults (Rose Pine, Tokyo Night, Nord, Gruvbox, Catppuccin Macchiato) are embedded in the binary and written to that directory on first run (idempotent — existing files are not overwritten). The active theme name is persisted in `~/.sqrit/config.toml`. Switched via `<space>t`, which opens a picker modal with live preview; Enter applies and persists, Esc reverts. Malformed or missing theme files fall back to a hardcoded default with a status-bar warning. See [ADR 5](docs/adr/0005-theme-toml-schema.md).

### Command Palette (v0.2)
Single-letter actions reached via the `<space>` prefix from `QueryNormal`, `Explorer`, and `Results`:
- `<space>f` — maximize focused pane (existing in v0.1)
- `<space>q` — quit
- `<space>c` — back to the connection picker (change connection)
- `<space>x` — disconnect current connection, return to picker
- `<space>z` — cancel running query (see [Cancel](#cancel-v02))
- `<space>t` — open theme picker
- `<space>h` — open query history picker

Inactive elsewhere. QueryInsert inserts `<space>` as text; Picker, ResultsFilter, and HistoryPicker type it into their filters.

### Help Overlay (v0.2)
Press `?` (no prefix) from `QueryNormal`, `Explorer`, or `Results` to open a modal listing that mode's keybindings. Content comes from the mode handler's `bindings()` method. Esc dismisses. Input modes keep `?` as literal text.

### Hint Bar (v0.3)
Single reserved row above the status bar. Every mode, including Picker and transient modes, renders its top keybindings from `ModeHandler::bindings()` on the left. `QueryNormal`, `Explorer`, and `Results` also render `<sp> cmd  ? help` on the right because both shortcuts are active there. Trailing mode bindings truncate on narrow terminals; a binding wider than the row becomes `…`. On a one-row terminal, the status bar takes the row and the hint is suppressed.

Configured under `[hint_bar]` in `~/.sqrit/config.toml`:
- `enabled` (bool, default `true`) — false hides the row entirely; the status bar reclaims the space.
- `auto_hide_narrow` (bool, default `false`) — true omits the row when terminal width < 40 cols, keeping the row present at all widths otherwise.

Colored via optional `hint_bar_bg`, `hint_bar_fg`, `hint_bar_key`, `hint_bar_separator` in the theme TOML's `[colors]` section. Missing fields fall back to existing palette colors. See [ADR 7](docs/adr/0007-hint-bar.md).

### Cell Viewer (v0.2)
Press `v` on a selected cell in Results to open a read-only modal with the full value. Long text is scrollable; blobs render as hex. `Tab` toggles between **raw** and **formatted** views (JSON pretty-print for text starting with `{` or `[`; chrono-formatted local time for date/timestamp column types). `y` copies the currently displayed form to the clipboard. Esc closes. No in-place editing in v0.2 — DML generation remains deferred.

### Query History (v0.2)
Per-connection ring of executed queries stored at `~/.sqrit/history/<connection-name>.jsonl` (append-only, capped at 500 entries, rotated on overflow). Each entry: `ts` (ISO 8601 UTC), `sql`, `duration_ms`, `status` (`ok`/`error`), `rows`. Accessed via `<space>h`, which opens a picker modal: newest-first, type to substring-filter on the SQL text, Enter pastes the selected query into the editor (never auto-executes — destructive-query safety), Esc cancels.

### Filter (v0.3)
Client-side **fuzzy** row filter on the **current results page**. `/` in Results mode opens a filter prompt at the bottom of the pane; live-filters as the user types using `nucleo-matcher`'s subsequence scorer (case-insensitive, smart-normalised). All columns are scored independently per row; row score is the sum of column scores. Rows render in descending score order, ties broken by original row order. Matched graphemes in each matching column render bold + underlined in the theme's focused-border color. Enter persists the filter — subsequent `PgDn`/`PgUp` page loads re-rank each new page against it. Esc cancels and clears. Filter operates only on rows already loaded (respects invariant V6 — never the full result set in memory).

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
- **Row filter (historical v0.2 behavior)** — `/` performed case-insensitive substring matching on the current results page; it was replaced by the v0.3 [Filter](#filter-v03).
- **Cancel running query** — `<space>z` invokes per-adapter native cancel via a new `Database::cancel()` trait method. See ADR 6.

## Deferred to v0.3+

- OS keyring for passwords
- CLI query mode (inline `-q` / file `-f` / CSV/JSON output)
- SSH tunnels (password + key auth)
- Docker auto-discovery
- Full vim engine (text objects, f/t, marks, registers)
- DML generation from selected row (editable cell viewer)
- Alias-aware autocomplete
- External-author theme TOMLs (user-supplied beyond the 4 bundled defaults — already supported by ADR 5, but no marketplace/discovery UX)
- Cloud-CLI integration (Azure / AWS / GCP)
