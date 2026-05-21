use sqrit::history::{history_path_for, HistoryEntry, HistoryStatus, HistoryStore};

#[test]
fn append_then_load_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let store = HistoryStore::new(dir.path().join("hist.jsonl"));
    let entry = HistoryEntry {
        ts: "2026-05-21T08:13:02Z".into(),
        sql: "SELECT 1".into(),
        duration_ms: 18,
        status: HistoryStatus::Ok,
        rows: Some(42),
    };

    store.append(&entry).unwrap();

    let loaded = store.load().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].sql, "SELECT 1");
    assert_eq!(loaded[0].duration_ms, 18);
    assert_eq!(loaded[0].status, HistoryStatus::Ok);
    assert_eq!(loaded[0].rows, Some(42));
}

#[test]
fn ring_buffer_caps_at_500() {
    let dir = tempfile::tempdir().unwrap();
    let store = HistoryStore::new(dir.path().join("hist.jsonl"));

    for i in 0..501 {
        store
            .append(&HistoryEntry {
                ts: "2026-05-21T08:13:02Z".into(),
                sql: format!("SELECT {}", i),
                duration_ms: 1,
                status: HistoryStatus::Ok,
                rows: Some(0),
            })
            .unwrap();
    }

    let loaded = store.load().unwrap();
    assert_eq!(loaded.len(), 500, "ring buffer caps file at 500 entries");
    assert_eq!(
        loaded[0].sql, "SELECT 1",
        "oldest entry (SELECT 0) must be dropped on overflow"
    );
    assert_eq!(loaded[499].sql, "SELECT 500", "newest entry preserved");
}

#[test]
fn path_for_sanitizes_connection_name() {
    let base = std::path::PathBuf::from("/root/.sqrit");

    let plain = history_path_for(&base, "prod");
    assert_eq!(plain, base.join("history/prod.jsonl"));

    let messy = history_path_for(&base, "my prod/db");
    assert_eq!(messy, base.join("history/my-prod-db.jsonl"));

    let dots = history_path_for(&base, "../etc/passwd");
    assert_eq!(
        dots,
        base.join("history/etc-passwd.jsonl"),
        "path traversal must be neutralized"
    );

    let empty = history_path_for(&base, "");
    assert_eq!(
        empty,
        base.join("history/unnamed-conn.jsonl"),
        "empty name must fall back to a non-empty basename"
    );

    let all_punct = history_path_for(&base, "///");
    assert_eq!(
        all_punct,
        base.join("history/unnamed-conn.jsonl"),
        "all-non-alnum name must fall back rather than produce `.jsonl`"
    );
}
