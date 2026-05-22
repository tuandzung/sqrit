use crate::db::types::QueryResult;

pub fn format_cell(result: &QueryResult, row: usize, col: usize) -> Option<String> {
    let col = result.columns.get(col)?;
    let r = result.rows.get(row)?;
    r.get(&col.name).map(|v| v.to_string())
}

pub fn format_row(result: &QueryResult, row: usize) -> Option<String> {
    let r = result.rows.get(row)?;
    let vals: Vec<String> = result
        .columns
        .iter()
        .map(|c| r.get(&c.name).map(|v| v.to_string()).unwrap_or_default())
        .collect();
    Some(vals.join("\t"))
}

pub fn format_all(result: &QueryResult) -> String {
    let header = result.column_names().join("\t");
    let rows: Vec<String> = result
        .rows
        .iter()
        .enumerate()
        .map(|(i, _)| format_row(result, i).unwrap_or_default())
        .collect();
    std::iter::once(header)
        .chain(rows)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_csv(result: &QueryResult) -> String {
    let header = result
        .columns
        .iter()
        .map(|c| csv_escape(&c.name))
        .collect::<Vec<_>>()
        .join(",");
    let rows: Vec<String> = result
        .rows
        .iter()
        .map(|r| {
            result
                .columns
                .iter()
                .map(|c| {
                    r.get(&c.name)
                        .map(|v| csv_escape(&v.to_string()))
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect();
    std::iter::once(header)
        .chain(rows)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_json(result: &QueryResult) -> String {
    let objects: Vec<String> = result
        .rows
        .iter()
        .map(|r| {
            let pairs: Vec<String> = result
                .columns
                .iter()
                .map(|c| {
                    let val = r.get(&c.name).map(|v| v.to_string()).unwrap_or_default();
                    format!("{}:{}", json_escape(&c.name), json_escape(&val))
                })
                .collect();
            format!("{{{}}}", pairs.join(","))
        })
        .collect();
    format!("[{}]", objects.join(","))
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn json_escape(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Long-lived clipboard writer. The implementation chooses a backend
/// lazily on the first `copy()`:
///
/// * **Linux/Wayland (`WAYLAND_DISPLAY` set, `wl-copy` available)**:
///   shells out to `wl-copy`. arboard 3.x's Wayland path is silent on
///   several modern compositors (verified on niri with the
///   `examples/clipboard_repro` harness: `set_text()` returns Ok but the
///   compositor never receives the selection). `wl-copy` daemonises
///   itself so the selection survives `wl-copy`'s parent process exit.
/// * **Otherwise (X11, macOS, Windows)**: owns a single
///   `arboard::Clipboard` for the lifetime of the app so X11's
///   selection-serve thread stays alive between user copies. Dropping
///   `Clipboard` after every `set_text` kills that thread before the
///   user's clipboard manager can sample the new contents (arboard logs
///   `"Clipboard was dropped very quickly after writing (0ms); clipboard
///   managers may not have seen the contents."` in that case).
///
/// Init is lazy so headless test environments (no `$DISPLAY` / no
/// `WAYLAND_DISPLAY` / no `wl-copy`) don't pay the connection cost or
/// fail at startup — they observe `copy()` returning an error. The
/// backend choice and a `failed` latch are cached so repeated copies
/// don't re-probe.
pub struct ClipboardWriter {
    backend: Backend,
    /// Number of times the backend has been probed (arboard
    /// `Clipboard::new()` or `wl-copy` selection). Stays at most `1` for
    /// the lifetime of one `ClipboardWriter`; tests assert this invariant.
    init_attempts: usize,
}

enum Backend {
    /// Backend not chosen yet — first `copy()` will probe.
    Pending,
    /// `wl-copy` CLI is available on `$PATH` and we're under Wayland.
    WlCopy,
    /// arboard handle constructed; reuse across copies.
    Arboard(arboard::Clipboard),
    /// Both paths failed at probe time; latched so we don't re-probe.
    Failed,
}

impl ClipboardWriter {
    pub fn new() -> Self {
        Self {
            backend: Backend::Pending,
            init_attempts: 0,
        }
    }

    pub fn init_attempts(&self) -> usize {
        self.init_attempts
    }

    /// Returns `true` once a usable backend has been chosen.
    pub fn is_initialized(&self) -> bool {
        matches!(self.backend, Backend::WlCopy | Backend::Arboard(_))
    }

    pub fn copy(&mut self, text: &str) -> anyhow::Result<()> {
        if matches!(self.backend, Backend::Pending) {
            self.init_attempts += 1;
            self.backend = probe_backend();
        }
        match &mut self.backend {
            Backend::WlCopy => wl_copy(text),
            Backend::Arboard(c) => {
                c.set_text(text)?;
                Ok(())
            }
            Backend::Failed => Err(anyhow::anyhow!("clipboard unavailable on this display")),
            Backend::Pending => unreachable!("probe sets backend above"),
        }
    }
}

/// Decide which backend to use. Probes `wl-copy --version` once to confirm
/// the binary is on `$PATH`; if it is, the actual selection writes go
/// through `wl_copy()`. Falls back to arboard for X11 / macOS / Windows
/// and when `wl-copy` is not installed.
fn probe_backend() -> Backend {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && wl_copy_available() {
        return Backend::WlCopy;
    }
    match arboard::Clipboard::new() {
        Ok(c) => Backend::Arboard(c),
        Err(_) => Backend::Failed,
    }
}

fn wl_copy_available() -> bool {
    use std::process::{Command, Stdio};
    Command::new("wl-copy")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn wl_copy(text: &str) -> anyhow::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    // CRITICAL: stderr MUST be `Stdio::null()`, not `Stdio::piped()`.
    // `wl-copy` daemonises a child that inherits its open file
    // descriptors and serves the selection until something else
    // overwrites the clipboard — effectively forever in a TUI session.
    // If we pipe stderr to ourselves, `child.wait()` returns when the
    // parent exits, but `child.wait_with_output()` would block waiting
    // for the inherited stderr pipe to EOF, which the daemon never
    // closes. That hangs `yy` / `yc` / `ya` on every copy.
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("wl-copy exited with status {}", status);
    }
    Ok(())
}

impl Default for ClipboardWriter {
    fn default() -> Self {
        Self::new()
    }
}
