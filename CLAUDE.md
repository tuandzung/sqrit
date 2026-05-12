# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo build                    # build
cargo run                      # run TUI
cargo test                     # all tests
cargo test --test sqlite_adapter  # single test file
cargo test connect_opens       # single test by name
```

## Architecture

**SPEC.md** drives all work. It defines invariants (§V), tasks with status (§T), and bug log (§B). Check it before starting any task. Use `/ck:build` to implement tasks, `/ck:spec` to mutate the spec.

**Three core layers**, all owned by a single `App` struct (V3: no Arc<Mutex<>> for UI state):

1. **Database** (`src/db/`) — `Database` trait in `mod.rs`, adapters per backend. All DB ops go through this trait (V1). SQLite adapter done (`sqlite.rs`), PostgreSQL and MySQL pending. Sync rusqlite calls wrapped in `tokio::task::spawn_blocking`.

2. **Modes** (`src/mode.rs` + `src/mode/`) — `Mode` enum dispatches `handle_key()` to mode-specific handlers. Every mode implements `handle_key()` (V4). Never match keys in the main loop. Currently stubs; real implementation pending.

3. **Event loop** (`src/app.rs`) — crossterm raw mode + alternate screen. Polls at 100ms, dispatches key events to active mode. Async DB calls dispatched via tokio, never block UI (V2).

**Connection config** (`src/config/mod.rs`) — TOML at `~/.sqrit/connections.toml`. Loaded on startup, saved on mutation (V5).

**Mode borrow pattern**: `let mode = self.mode; mode.handle_key(key, self);` — copies Mode (it's Copy) to avoid borrow conflict.

## Invariants (from SPEC.md §V)

- V1: No direct sqlx/rusqlite outside adapter impls
- V2: DB calls never block UI thread
- V3: Single App struct, no shared mutable UI state
- V4: Modes handle keys, main loop dispatches only
- V5: Connections persisted to disk
- V6: Paginated results, never full result set in memory
- V7: Autocomplete triggers after 300ms idle

## Domain Glossary

See `CONTEXT.md` for term definitions (Connection, Backend, Query, Results, Explorer, Mode, Pane, Autocomplete, Status Bar).
