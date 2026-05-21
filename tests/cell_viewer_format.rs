use sqrit::cell_viewer::{format_value, ViewMode};
use sqrit::db::types::Value;

#[test]
fn raw_value_uses_display_impl() {
    assert_eq!(format_value(&Value::Integer(42), None, ViewMode::Raw), "42");
    assert_eq!(
        format_value(&Value::Text("hi".to_string()), None, ViewMode::Raw),
        "hi"
    );
    assert_eq!(format_value(&Value::Null, None, ViewMode::Raw), "NULL");
    assert_eq!(
        format_value(&Value::Boolean(true), None, ViewMode::Raw),
        "true"
    );
}

// --- Slice 2: Formatted pretty-prints JSON object/array text ---

#[test]
fn formatted_pretty_prints_json_object() {
    let v = Value::Text(r#"{"a":1,"b":[2,3]}"#.to_string());

    let out = format_value(&v, None, ViewMode::Formatted);

    assert!(out.contains("\n"), "pretty JSON must contain newlines");
    assert!(out.contains("  "), "pretty JSON must use indentation");
    // Sanity: keys preserved in order via serde_json default
    assert!(out.contains("\"a\": 1"));
    assert!(out.contains("\"b\""));
}

#[test]
fn formatted_pretty_prints_json_array() {
    let v = Value::Text("[1,2,3]".to_string());

    let out = format_value(&v, None, ViewMode::Formatted);

    assert!(out.contains("\n"));
    assert!(out.contains("1"));
    assert!(out.contains("3"));
}

#[test]
fn formatted_non_json_text_falls_back_to_raw() {
    let v = Value::Text("plain text".to_string());

    let out = format_value(&v, None, ViewMode::Formatted);

    assert_eq!(out, "plain text");
}

#[test]
fn formatted_invalid_json_text_falls_back_to_raw() {
    // Starts with `{` but isn't valid JSON — must not crash, must not lie.
    let v = Value::Text("{ not json".to_string());

    let out = format_value(&v, None, ViewMode::Formatted);

    assert_eq!(out, "{ not json");
}

// --- Slice 3: Blob renders as a hex dump (16 bytes/line, address column) ---

#[test]
fn formatted_blob_renders_hex_dump_with_address_column() {
    let bytes: Vec<u8> = (0..20).collect(); // 0x00..0x13 — 20 bytes → two rows
    let v = Value::Blob(bytes);

    let out = format_value(&v, None, ViewMode::Formatted);

    // First row: address 0x00000000 + 16 bytes
    let first_line = out.lines().next().unwrap();
    assert!(
        first_line.starts_with("00000000"),
        "first line should start with address column, got: {:?}",
        first_line
    );
    assert!(first_line.contains("00 01 02 03"));
    assert!(first_line.contains("0e 0f"));

    // Second row: address 0x00000010 + 4 remaining bytes (no padding required)
    let second_line = out.lines().nth(1).unwrap();
    assert!(
        second_line.starts_with("00000010"),
        "second line address should be 16, got: {:?}",
        second_line
    );
    assert!(second_line.contains("10 11 12 13"));
}

#[test]
fn formatted_empty_blob_still_renders_string() {
    let out = format_value(&Value::Blob(vec![]), None, ViewMode::Formatted);
    // Don't care about exact content; just that it doesn't panic and returns
    // a (possibly empty) string.
    let _ = out.len();
}

// --- Slice 4: Date/timestamp text re-rendered via chrono ---

#[test]
fn formatted_timestamp_with_tz_renders_with_space_and_offset() {
    // RFC3339 with `T` separator + `Z` offset is the canonical SQL serialized form.
    // Formatted view must route through chrono → local-zone re-render, which
    // (a) replaces `T` with a space and (b) makes the numeric offset explicit.
    // Both differences are observable regardless of the test host's timezone.
    let v = Value::Text("2026-05-21T03:00:00Z".to_string());

    let out = format_value(&v, Some("timestamptz"), ViewMode::Formatted);

    assert!(
        !out.contains('T'),
        "formatted timestamp should use a space separator, got: {:?}",
        out
    );
    assert!(
        out.contains('+') || out.contains('-'),
        "formatted timestamp should include a numeric offset, got: {:?}",
        out
    );
    assert!(
        chrono::DateTime::parse_from_str(&out, "%Y-%m-%d %H:%M:%S %z").is_ok()
            || chrono::DateTime::parse_from_str(&out, "%Y-%m-%d %H:%M:%S %:z").is_ok(),
        "expected a `YYYY-MM-DD HH:MM:SS ±HHMM`-shaped output, got: {:?}",
        out
    );
}

#[test]
fn formatted_non_date_column_with_date_text_still_renders_raw() {
    // No hint column type → no special handling (string just falls through raw).
    let v = Value::Text("2026-05-21T03:00:00Z".to_string());

    let out = format_value(&v, None, ViewMode::Formatted);

    assert_eq!(out, "2026-05-21T03:00:00Z");
}

#[test]
fn formatted_unparseable_date_falls_back_to_raw() {
    let v = Value::Text("not a date".to_string());

    let out = format_value(&v, Some("timestamp"), ViewMode::Formatted);

    assert_eq!(out, "not a date");
}
