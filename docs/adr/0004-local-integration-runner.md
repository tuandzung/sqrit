# ADR 4: Local integration runner with Just + docker compose

**Status:** Accepted

## Context

Adapter tests for PostgreSQL and MySQL are gated behind `#[ignore]` and require running databases (`DATABASE_URL` / `MYSQL_URL`). CI provides them via GitHub Actions `services:` (see `.github/workflows/integration.yml`), but locally a contributor has to remember the exact ports (15432 / 13306), credentials (sqrit/sqrit), db name (`sqrit_test`), and the cargo invocation (`--test pg_adapter --test mysql_adapter -- --include-ignored`). Friction discourages running integration tests before pushing.

## Decision

Introduce a local-only runner consisting of:

- **`docker-compose.yml`** at repo root — declares `postgres:16` and `mysql:8` services with the same ports, credentials, and database name as CI, plus healthchecks identical to the CI service definitions.
- **`justfile`** at repo root — wraps `docker compose` and `cargo test` in named recipes. Targets: `db-up`, `db-down`, `db-logs`, `db-psql`, `db-mysql`, `it`, `it-clean`, `it-pg`, `it-mysql`, `it-sqlite`, plus dev shortcuts `test`, `lint`, `fmt`, `check`.
- **Lifecycle**: `db-up` is idempotent (`compose up -d --wait`); `it` brings DBs up if needed, runs tests, and leaves containers running for fast iteration; `it-clean` is the one-shot CI-mirror. `db-down` runs `compose down -v` to wipe volumes for a clean restart.
- **Match CI semantics exactly**: ports `15432:5432` / `13306:3306`, user `sqrit`, password `sqrit`, db `sqrit_test`. Existing adapter test defaults already match — no env wiring needed.

CI (`integration.yml`) is **not** migrated to this runner. It keeps GitHub Actions `services:` because they boot faster on hosted runners than a compose stack.

## Rationale

- **Just over Make**: Just's recipe model (per-recipe shells, `--list` discoverability, no tab-indentation footguns, native cross-platform) fits a small Rust project better than Make. Make would also work, but the user explicitly preferred Just.
- **docker compose over ad-hoc `docker run`**: Compose v2 with healthchecks + `--wait` gives identical semantics to the CI `services:` block in one declarative file. Ad-hoc `docker run` would scatter env/port/health config across justfile recipes.
- **docker compose over testcontainers**: Testcontainers would eliminate the runner entirely, but requires rewriting every `#[ignore]`d adapter test to start its own container and consume a dynamic port. Larger refactor; out of scope for the contributor-ergonomics goal. Worth reconsidering if test isolation requirements change.
- **Match-CI defaults over .env file**: Adapter tests already default to the CI URLs via `unwrap_or_else`. Matching exactly means `cargo test` works with zero env setup once containers are up. A `.env` file would add a moving part for no current benefit.
- **Idempotent up + leave running**: The dev loop is `just it` → edit → `just it` again. Compose `up -d --wait` is effectively a no-op when containers are already healthy (~1s). Tearing down between runs would add ~15-30s of mysql:8 init each time.
- **Containers stay up on test failure** (`it-clean` chains plain `db-up && cargo test && db-down`): leaves DB state available for `docker compose exec` inspection if tests fail. User must `just db-down` manually after debugging — accepted tradeoff for simpler recipes.
- **CI stays on GitHub Services**: GitHub Actions services start in parallel with the job; compose adds startup overhead. The runners are environment-specific and the parity risk is small (both paths share the same ports/creds/test invocation). Migration can happen later if drift bites.

## Consequences

- Contributors need `docker` (with `docker compose` v2.1+) and `just` installed locally. Prereqs documented in README.
- Two paths to integration testing — local (compose) and CI (Services). Drift is possible. Mitigation: both use the same env URL format and the same `cargo test` invocation; any divergence will surface as a test that passes one but fails the other.
- `down -v` wipes volumes by default, so `db-down` is destructive. This is the expected meaning of "tear down" and is documented in the README and `just --list`.
- Compose v1 (`docker-compose` hyphen) is not supported. Compose v2 ships with Docker Desktop and modern Linux Docker installs (2022+).
- New `justfile` becomes the canonical entry point for dev tasks. CLAUDE.md's raw cargo commands remain valid; `just` wraps them.
