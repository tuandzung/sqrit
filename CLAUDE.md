# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo build                    # build
cargo run                      # run TUI
cargo test                     # all tests (110+; PG/MySQL need servers)
cargo test --test sqlite_adapter  # single test file
cargo test connect_opens       # single test by name
cargo clippy -- -D warnings    # lint
cargo fmt --check              # format check
```

## CI/CD

Three GitHub Actions workflows:

| Workflow | Trigger | What it does |
|----------|---------|--------------|
| `ci.yml` | PR to main | build, clippy (`-D warnings`), fmt check, SQLite tests |
| `integration.yml` | Push to main | full test suite with PG + MySQL via GitHub Services |
| `release.yml` | Tag `v*` | extract changelog, cross-compile 4 targets, publish GitHub Release |

**Release targets**: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` (via `cross` on Ubuntu), `x86_64-apple-darwin`, `aarch64-apple-darwin` (native on macOS).

**Integration test env vars**: `PG_HOST=localhost PG_PORT=5432 PG_USER=sqrit_test PG_PASS=sqrit_test PG_DB=sqrit_test` and equivalent `MYSQL_*`.

**Release body**: extracted from CHANGELOG.md matching the tag version section.

## Architecture

**SPEC.md** drives all work. It defines invariants (§V), tasks with status (§T), and bug log (§B). Check it before starting any task. Use `/ck:build` to implement tasks, `/ck:spec` to mutate the spec.

**Three core layers**, all owned by a single `App` struct (V3: no Arc<Mutex<>> for UI state):

1. **Database** (`src/db/`) — `Database` trait in `mod.rs`, adapters per backend. All DB ops go through this trait (V1). SQLite, PostgreSQL, MySQL adapters done. Sync rusqlite calls wrapped in `tokio::task::spawn_blocking`.

2. **Modes** (`src/mode.rs` + `src/mode/`) — `Mode` enum dispatches `handle_key()` to mode-specific handlers. Every mode implements `handle_key()` (V4). Never match keys in the main loop. Modes: Picker, Explorer, QueryNormal, QueryInsert, Results.

3. **Event loop** (`src/app.rs`) — crossterm raw mode + alternate screen. Polls at 100ms, dispatches key events to active mode. DB calls dispatched via `tokio::spawn`, results returned through `mpsc` channel (V2). Schema loads also async. `drain_async_results()` processes results each tick.

**Connection config** (`src/config/mod.rs`) — TOML at `~/.sqrit/connections.toml`. Loaded on startup, saved on mutation (V5).

**Mode borrow pattern**: `let mode = self.mode; mode.handle_key(key, self);` — copies Mode (it's Copy) to avoid borrow conflict.

**App state fields** — `results: Option<QueryResult>`, `query_status: QueryStatus` (Idle/Running/Success/Error), `pending_query: Option<String>`, `results_state: ResultsState`, `editor: EditorBuffer`, `normal_state: NormalState`.

**SQL tokenizer** (`src/sql.rs`) — `tokenize(sql) -> Vec<Token>` with kinds: Keyword, Type, String, Comment, Number, Identifier, Operator, Punctuation, Whitespace. Editor rendering converts tokens to styled `Span`s.

**Results navigation** (`src/results.rs`) — `ResultsState` tracks `selected_row/col`, `scroll_row`, `visible_rows`. Auto-scrolls when selection exceeds visible area.

## Invariants (from SPEC.md §V)

- V1: No direct sqlx/rusqlite outside adapter impls
- V2: DB calls never block UI thread (fixed: spawn tokio task, mpsc channel)
- V3: Single App struct, no shared mutable UI state
- V4: Modes handle keys, main loop dispatches only
- V5: Connections persisted to disk
- V6: Paginated results, never full result set in memory
- V7: Autocomplete triggers after 300ms idle

## Bug Log

- B1 (fixed): `execute_pending().await` blocked event loop. Fixed: spawn tokio task + mpsc channel.

## Domain Glossary

See `CONTEXT.md` for term definitions (Connection, Backend, Query, Results, Explorer, Mode, Pane, Autocomplete, Status Bar).
