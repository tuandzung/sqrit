# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo build                    # build
cargo run                      # run TUI
cargo test                     # all tests (63)
cargo test --test sqlite_adapter  # single test file
cargo test connect_opens       # single test by name
```

## Architecture

**SPEC.md** drives all work. It defines invariants (§V), tasks with status (§T), and bug log (§B). Check it before starting any task. Use `/ck:build` to implement tasks, `/ck:spec` to mutate the spec.

**Three core layers**, all owned by a single `App` struct (V3: no Arc<Mutex<>> for UI state):

1. **Database** (`src/db/`) — `Database` trait in `mod.rs`, adapters per backend. All DB ops go through this trait (V1). SQLite adapter done (`sqlite.rs`), PostgreSQL and MySQL pending. Sync rusqlite calls wrapped in `tokio::task::spawn_blocking`.

2. **Modes** (`src/mode.rs` + `src/mode/`) — `Mode` enum dispatches `handle_key()` to mode-specific handlers. Every mode implements `handle_key()` (V4). Never match keys in the main loop. Modes: Picker, Explorer, QueryNormal, QueryInsert, Results.

3. **Event loop** (`src/app.rs`) — crossterm raw mode + alternate screen. Polls at 100ms, dispatches key events to active mode. After dispatch, drains `pending_query` via `execute_pending()` (V2, known drift: B1).

**Connection config** (`src/config/mod.rs`) — TOML at `~/.sqrit/connections.toml`. Loaded on startup, saved on mutation (V5).

**Mode borrow pattern**: `let mode = self.mode; mode.handle_key(key, self);` — copies Mode (it's Copy) to avoid borrow conflict.

**App state fields** — `results: Option<QueryResult>`, `query_status: QueryStatus` (Idle/Running/Success/Error), `pending_query: Option<String>`, `results_state: ResultsState`, `editor: EditorBuffer`, `normal_state: NormalState`.

**SQL tokenizer** (`src/sql.rs`) — `tokenize(sql) -> Vec<Token>` with kinds: Keyword, Type, String, Comment, Number, Identifier, Operator, Punctuation, Whitespace. Editor rendering converts tokens to styled `Span`s.

**Results navigation** (`src/results.rs`) — `ResultsState` tracks `selected_row/col`, `scroll_row`, `visible_rows`. Auto-scrolls when selection exceeds visible area.

## Invariants (from SPEC.md §V)

- V1: No direct sqlx/rusqlite outside adapter impls
- V2: DB calls never block UI thread (B1: currently drifts — `execute_pending` awaits inline)
- V3: Single App struct, no shared mutable UI state
- V4: Modes handle keys, main loop dispatches only
- V5: Connections persisted to disk
- V6: Paginated results, never full result set in memory
- V7: Autocomplete triggers after 300ms idle

## Bug Log

- B1: `execute_pending().await` blocks event loop during query. Fix: spawn tokio task + oneshot channel.

## Domain Glossary

See `CONTEXT.md` for term definitions (Connection, Backend, Query, Results, Explorer, Mode, Pane, Autocomplete, Status Bar).
