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

/// Long-lived clipboard writer. Owns a single `arboard::Clipboard` for the
/// lifetime of the app so that X11's selection-serve thread stays alive
/// between user copies — dropping the `Clipboard` after every `set_text`
/// kills that thread before the user's clipboard manager can sample the
/// new contents (arboard logs the warning
/// `"Clipboard was dropped very quickly after writing (0ms); clipboard
/// managers may not have seen the contents."` in that case).
///
/// Init is lazy so headless test environments (no `$DISPLAY` / no
/// `WAYLAND_DISPLAY`) don't pay the connection cost or fail at startup —
/// they simply observe `copy()` returning the underlying arboard error.
/// `init_failed` latches the first failure so we don't reattempt
/// (and re-log) on every keystroke.
pub struct ClipboardWriter {
    inner: Option<arboard::Clipboard>,
    init_failed: bool,
    /// Number of times we attempted `Clipboard::new()`. Exposed for
    /// regression tests that lock the "one handle for the whole app"
    /// invariant — if a future refactor reverts to per-call construction,
    /// this counter will tick up and the test will fail.
    init_attempts: usize,
}

impl ClipboardWriter {
    pub fn new() -> Self {
        Self {
            inner: None,
            init_failed: false,
            init_attempts: 0,
        }
    }

    /// How many times the underlying `arboard::Clipboard` has been
    /// constructed. Stays at most `1` for the lifetime of a single
    /// `ClipboardWriter` even across repeated `copy()` calls.
    pub fn init_attempts(&self) -> usize {
        self.init_attempts
    }

    /// Returns `true` once a real `arboard::Clipboard` handle is in hand.
    pub fn is_initialized(&self) -> bool {
        self.inner.is_some()
    }

    pub fn copy(&mut self, text: &str) -> anyhow::Result<()> {
        if self.inner.is_none() {
            if self.init_failed {
                return Err(anyhow::anyhow!("clipboard unavailable on this display"));
            }
            self.init_attempts += 1;
            match arboard::Clipboard::new() {
                Ok(c) => self.inner = Some(c),
                Err(e) => {
                    self.init_failed = true;
                    return Err(e.into());
                }
            }
        }
        let clipboard = self
            .inner
            .as_mut()
            .expect("clipboard handle just stored above");
        clipboard.set_text(text)?;
        Ok(())
    }
}

impl Default for ClipboardWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Backwards-compatible one-shot copy. New code should reuse a single
/// [`ClipboardWriter`] held on `App` so the X11 serve thread lives across
/// copies; this free function constructs and drops a fresh `Clipboard`
/// every call and will trigger arboard's "dropped quickly" warning on
/// Linux/X11.
#[deprecated(
    note = "use `ClipboardWriter::copy` via `App::clipboard_writer` so the X11 serve thread survives between copies"
)]
pub fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}
