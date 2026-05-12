# sqrit

The lazygit of SQL databases. Connect, query, browse — from the terminal.

## Features

- **Three backends**: SQLite, PostgreSQL, MySQL
- **Vim-lite editor**: normal/insert modes, h/j/k/l, w/b, dd/yy/p, undo
- **SQL highlighting**: keywords, types, strings, comments, numbers
- **3-pane TUI**: Explorer sidebar, Query editor, Results table
- **Connection picker**: filter by name, select to connect
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

#### Results Table
| Key | Action |
|-----|--------|
| `h/j/k/l` or arrows | Navigate cells |
| `PgDn` | Next page of results |
| `PgUp` | Previous page of results |
| `yc` | Copy selected cell |
| `yy` | Copy selected row (TSV) |
| `ya` | Copy all rows with header (TSV) |
| `q` | Back to query editor |

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
git clone https://github.com/user/sqrit.git
cd sqrit
cargo build --release
```

## Development

```bash
cargo test          # run all tests
cargo test --test sqlite_adapter  # single test suite
```

Spec-driven development. See `SPEC.md` for task status and invariants.

## License

MIT
