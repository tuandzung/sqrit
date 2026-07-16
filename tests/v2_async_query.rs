mod common;

use std::collections::HashMap;

use sqrit::app::{App, AsyncResult, QueryStatus};
use sqrit::db::types::{QueryResult, ResultColumn, Value};
use sqrit::db::Database;
use sqrit::results_render::{matched_ranges_for, render_cell};

fn make_connected_app() -> App {
    common::test_app()
}

#[test]
fn accepted_page_result_reranks_committed_filter_before_exposure() {
    let mut app = make_connected_app();
    let old_result = QueryResult {
        columns: vec![ResultColumn::untyped("name"), ResultColumn::untyped("city")],
        rows: vec![
            HashMap::from([
                ("name".into(), Value::Text("zero".into())),
                ("city".into(), Value::Text("Paris".into())),
            ]),
            HashMap::from([
                ("name".into(), Value::Text("one".into())),
                ("city".into(), Value::Text("Rome".into())),
            ]),
            HashMap::from([
                ("name".into(), Value::Text("two".into())),
                ("city".into(), Value::Text("lilac".into())),
            ]),
        ],
        rows_affected: None,
        total_count: None,
    };
    app.results_state.filter = Some("li".into());
    app.results_state.filter_hits = app.fuzzy_filter.rank(&old_result, "li");
    app.results = Some(old_result);
    app.query_id = 7;

    let new_result = QueryResult {
        columns: vec![ResultColumn::untyped("label")],
        rows: vec![HashMap::from([(
            "label".into(),
            Value::Text("lima".into()),
        )])],
        rows_affected: None,
        total_count: None,
    };
    app.async_tx
        .send(AsyncResult::QueryDone {
            query_id: 7,
            status: QueryStatus::Success,
            result: Some(new_result),
            has_next_page: false,
        })
        .unwrap();
    app.drain_async_results();

    let result = app.results.as_ref().unwrap();
    let visible = app.results_state.visible_row_indices(result);
    let coordinates: Vec<_> = app
        .results_state
        .filter_hits
        .iter()
        .flat_map(|hit| {
            hit.matches
                .iter()
                .map(move |(column, range)| (hit.row_index, *column, range.clone()))
        })
        .collect();
    let rendered = render_cell(
        "lima",
        matched_ranges_for(&app.results_state.filter_hits, 0, 0),
        &app.theme,
    )
    .into_iter()
    .map(|span| span.content)
    .collect::<Vec<_>>()
    .concat();

    assert_eq!(
        (result.column_names(), visible, coordinates, rendered),
        (vec!["label"], vec![0], vec![(0, 0, 0..2)], "lima".into())
    );
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
    assert_eq!(result.column_names(), vec!["val"]);
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

// Helper: simulate the event loop connect + schema load spawn
fn spawn_connect_and_schema(app: &mut App) {
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
}

// V2: connect + schema load via async task (picker flow)
#[tokio::test]
async fn connect_and_schema_load_via_async_task() {
    let mut app = make_connected_app();

    // Use a temp file so the table persists across adapter instances
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    // Create table via a connected adapter
    {
        let mut db = sqrit::db::sqlite::SqliteAdapter::new(&path);
        db.connect().await.unwrap();
        db.execute("CREATE TABLE test_v2 (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();
    }

    // Replace with fresh unconnected adapter pointing at same file (simulates picker)
    app.db = Some(Box::new(sqrit::db::sqlite::SqliteAdapter::new(&path)));
    app.pending_schema_load = true;
    app.explorer_state
        .expanded
        .insert(sqrit::explorer::NodeKey::Namespace("stale".to_string()));
    app.explorer_state.selected = 4;
    app.explorer_state.scroll_offset = 2;

    spawn_connect_and_schema(&mut app);

    common::wait_for_connect(&mut app, std::time::Duration::from_secs(5)).await;

    // Adapter should be connected (replaced in app.db)
    assert!(app.explorer_state.schema.is_some());
    let schema = app.explorer_state.schema.as_ref().unwrap();
    assert!(schema.namespaces[0]
        .tables
        .iter()
        .any(|table| table.name == "test_v2"));
    assert_eq!(app.explorer_state.selected, 0);
    assert_eq!(app.explorer_state.scroll_offset, 0);
    assert_eq!(
        app.explorer_state.expanded,
        std::collections::HashSet::from([sqrit::explorer::NodeKey::Namespace(String::new())])
    );

    // Query execution should work with the connected adapter
    app.pending_query = Some("SELECT 1 AS val".to_string());
    app.execute_pending();
    common::wait_for_query(&mut app, std::time::Duration::from_secs(5)).await;
    assert_eq!(app.query_status, QueryStatus::Success);
}

// T7: AsyncResult::Cancelled { in_tx: true } surfaces the ROLLBACK hint.
// Injecting the variant directly avoids any DB setup; the status text the
// user actually sees is the bit ADR 6 nails down.
#[tokio::test]
async fn cancelled_status_in_tx_shows_rollback_hint() {
    let mut app = make_connected_app();

    app.async_tx
        .send(sqrit::app::AsyncResult::Cancelled { in_tx: true })
        .unwrap();
    app.drain_async_results();

    assert_eq!(
        app.status_message,
        "query cancelled — transaction may need ROLLBACK"
    );
    assert_eq!(app.query_status, QueryStatus::Idle);
}

// T7: AsyncResult::Cancelled { in_tx: false } surfaces the plain message.
#[tokio::test]
async fn cancelled_status_without_tx_is_plain() {
    let mut app = make_connected_app();

    app.async_tx
        .send(sqrit::app::AsyncResult::Cancelled { in_tx: false })
        .unwrap();
    app.drain_async_results();

    assert_eq!(app.status_message, "query cancelled");
    assert_eq!(app.query_status, QueryStatus::Idle);
}

// T7: <space>z fires DB cancel, status bar shows "query cancelled".
#[tokio::test]
async fn cancel_sets_status_and_drops_stale_result() {
    let mut app = make_connected_app();
    if let Some(ref mut db) = app.db {
        db.connect().await.unwrap();
    }

    app.editor.insert_str(
        "WITH RECURSIVE c(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM c WHERE n < 100000000) \
         SELECT MAX(n) FROM c",
    );
    app.pending_query = Some(app.editor.text());
    app.execute_pending();
    assert_eq!(app.query_status, QueryStatus::Running);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    app.trigger_cancel();

    let start = std::time::Instant::now();
    loop {
        app.drain_async_results();
        if app.status_message.starts_with("query cancelled") {
            break;
        }
        if start.elapsed() > std::time::Duration::from_secs(3) {
            panic!(
                "cancel did not surface status within 3s; got {:?}, query_status={:?}",
                app.status_message, app.query_status
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    assert_eq!(app.query_status, QueryStatus::Idle);
}

// V2: connect failure shows error in status bar
#[tokio::test]
async fn connect_failure_shows_error() {
    let mut app = make_connected_app();
    // Use invalid path that can't connect
    app.db = Some(Box::new(sqrit::db::sqlite::SqliteAdapter::new(
        "/nonexistent/dir/db.sqlite",
    )));
    app.pending_schema_load = true;

    spawn_connect_and_schema(&mut app);

    common::wait_for_connect(&mut app, std::time::Duration::from_secs(5)).await;

    assert!(matches!(app.query_status, QueryStatus::Error(_)));
    assert!(app.explorer_state.schema.is_none());
}

// Test adapter: connect succeeds, schema_info fails
struct SchemaFailAdapter;

#[async_trait::async_trait]
impl sqrit::db::Database for SchemaFailAdapter {
    async fn connect(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn disconnect(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn execute(&self, _query: &str) -> anyhow::Result<sqrit::db::types::QueryResult> {
        Ok(sqrit::db::types::QueryResult {
            columns: vec![sqrit::db::types::ResultColumn::untyped("val")],
            rows: vec![{
                let mut map = std::collections::HashMap::new();
                map.insert("val".to_string(), sqrit::db::types::Value::Integer(1));
                map
            }],
            rows_affected: Some(1),
            total_count: None,
        })
    }
    async fn execute_paginated(
        &self,
        query: &str,
        _offset: u64,
        _limit: u64,
    ) -> anyhow::Result<sqrit::db::types::QueryResult> {
        self.execute(query).await
    }
    async fn list_tables(&self) -> anyhow::Result<Vec<String>> {
        Ok(vec![])
    }
    async fn list_views(&self) -> anyhow::Result<Vec<String>> {
        Ok(vec![])
    }
    async fn list_columns(
        &self,
        _table: &str,
    ) -> anyhow::Result<Vec<sqrit::db::types::ColumnInfo>> {
        Ok(vec![])
    }
    async fn schema_info(&self) -> anyhow::Result<sqrit::db::types::SchemaInfo> {
        anyhow::bail!("intentional schema_info failure for test")
    }
    fn clone_box(&self) -> Box<dyn sqrit::db::Database> {
        Box::new(SchemaFailAdapter)
    }
}

// V2: connect succeeds but schema_info fails — non-fatal, queries still work
#[tokio::test]
async fn connect_ok_schema_fail_keeps_querying() {
    let mut app = make_connected_app();
    app.db = Some(Box::new(SchemaFailAdapter));
    app.pending_schema_load = true;

    spawn_connect_and_schema(&mut app);

    // Yield to let spawned task complete (no-op adapter, instant)
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;
    app.drain_async_results();

    // Connected adapter kept, schema is None, no error
    assert!(app.db.is_some());
    assert!(app.explorer_state.schema.is_none());
    assert!(!matches!(app.query_status, QueryStatus::Error(_)));

    // Queries should still succeed
    app.pending_query = Some("SELECT 1".to_string());
    app.execute_pending();
    common::wait_for_query(&mut app, std::time::Duration::from_secs(5)).await;
    assert_eq!(app.query_status, QueryStatus::Success);
}
