# ADR 2: Database trait abstraction from day one

**Status:** Accepted

## Context

sqrit supports three database backends (SQLite, PostgreSQL, MySQL) all equally first-class. Two options: define a shared `Database` trait upfront, or build SQLite concrete first and extract the trait when adding the second backend.

## Decision

Define a generic `Database` trait from the start. All backends implement it.

## Rationale

- Three first-class backends means the abstraction pays for itself immediately — no "single concrete" phase.
- Designing the trait upfront forces clarity on what schema introspection, query execution, and connection lifecycle look like across all three databases.
- The trait will iterate, but the cost of iterating on a trait is lower than unifying three divergent concrete implementations.

## Consequences

- Trait design will be informed by the least common denominator. SQLite has no concept of host/port/user/password — the trait's `connect()` must accommodate both file-path and network connection params (likely via an enum or config struct).
- PostgreSQL and MySQL have different type systems — result rows will need a unified representation (likely `Vec<HashMap<String, Value>>` or a custom `Row` type).
- Schema introspection queries differ significantly across backends (`sqlite_master` vs `information_schema`). Each adapter owns its introspection SQL.
