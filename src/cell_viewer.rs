use chrono::TimeZone;

use crate::db::types::Value;

/// Which rendering the cell viewer modal is currently showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Raw,
    Formatted,
}

/// Render a cell `value` for display in the cell viewer.
///
/// - `Raw` always defers to `Value`'s `Display` impl.
/// - `Formatted` applies type-aware rendering (JSON pretty-print, hex dump,
///   local-time conversion), falling back to the same string as `Raw` when
///   no specialization applies.
pub fn format_value(value: &Value, column_type: Option<&str>, view: ViewMode) -> String {
    match view {
        ViewMode::Raw => value.to_string(),
        ViewMode::Formatted => formatted(value, column_type),
    }
}

fn formatted(value: &Value, column_type: Option<&str>) -> String {
    match value {
        Value::Text(s) => {
            if is_datetime_column(column_type) {
                if let Some(rendered) = format_datetime(s) {
                    return rendered;
                }
            }
            format_text(s)
        }
        Value::Blob(bytes) => format_blob(bytes),
        _ => value.to_string(),
    }
}

/// Lowercased SQL column-type names that mean "the text is a date or
/// timestamp". Conservative: only the common DB-portable ones.
fn is_datetime_column(column_type: Option<&str>) -> bool {
    let Some(t) = column_type else { return false };
    let t = t.to_lowercase();
    matches!(
        t.as_str(),
        "date" | "datetime" | "timestamp" | "timestamptz" | "timestamp with time zone"
    )
}

/// Best-effort chrono parse → local-zone re-render. Returns `None` when the
/// text doesn't parse as any of the supported shapes, so the caller can fall
/// back to the raw string rather than silently mangle content.
fn format_datetime(s: &str) -> Option<String> {
    let s = s.trim();
    // RFC 3339 (`2026-05-21T03:00:00Z` / `…+02:00`)
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        let local: chrono::DateTime<chrono::Local> = dt.with_timezone(&chrono::Local);
        return Some(local.format("%Y-%m-%d %H:%M:%S %z").to_string());
    }
    // `YYYY-MM-DD HH:MM:SS` (no zone) — interpret as local
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        let local = chrono::Local
            .from_local_datetime(&naive)
            .single()
            .unwrap_or_else(|| chrono::Local.from_utc_datetime(&naive));
        return Some(local.format("%Y-%m-%d %H:%M:%S %z").to_string());
    }
    // Naive date
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d.format("%Y-%m-%d").to_string());
    }
    None
}

/// 16-bytes-per-row hex dump with an 8-char hex address column.
/// Final line is not padded — the address tells the reader where it ended.
fn format_blob(bytes: &[u8]) -> String {
    bytes
        .chunks(16)
        .enumerate()
        .map(|(i, chunk)| {
            let addr = i * 16;
            let hex = chunk
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{:08x}  {}", addr, hex)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Detect JSON object/array text and pretty-print it. Falls back to the raw
/// string when the input doesn't parse — we never lie about content.
fn format_text(s: &str) -> String {
    let trimmed = s.trim_start();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return s.to_string();
    }
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| s.to_string()),
        Err(_) => s.to_string(),
    }
}
