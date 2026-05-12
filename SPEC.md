# SPEC.md — sqrit v0.1

## §G — Goal

SQL TUI client in Rust. Connect, query, browse. Three backends: SQLite, PostgreSQL, MySQL. Vim-lite editing. Zero-config UX — run `sqrit`, pick connection, go.

## §C — Constraints

- C1: `ratatui` + `crossterm` for TUI
- C2: `tokio` async runtime, single event loop
- C3: `sqlx` for PostgreSQL and MySQL drivers
- C4: `rusqlite` for SQLite, wrapped in `tokio::task::spawn_blocking`
- C5: Connections saved in `~/.sqrit/connections.toml`, plaintext passwords
- C6: No CLI mode, no non-interactive query execution
- C7: No SSH tunnels, no Docker discovery, no keyring
- C8: Binary name `sqrit`, crate name `sqrit`

## §I — External Surfaces

- I1: `~/.sqrit/connections.toml` — connection config file (TOML)
- I2: Terminal — crossterm raw mode, alternate screen
- I3: SQLite files — local `.db` files via `rusqlite`
- I4: PostgreSQL — TCP connection via `sqlx::PgPool`
- I5: MySQL — TCP connection via `sqlx::MySqlPool`
- I6: System clipboard — copy cell/row/all/export via `arboard` or similar

## §V — Invariants

- V1: All DB operations go through `Database` trait. No direct `sqlx`/`rusqlite` calls outside adapter implementations.
- V2: Query execution never blocks the UI thread. All DB calls dispatched via tokio tasks, results returned through channels.
- V3: UI state is a single `App` struct owned by the event loop. No shared mutable state, no `Arc<Mutex<>>` for UI state.
- V4: Every mode implements `handle_key()`. Main loop dispatches to active mode, never matches keys directly.
- V5: Connection config is loaded on startup and saved on mutation. No in-memory-only connections in baseline.
- V6: Results pagination: never load entire result set into memory. Fetch page-size chunks from backend cursor or use `LIMIT/OFFSET`.
- V7: Autocomplete popup dismisses on `Esc`, accepts on `Tab`, triggers only after configurable idle timeout (default 300ms).

## §T — Tasks

id|status|task|deps
T1|x|scaffold: Cargo.toml, src/main.rs, module skeleton, basic ratatui event loop with empty render|-
T2|x|define `Database` trait: connect, disconnect, execute, list_tables, list_columns, list_views|-
T3|x|SQLite adapter: implement `Database` trait using `rusqlite` + `spawn_blocking`|T2
T4|.|PostgreSQL adapter: implement `Database` trait using `sqlx::PgPool`|T2
T5|.|MySQL adapter: implement `Database` trait using `sqlx::MySqlPool`|T2
T6|x|connection config: TOML structs, load/save `~/.sqrit/connections.toml`, CRUD operations|-
T7|x|connection picker screen: list saved connections, filter by name, select to connect|T6,T3
T8|x|3-pane layout: Explorer sidebar (left), Query (top-right), Results (bottom-right). Status bar bottom.|T1
T9|x|query editor insert mode: raw text input, backspace, arrow keys, home/end, multiline|T8
T10|x|query editor normal mode: `i`→insert, `h/j/k/l`, `w/b`, `0/$`, `gg/G`, `dd`, `yy`, `p`, `x`, `u`|T9
T11|x|SQL syntax highlighting: tokenize query text, color keywords/types/strings/comments|T9
T12|x|query execution: wire `Enter` (normal) and `Ctrl+Enter` (insert) to `Database::execute`, show status|T3,T10
T13|x|results table: render `Vec<Row>` as scrollable table, column headers, `h/j/k/l` cell nav|T8,T12
T14|.|results pagination: fetch page-size chunks, `PgDn`/`PgUp` load next/prev page|T13,V6
T15|.|results copy/export: `yc` copy cell, `yy` copy row, `ya` copy all, export to CSV/JSON|T13,I6
T16|.|explorer tree: render schema tree (tables→columns, views→columns), expand/collapse with Enter|T8,T3
T17|.|explorer actions: `s` runs `SELECT * FROM <table> LIMIT 100`, toggle explorer `<space>e`|T16,T12
T18|.|autocomplete popup: render suggestion list below cursor position, `Tab` accept, `Esc` dismiss|T9
T19|.|autocomplete engine: keyword list + table/column suggestions from current schema via `Database` trait|T18,T3,V7
T20|.|status bar: show mode, connection name, query status (idle/running/error), error messages|T8
T21|.|maximize toggle: `<space>f` expands focused pane to full screen, toggle back|T8
T22|.|pane focus: `e`/`q`/`r` keys switch focus to Explorer/Query/Results|T8
T23|.|integration tests: test each adapter with real DB (SQLite file, PG/MySQL in Docker)|T3,T4,T5

## §B — Bug Log

id|date|cause|fix
