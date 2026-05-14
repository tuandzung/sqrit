mod common;

use sqrit::app::{App, QueryStatus};

fn make_connected_app() -> App {
    common::test_app()
}

// V2: execute_pending must not block — spawns task, returns immediately
#[tokio::test]
async fn execute_pending_returns_immediately() {
    let mut app = make_connected_app();
    if let Some(ref mut db) = app.db {
        db.connect().await.unwrap();
    }

    app.editor.insert_str("SELECT 1 AS val");
    app.pending_query = Some(app.editor.text());

    // execute_pending is sync — spawns task and returns immediately
    app.execute_pending();
    assert!(app.pending_query.is_none());
    assert_eq!(app.query_status, QueryStatus::Running);
    // Results not yet available — task is running
    assert!(app.results.is_none());

    // Yield to let the spawned task complete
    common::wait_for_query(&mut app, std::time::Duration::from_secs(5)).await;

    assert_eq!(app.query_status, QueryStatus::Success);
    assert!(app.results.is_some());
    let result = app.results.unwrap();
    assert_eq!(result.columns, vec!["val".to_string()]);
    assert_eq!(result.rows.len(), 1);
}

// V2: query status transitions Running → Error on bad SQL
#[tokio::test]
async fn execute_pending_error_via_channel() {
    let mut app = make_connected_app();
    if let Some(ref mut db) = app.db {
        db.connect().await.unwrap();
    }

    app.pending_query = Some("INVALID SQL !!@@".to_string());
    app.execute_pending();

    assert_eq!(app.query_status, QueryStatus::Running);
    assert!(app.results.is_none());

    common::wait_for_query(&mut app, std::time::Duration::from_secs(5)).await;

    assert!(matches!(app.query_status, QueryStatus::Error(_)));
    assert!(app.results.is_none());
}

// V2: connect + schema load via async task (picker flow)
#[tokio::test]
async fn connect_and_schema_load_via_async_task() {
    let mut app = make_connected_app();
    // Add a table so schema is non-empty
    {
        let mut db = app.db.as_ref().unwrap().clone_box();
        db.connect().await.unwrap();
        db.execute("CREATE TABLE test_v2 (id INTEGER PRIMARY KEY, name TEXT)").await.unwrap();
    }

    // Replace with fresh unconnected adapter (simulates picker)
    app.db = Some(Box::new(sqrit::db::sqlite::SqliteAdapter::new(":memory:")));
    app.pending_schema_load = true;

    // Simulate the event loop connect + schema load path
    if app.pending_schema_load {
        if let Some(ref db) = app.db {
            let mut db = db.clone_box();
            let tx = app.async_tx.clone();
            app.pending_schema_load = false;
            tokio::spawn(async move {
                if let Err(e) = db.connect().await {
                    let _ = tx.send(sqrit::app::AsyncResult::ConnectFailed(e.to_string()));
                    return;
                }
                let schema = db.schema_info().await.ok();
                let _ = tx.send(sqrit::app::AsyncResult::Connected { db, schema });
            });
        }
    }

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    app.drain_async_results();

    // Adapter should be connected (replaced in app.db)
    assert!(app.explorer_state.schema.is_some());
    // Query execution should work with the connected adapter
    app.pending_query = Some("SELECT 1 AS val".to_string());
    app.execute_pending();
    common::wait_for_query(&mut app, std::time::Duration::from_secs(5)).await;
    assert_eq!(app.query_status, QueryStatus::Success);
}

// V2: connect failure shows error in status bar
#[tokio::test]
async fn connect_failure_shows_error() {
    let mut app = make_connected_app();
    // Use invalid path that can't connect
    app.db = Some(Box::new(sqrit::db::sqlite::SqliteAdapter::new("/nonexistent/dir/db.sqlite")));
    app.pending_schema_load = true;

    if app.pending_schema_load {
        if let Some(ref db) = app.db {
            let mut db = db.clone_box();
            let tx = app.async_tx.clone();
            app.pending_schema_load = false;
            tokio::spawn(async move {
                if let Err(e) = db.connect().await {
                    let _ = tx.send(sqrit::app::AsyncResult::ConnectFailed(e.to_string()));
                    return;
                }
                let schema = db.schema_info().await.ok();
                let _ = tx.send(sqrit::app::AsyncResult::Connected { db, schema });
            });
        }
    }

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    app.drain_async_results();

    assert!(matches!(app.query_status, QueryStatus::Error(_)));
    assert!(app.explorer_state.schema.is_none());
}
