# ADR 6: Per-adapter DB-level query cancel

**Status:** Accepted (planned for v0.2)

## Context

v0.2 adds a "cancel running query" affordance (`<space>z`). The naïve implementation — call `JoinHandle::abort()` on the in-flight tokio task — drops the future on the local side but leaves the actual SQL running on the database server until it completes on its own. The UI tells the user "cancelled" while the server still chews on a 30-second sequential scan. This is the same shape of bug as B1 (silent error swallowing): the system lies to the user.

We need cancellation that the database itself observes. Each backend exposes a different mechanism for this, and there is no neutral abstraction in `sqlx` or `rusqlite`.

## Decision

Add an async method to the `Database` trait:

```rust
#[async_trait]
trait Database {
    // ... existing methods ...
    async fn cancel(&self) -> anyhow::Result<()>;
}
```

Per-adapter implementations:

- **SQLite (`rusqlite`)**: capture the `InterruptHandle` from the connection at `connect()` time. `cancel()` calls `handle.interrupt()`, which sets a flag the `rusqlite` query loop checks between steps. Returns immediately; the in-flight query completes with `SQLITE_INTERRUPT`. Safe to call across threads (the handle is `Send + Sync`).

- **PostgreSQL (`sqlx`)**: at `connect()`, capture the backend process ID from the first acquired connection via the `postgres_protocol`-level `process_id` (exposed by `sqlx::postgres::PgConnection`). `cancel()` opens a side connection from the pool (`pool.acquire()`) and executes `SELECT pg_cancel_backend($1)` with the captured PID. The original query receives a `QueryCanceled` error.

- **MySQL (`sqlx`)**: at `connect()`, capture the connection ID via `SELECT CONNECTION_ID()` on the first acquired connection. `cancel()` opens a side connection and executes `KILL QUERY <id>`. The original query receives a server-side cancellation.

The UI layer:

- `<space>z` calls `db.cancel().await` (spawned on tokio so the UI thread never blocks).
- The cancelled query's future returns an error; the existing `query_id` guard in `App::drain_async_results` already discards results from stale query IDs, so the UI's existing plumbing handles the cleanup without changes.
- Status bar shows `query cancelled`.

## Rationale

- **Honesty over simplicity.** `JoinHandle::abort()` ships in one line; we don't ship it because the user-visible signal would be a lie. The B1 post-mortem explicitly called out silent lies in async paths as a recurring class of bug; this decision is the v0.2 application of that lesson.
- **Trait method over per-call-site wiring.** Every existing adapter already implements the `Database` trait — adding one more async method is the lowest-friction extension. Alternatives like a separate `Cancellable` trait fragment the abstraction for no benefit, since cancellation is a property of every backend we ship.
- **Side connection for PG/MySQL, in-handle interrupt for SQLite.** No commonality to extract — the three mechanisms are different enough that abstraction would only obscure them. Each adapter owns its own pattern.
- **Capture PID/connection-ID at connect time**, not per-query: avoids racing with a query that hasn't issued yet, and PG/MySQL guarantee these IDs are stable for the lifetime of the connection.
- **`anyhow::Result<()>`** rather than a custom `CancelError`: cancel failures are almost always transient (network blip, pool exhausted) and the right response is a status-bar message, not a typed handling chain.

## Consequences

- `Database` trait grows by one method. All three adapter impls update. New mock/test adapters will need to implement it (`tests/v2_async_query.rs` has a fake — needs a no-op `cancel()`).
- PG/MySQL adapters now hold an additional piece of state captured at connect (`backend_pid` / `connection_id`). Adapter struct fields grow. No behavior change when cancel is never invoked.
- Side-connection cancel for PG/MySQL means the pool needs at least 2 connections available. Pool defaults are 10+; not a real constraint, but documented.
- SQLite's `InterruptHandle` only cancels the currently-running statement on that connection. Multi-statement queries (we don't currently support them) would only stop at the next statement boundary.
- Cancel-of-no-running-query is a no-op (server returns "no query to cancel"). UI doesn't need to track query state to gate the keybinding; the safe path is "always try, ignore the not-running case."
- Test coverage: add an integration test per adapter (likely in `tests/<adapter>_adapter.rs` behind `#[ignore]`) that starts a deliberately slow query (`SELECT pg_sleep(30)` for PG, `SELECT SLEEP(30)` for MySQL, recursive CTE for SQLite), invokes `cancel()`, and asserts the future resolves within ~1 second with a cancellation-shaped error.
