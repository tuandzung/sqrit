# sqrit

The lazygit of SQL databases. Connect, query, browse — from the terminal.

## Features

- **Three backends**: SQLite, PostgreSQL, MySQL
- **Vim-lite editor**: normal/insert modes, h/j/k/l, w/b, dd/yy/p, undo
- **SQL highlighting**: keywords, types, strings, comments, numbers
- **3-pane TUI**: Explorer sidebar, Query editor, Results table
- **Connection picker**: filter by name, select to connect
- **Non-blocking I/O**: async DB calls via Tokio — UI never freezes during queries or schema loads
- **Themes**: five bundled palettes (Rose Pine, Tokyo Night, Nord, Gruvbox, Catppuccin Macchiato), live picker via `<space>t`, plus any user TOML in `~/.sqrit/themes/`
- **Space command palette**: `<space>` arms a one-shot palette for top-level actions (maximize, theme, quit, history, cancel, disconnect)
- **Help overlay**: `?` lists the active mode's keybindings
- **Fuzzy row filter**: `/` in Results live-filters loaded rows by subsequence match across all columns; matched chars highlighted
- **Query history**: every executed query persisted to `~/.sqrit/history/<connection>.jsonl`, picker via `<space>h`
- **Cell viewer**: `v` in Results opens a modal with raw/formatted toggle (pretty JSON, hex blobs, timezone-aware dates)
- **Query cancel**: `<space>z` cancels the running query at the DB level (SQLite interrupt, PG `pg_cancel_backend`, MySQL `KILL QUERY`)
- **Bracketed paste**: multi-line clipboard input survives Insert mode (no more LF → `j`)
- **Cross-platform clipboard**: native `wl-copy` on Linux/Wayland; arboard everywhere else
- **Zero-config UX**: run `sqrit`, pick a connection, go

## Install

```bash
cargo install --path .
```

## Usage

```bash
sqrit
```

On first run, the connection picker appears. Connections are stored in `~/.sqrit/connections.toml`.

### Key Bindings

Press `?` in any non-insert mode for a live help overlay listing the active mode's keybindings.

#### Connection Picker
| Key | Action |
|-----|--------|
| `j/k` or Up/Down | Move selection |
| Type | Filter connections |
| Enter | Connect |
| Backspace | Clear filter |
| `q` | Quit |

#### Explorer
| Key | Action |
|-----|--------|
| `j/k` or Up/Down | Navigate items |
| Enter | Expand/collapse table or view |
| `s` | `SELECT * FROM <table> LIMIT 100` |
| `q` | Back to query editor |

#### Query Editor — Normal Mode
| Key | Action |
|-----|--------|
| `i` | Enter insert mode |
| `h/j/k/l` | Move cursor |
| `w/b` | Word forward/backward |
| `0/$` | Line start/end |
| `gg/G` | File top/bottom |
| `x` | Delete char |
| `dd` | Delete line |
| `yy` | Yank line |
| `p` | Paste below |
| `u` | Undo |
| Enter | Execute query |

#### Query Editor — Insert Mode
| Key | Action |
|-----|--------|
| Esc | Back to normal mode |
| Ctrl+Enter | Execute query |
| Enter | New line |
| Arrow keys, Home/End | Navigation |
| Tab | Accept autocomplete suggestion |
| Paste | Bracketed-paste-aware (multi-line clipboard works) |

#### Results Table
| Key | Action |
|-----|--------|
| `h/j/k/l` or arrows | Navigate cells |
| `PgDn` | Next page of results |
| `PgUp` | Previous page of results |
| `yc` | Copy selected cell |
| `yy` | Copy selected row (TSV) |
| `ya` | Copy all rows with header (TSV) |
| `v` | Open cell viewer modal |
| `/` | Open fuzzy row filter (matched chars highlighted) |
| `,c` | Clear active filter |
| `q` | Back to query editor |

#### Cell Viewer
| Key | Action |
|-----|--------|
| `Tab` | Toggle raw ↔ formatted (JSON pretty, hex blob, dated text) |
| `y` | Copy displayed string to clipboard |
| `j/k` | Scroll |
| `Esc` | Close |

#### Space Command Palette
A leading `<space>` from any non-insert/non-picker mode arms a one-shot palette. The next key dispatches:

| Key | Action |
|-----|--------|
| `<space>f` | Maximize / restore focused pane |
| `<space>t` | Theme picker (live preview, Enter persists, Esc reverts) |
| `<space>h` | Query history picker |
| `<space>z` | Cancel running query |
| `<space>c` | Back to connection picker (keeps db) |
| `<space>x` | Disconnect (clears db/schema/active connection) |
| `<space>q` | Quit |

## Configuration

Connections are defined in `~/.sqrit/connections.toml`:

```toml
[[connections]]
name = "my-sqlite"
db_type = "sqlite"
file_path = "/path/to/database.db"

[[connections]]
name = "my-postgres"
db_type = "postgres"
host = "localhost"
port = 5432
username = "user"
password = "pass"
database = "mydb"

[[connections]]
name = "my-mysql"
db_type = "mysql"
host = "localhost"
port = 3306
username = "user"
password = "pass"
database = "mydb"
```

## Build from Source

```bash
git clone https://github.com/tuandzung/sqrit.git
cd sqrit
cargo build --release
```

## Development

Raw cargo:

```bash
cargo test          # run all tests
cargo test --test sqlite_adapter  # single test suite
```

### Local Integration Tests

Postgres and MySQL adapter tests are `#[ignore]`d by default and require
running databases. A `justfile` + `docker-compose.yml` at the repo root
provide a one-command local runner.

**Prereqs**: [Docker](https://docs.docker.com/engine/install/) (with
`docker compose` v2) and [just](https://just.systems/man/en/chapter_4.html).

```bash
just              # list all recipes
just it           # start postgres+mysql, run integration tests, leave containers up
just it-clean     # one-shot: up, test, down (mirrors CI)
just it-pg        # postgres adapter tests only
just it-mysql     # mysql adapter tests only
just it-sqlite    # sqlite adapter tests (no docker)
just db-up        # start containers (idempotent)
just db-down      # stop and wipe volumes
just check        # fmt --check + clippy + unit tests (pre-push gate)
```

Local ports/credentials match CI: postgres on `15432`, mysql on `13306`,
user/password `sqrit/sqrit`, database `sqrit_test`. See
[ADR 4](docs/adr/0004-local-integration-runner.md) for design notes.

## Architecture

Single `App` struct owns all state. Three core layers:

1. **Database** (`src/db/`) — `Database` trait with adapters per backend. All DB ops go through this trait. Async via `tokio::spawn` + `mpsc` channel — UI never blocks on DB calls.

2. **Modes** (`src/mode.rs` + `src/mode/`) — flat `Mode` enum dispatches via a `ModeHandler { dispatch, bindings, handle_paste }` trait. Help overlay reads `bindings()` from the same impl block as the dispatch, so a new key without a help entry is a one-file omission PR review catches. Modes: Picker, Explorer, QueryNormal, QueryInsert, Results, ResultsFilter, HistoryPicker, ThemePicker, Help, CellViewer.

3. **Event loop** (`src/app.rs`) — 100ms poll loop. Spawns async DB tasks, drains results via `mpsc` channel. Connection happens async on picker selection — adapter created, connected, and schema loaded in a single spawned task.

**Async result flow**: `AsyncResult` enum carries `QueryDone`, `Connected`, and `ConnectFailed` messages through the channel. `drain_async_results()` processes them each tick. The `query_id` counter prevents stale result overwrites.

Architectural decisions are recorded in [`docs/adr/`](docs/adr/).

## License

MIT
