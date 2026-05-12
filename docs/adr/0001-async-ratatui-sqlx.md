# ADR 1: Async-first event loop with ratatui + tokio + sqlx

**Status:** Accepted

## Context

sqrit needs a TUI framework and database driver strategy that supports three backends (SQLite, PostgreSQL, MySQL) with non-blocking query execution. Queries can take seconds to return — the UI must remain responsive during execution.

## Decision

- **TUI:** `ratatui` with `crossterm` backend. Immediate mode, full control over render loop.
- **Async runtime:** `tokio` throughout. Single runtime, single event loop.
- **PostgreSQL/MySQL:** `sqlx` — async-native, connection pooling, compile-time query checking available.
- **SQLite:** `rusqlite` (synchronous) wrapped in `tokio::task::spawn_blocking`.
- **Event loop:** One `tokio::select!` branching on: crossterm UI events (from spawned reader task), query results (from DB tasks), and background tasks (autocomplete, schema loading).

## Rationale

- `ratatui` is the de facto Rust TUI standard. Maximum flexibility for custom rendering (results table, schema tree, autocomplete popup).
- `sqlx` is the natural async choice for PG/MySQL — shared trait (`sqlx::Any` or generic over `sqlx::Postgres`/`sqlx::MySql`).
- `rusqlite` is the only mature Rust SQLite binding. It's synchronous by design — `spawn_blocking` is the standard integration pattern with tokio.
- Single `select!` keeps the mental model simple: one event loop, channels from workers.

## Consequences

- SQLite queries have slight latency overhead from thread pool scheduling (negligible in practice).
- The `Database` trait must abstract over both async (`sqlx`) and sync (`rusqlite`) backends — trait methods will be async, SQLite impl uses `spawn_blocking` internally.
- `ratatui` requires manual widget implementation — no built-in tree widget, no popup/modal system, no scrolling text area. All must be built or sourced from ecosystem crates.
