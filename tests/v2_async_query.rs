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

// V2: schema load via async task
#[tokio::test]
async fn schema_load_via_async_task() {
    let mut app = make_connected_app();
    if let Some(ref mut db) = app.db {
        db.connect().await.unwrap();
        // Create a table so schema is non-empty
        db.execute("CREATE TABLE test_v2 (id INTEGER PRIMARY KEY, name TEXT)").await.unwrap();
    }

    app.pending_schema_load = true;

    // Simulate the event loop schema load path
    if app.pending_schema_load {
        if let Some(ref db) = app.db {
            let db = db.clone_box();
            let tx = app.async_tx.clone();
            app.pending_schema_load = false;
            tokio::spawn(async move {
                if let Ok(schema) = db.schema_info().await {
                    let _ = tx.send(sqrit::app::AsyncResult::SchemaLoaded(schema));
                }
            });
        }
    }

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    app.drain_async_results();

    assert!(app.explorer_state.schema.is_some());
    let schema = app.explorer_state.schema.unwrap();
    assert!(schema.tables.iter().any(|t| t.name == "test_v2"));
}

// V2: schema load failure leaves explorer_state unchanged
#[tokio::test]
async fn schema_load_failure_is_silent() {
    let mut app = make_connected_app();
    // Don't connect — schema_info() will fail with "not connected"
    app.pending_schema_load = true;

    if app.pending_schema_load {
        if let Some(ref db) = app.db {
            let db = db.clone_box();
            let tx = app.async_tx.clone();
            app.pending_schema_load = false;
            tokio::spawn(async move {
                if let Ok(schema) = db.schema_info().await {
                    let _ = tx.send(sqrit::app::AsyncResult::SchemaLoaded(schema));
                }
                // On error, nothing is sent — explorer_state stays None
            });
        }
    }

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    app.drain_async_results();

    assert!(app.explorer_state.schema.is_none());
}
