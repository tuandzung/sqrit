# sqrit task runner. See docs/adr/0004-local-integration-runner.md.
# Run `just` (no args) to list available recipes.

# Default recipe lists everything.
default:
    @just --list

# --- Database lifecycle ---------------------------------------------------

# Start postgres + mysql containers, wait until both report healthy.
db-up:
    docker compose up -d --wait

# Stop containers AND remove named volumes (clean wipe).
db-down:
    docker compose down -v

# Tail container logs (Ctrl-C to detach).
db-logs:
    docker compose logs -f

# Open a psql shell against the local postgres container.
db-psql:
    docker compose exec postgres psql -U sqrit -d sqrit_test

# Open a mysql shell against the local mysql container.
db-mysql:
    docker compose exec mysql mysql -usqrit -psqrit sqrit_test

# --- Integration tests ----------------------------------------------------

# Run full integration suite (pg + mysql adapters); containers stay up.
it: db-up
    cargo test --locked --test pg_adapter --test mysql_adapter -- --include-ignored

# One-shot CI-style run: up + test + down. Containers stay up on failure.
it-clean:
    just db-up
    cargo test --locked --test pg_adapter --test mysql_adapter -- --include-ignored
    just db-down

# Run only the postgres adapter tests against a freshly-started pg container.
it-pg:
    docker compose up -d --wait postgres
    cargo test --locked --test pg_adapter -- --include-ignored

# Run only the mysql adapter tests against a freshly-started mysql container.
it-mysql:
    docker compose up -d --wait mysql
    cargo test --locked --test mysql_adapter -- --include-ignored

# Run sqlite adapter tests (no docker required).
it-sqlite:
    cargo test --locked --test sqlite_adapter

# --- Dev shortcuts --------------------------------------------------------

# Run unit + sqlite tests (no docker required).
test:
    cargo test --locked

# Lint with clippy, deny all warnings.
lint:
    cargo clippy --tests -- -D warnings

# Format the entire workspace.
fmt:
    cargo fmt

# Pre-push gate: fmt check + lint + unit tests (no docker).
check:
    cargo fmt --check
    cargo clippy --tests -- -D warnings
    cargo test --locked
