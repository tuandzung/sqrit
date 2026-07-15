# ADR 8 — Namespace-aware introspection

**Status:** Accepted
**Date:** 2026-07-15

## Context

v0.2 modeled `SchemaInfo` as flat table and view lists. That shape queried only PostgreSQL's `public` schema, collapsed objects from distinct namespaces, and could not represent materialized views, indexes, triggers, functions, procedures, or sequences.

## Decision

`Database::schema_info()` returns a namespace-rooted result:

```text
SchemaInfo { namespaces: Vec<Namespace> }
Namespace { name, tables, views, materialized_views, indexes, triggers, functions, procedures, sequences }
```

- SQLite returns one namespace with an empty name. It populates tables, views, indexes, and triggers.
- PostgreSQL returns one namespace per user schema and filters `pg_catalog`, `information_schema`, `pg_toast`, `pg_temp_*`, and `pg_toast_temp_*` server-side. It supports every object kind in the model.
- MySQL returns one namespace named by `DATABASE()` and introspects only that database. It populates tables, views, indexes, triggers, functions, and procedures.

Explorer renders `Namespace → Group (Object Kind) → Object → Column`. Columns appear only under tables, views, and materialized views. Explorer omits the namespace row when introspection returns exactly one namespace.

## Alternatives considered

- **Flat objects with a kind tag.** Rejected: it preserves no namespace boundary and produces an unwieldy list when a database has many objects.
- **A trait method for each object kind.** Rejected: callers would coordinate adapter-specific loading, and each new kind would expand the trait.

## Consequences

- **Invariant V11:** `Database::schema_info()` always returns `Vec<Namespace>`, never flat table and view lists.
- `schema_info()` remains one trait call, but adapters execute several introspection queries plus a column query for each table, view, and materialized view. The result is not an atomic metadata snapshot.
- Explorer-generated `SELECT *` queries quote both identifiers through `db::quote`: `"namespace"."object"` for PostgreSQL, `` `namespace`.`object` `` for MySQL, and `"object"` for SQLite's empty namespace.
