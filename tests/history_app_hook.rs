mod common;

use sqrit::history::{history_path_for, HistoryStatus, HistoryStore};

#[tokio::test]
async fn executed_query_appends_history_entry() {
    let mut app = common::test_app();
    if let Some(ref mut db) = app.db {
        db.connect().await.unwrap();
    }

    let dir = tempfile::tempdir().unwrap();
    app.sqrit_dir = dir.path().to_path_buf();
    app.active_connection = Some("test".to_string());

    app.pending_query = Some("SELECT 1 AS val".to_string());
    app.execute_pending();
    common::wait_for_query(&mut app, std::time::Duration::from_secs(5)).await;

    let path = history_path_for(&app.sqrit_dir, "test");
    let entries = HistoryStore::new(path).load().unwrap();
    assert_eq!(entries.len(), 1, "exactly one history entry on success");
    assert_eq!(entries[0].sql, "SELECT 1 AS val");
    assert_eq!(entries[0].status, HistoryStatus::Ok);
    assert_eq!(entries[0].rows, Some(1));
}

#[tokio::test]
async fn errored_query_appends_history_entry_with_error_status() {
    let mut app = common::test_app();
    if let Some(ref mut db) = app.db {
        db.connect().await.unwrap();
    }

    let dir = tempfile::tempdir().unwrap();
    app.sqrit_dir = dir.path().to_path_buf();
    app.active_connection = Some("test".to_string());

    app.pending_query = Some("THIS IS NOT SQL".to_string());
    app.execute_pending();
    common::wait_for_query(&mut app, std::time::Duration::from_secs(5)).await;

    let path = history_path_for(&app.sqrit_dir, "test");
    let entries = HistoryStore::new(path).load().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].status, HistoryStatus::Error);
    assert_eq!(entries[0].sql, "THIS IS NOT SQL");
    assert_eq!(
        entries[0].rows, None,
        "errored queries must record rows=None so no stale count leaks"
    );
}

#[tokio::test]
async fn statement_execution_records_only_the_selected_sql() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut app = common::test_app();
    if let Some(ref mut db) = app.db {
        db.connect().await.unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    app.sqrit_dir = dir.path().to_path_buf();
    app.active_connection = Some("test".to_string());
    app.editor
        .insert_str("SELECT 1 AS first; SELECT 2 AS second;");
    for _ in 0..20 {
        app.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
    }
    app.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
    app.handle_key_event(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));

    app.execute_pending();
    common::wait_for_query(&mut app, std::time::Duration::from_secs(5)).await;

    let entries = HistoryStore::new(history_path_for(&app.sqrit_dir, "test"))
        .load()
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].sql, "SELECT 1 AS first;");
}
