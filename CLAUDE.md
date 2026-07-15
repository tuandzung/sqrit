# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

Local dev wraps `cargo` behind a `justfile`. Use either:

```bash
# raw cargo
cargo build                    # build
cargo run                      # run TUI
cargo test                     # all tests except #[ignore]'d adapter tests
cargo test --test sqlite_adapter
cargo clippy -- -D warnings    # lint
cargo fmt --check              # format check

# just (preferred for dev loop; see ADR 4)
just                           # list recipes
just check                     # fmt --check + clippy + test (pre-push gate)
just it                        # docker compose up + pg/mysql adapter tests
just it-sqlite                 # sqlite adapter only (no docker)
just db-up / db-down           # container lifecycle
```

Integration tests for PostgreSQL and MySQL are `#[ignore]`d and require running databases. The `justfile` + `docker-compose.yml` at the repo root provide them locally on the same ports/credentials as CI (`15432`, `13306`, `sqrit/sqrit`, `sqrit_test`). See `docs/adr/0004-local-integration-runner.md`.

## CI/CD

Three GitHub Actions workflows:

| Workflow | Trigger | What it does |
|----------|---------|--------------|
| `ci.yml` | PR to main | build, clippy (`-D warnings`), fmt check, SQLite tests |
| `integration.yml` | Push to main | full test suite with PG + MySQL via GitHub Services |
| `release.yml` | Tag `v*` | extract changelog, cross-compile 4 targets, publish GitHub Release |

**Release targets**: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` (via `cross` on Ubuntu), `x86_64-apple-darwin`, `aarch64-apple-darwin` (native on macOS).

**Integration test env vars**: `DATABASE_URL=postgres://sqrit:sqrit@localhost:15432/sqrit_test` and `MYSQL_URL=mysql://sqrit:sqrit@localhost:13306/sqrit_test`. Adapter tests default to these URLs when the env vars are unset.

**Release body**: extracted from CHANGELOG.md matching the tag version section (`## [<version>]` … up to the next `## [`).

**Cutting a release**: bump `Cargo.toml` version, add a `## [<version>] - YYYY-MM-DD` section to `CHANGELOG.md`, merge to `main`, then `git tag vX.Y.Z && git push --tags`.

## Architecture

**Three core layers**, all owned by a single `App` struct (no `Arc<Mutex<>>` for UI state — see invariant V3 below):

1. **Database** (`src/db/`) — `Database` trait in `mod.rs`, adapters per backend. All DB ops go through this trait (V1). SQLite (`rusqlite` via `spawn_blocking`), PostgreSQL + MySQL (`sqlx`).

2. **Modes** (`src/mode.rs` + `src/mode/`) — `Mode` enum dispatches `handle_key()` to mode-specific handlers. Every mode implements `handle_key()` (V4). Never match keys in the main loop. Modes: `Picker`, `Explorer`, `QueryNormal`, `QueryInsert`, `Results`, `Command` (vim-style `:` prompt).

3. **Event loop** (`src/app.rs`) — crossterm raw mode + alternate screen. Polls at 100ms, dispatches key events to active mode. DB calls dispatched via `tokio::spawn`, results returned through `mpsc` channel (V2). Schema loads also async. `drain_async_results()` processes results each tick.

**Connection config** (`src/config/mod.rs`) — TOML at `~/.sqrit/connections.toml`. Loaded on startup, saved on mutation (V5).

**Mode borrow pattern**: `let mode = self.mode; mode.handle_key(key, self);` — copies `Mode` (it's `Copy`) to avoid the borrow conflict from calling a method that takes `&mut self` while `self.mode` is borrowed.

**App state fields** — `results: Option<QueryResult>`, `query_status: QueryStatus` (`Idle`/`Running`/`Success`/`Error`), `pending_query: Option<String>`, `results_state: ResultsState`, `editor: EditorBuffer`, `normal_state: NormalState`, `command_buffer: String`, `command_origin: Option<Mode>`.

**SQL tokenizer** (`src/sql.rs`) — `tokenize(sql) -> Vec<Token>` with kinds: `Keyword`, `Type`, `String`, `Comment`, `Number`, `Identifier`, `Operator`, `Punctuation`, `Whitespace`. Editor rendering converts tokens to styled `Span`s.

**Results navigation** (`src/results.rs`) — `ResultsState` tracks `selected_row/col`, `scroll_row`, `visible_rows`. Auto-scrolls when selection exceeds visible area.

## Invariants

Project-level rules; violating one usually means an architectural mistake.

- V1: No direct `sqlx`/`rusqlite` outside adapter impls.
- V2: DB calls never block UI thread (spawn tokio task + mpsc channel).
- V3: Single `App` struct, no shared mutable UI state (no `Arc<Mutex<>>` around UI fields).
- V4: Modes handle keys, main loop dispatches only.
- V5: Connections persisted to disk.
- V6: Paginated results, never full result set in memory.
- V7: Autocomplete triggers after 300ms idle.
- V8: INSERT mode renders a visible terminal cursor that honors viewport scroll.
- V9: Bracketed paste is enabled at startup and disabled at shutdown (including via the panic hook). Multi-line clipboard input is delivered to modes via `ModeHandler::handle_paste`, never as a stream of `Char(c)` events.
- V10: Hint bar bindings come from `ModeHandler::bindings()` only — never inline strings. This keeps the hint bar and the help overlay in sync by construction.

## Decision records

Significant decisions live in `docs/adr/`:

- `0001-async-ratatui-sqlx.md` — TUI + async-runtime + driver choices.
- `0002-database-trait-from-start.md` — `Database` trait as the single DB seam.
- `0003-mode-dispatch-keybinding.md` — flat `Mode` enum, no hierarchical state machine; origin-tracking pattern for transient prompts; v0.2 addendum adds a trait-based dispatch refinement so help-overlay bindings can't drift from handlers.
- `0004-local-integration-runner.md` — `justfile` + `docker-compose.yml` for local integration tests; CI stays on GitHub Services.
- `0005-theme-toml-schema.md` — themes as external TOML in `~/.sqrit/themes/`, five defaults embedded + written on first run (Rose Pine, Tokyo Night, Nord, Gruvbox, Catppuccin Macchiato), `~/.sqrit/config.toml` selects the active theme.
- `0006-per-adapter-query-cancel.md` — DB-level cancel via a `Database::cancel()` trait method; per-adapter native mechanisms (SQLite interrupt, PG `pg_cancel_backend`, MySQL `KILL QUERY`).

## Domain Glossary

See `CONTEXT.md` for term definitions (Connection, Backend, Query, Results, Explorer, Mode, Pane, Autocomplete, Status Bar, Theme, Command Palette, Help Overlay, Cell Viewer, Query History, Filter, Cancel).
