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

### Explorer
Left sidebar schema browser. Tree hierarchy: connection → tables → columns, views → columns.
`Enter` expands/collapses. `s` runs `SELECT * FROM <table> LIMIT 100`.
Toggleable with `<space>e`.

### Mode
Current input handling state. Three categories:
- **Edit modes** (query pane): Normal, Insert
- **Focus states** (which pane is active): Explorer, Query, Results
- **Transient modes**: Command (vim-style `:` prompt; tracks origin mode for Esc/return)

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
While in Command mode, replaced by an editable `:<buffer>` prompt; on Enter, the buffer is parsed (`q`/`quit`/`q!`/`quit!` → quit) and mode returns to origin.

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

## Deferred to v0.2

- OS keyring for passwords
- Query history
- CLI query mode
- SSH tunnels
- Docker auto-discovery
- Full vim engine (text objects, f/t, marks, registers)
- Fuzzy row filtering in results
- Cell value viewer
- DML generation from selected row
- Alias-aware autocomplete
- Themes
