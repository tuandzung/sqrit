mod common;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sqrit::app::AsyncResult;
use sqrit::db::types::{Namespace, ObjectKind, ObjectRef, SchemaInfo, ViewObject};
use sqrit::explorer::NodeKey;
use sqrit::mode::definition_viewer::{DefinitionContent, DefinitionViewerState};
use sqrit::mode::Mode;

fn object() -> ObjectRef {
    ObjectRef {
        namespace: String::new(),
        kind: ObjectKind::View,
        name: "active_users".into(),
        relation: None,
        identity_arguments: None,
    }
}

fn press(app: &mut sqrit::app::App, code: KeyCode) {
    app.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn loading_viewer_scrolls_safely_and_ignores_copy() {
    let mut app = common::test_app();
    app.mode = Mode::DefinitionViewer;
    app.definition_viewer = Some(DefinitionViewerState {
        object: object(),
        content: DefinitionContent::Loading,
        scroll: 0,
    });
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Char('y'));
    assert_eq!(app.definition_viewer.as_ref().unwrap().scroll, 1);
    assert!(app.status_message.is_empty());
}

#[test]
fn ready_viewer_scrolls_and_closes_to_explorer() {
    let mut app = common::test_app();
    app.mode = Mode::DefinitionViewer;
    app.definition_viewer = Some(DefinitionViewerState {
        object: object(),
        content: DefinitionContent::Ready("CREATE VIEW active_users AS SELECT 1;".into()),
        scroll: 2,
    });
    press(&mut app, KeyCode::Char('k'));
    assert_eq!(app.definition_viewer.as_ref().unwrap().scroll, 1);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.mode, Mode::Explorer);
    assert!(app.definition_viewer.is_none());
}

#[test]
fn stale_or_closed_async_results_are_ignored() {
    let mut app = common::test_app();
    app.mode = Mode::DefinitionViewer;
    app.definition_request_id = 2;
    app.definition_viewer = Some(DefinitionViewerState {
        object: object(),
        content: DefinitionContent::Loading,
        scroll: 0,
    });
    app.async_tx
        .send(AsyncResult::DefinitionLoaded {
            request_id: 1,
            result: Ok(Some("stale".into())),
        })
        .unwrap();
    app.drain_async_results();
    assert!(matches!(
        app.definition_viewer.as_ref().unwrap().content,
        DefinitionContent::Loading
    ));

    press(&mut app, KeyCode::Esc);
    app.async_tx
        .send(AsyncResult::DefinitionLoaded {
            request_id: 2,
            result: Ok(Some("closed".into())),
        })
        .unwrap();
    app.drain_async_results();
    assert!(app.definition_viewer.is_none());
}

#[test]
fn current_result_transitions_loading_to_ready_or_error() {
    let mut app = common::test_app();
    app.mode = Mode::DefinitionViewer;
    app.definition_request_id = 7;
    app.definition_viewer = Some(DefinitionViewerState {
        object: object(),
        content: DefinitionContent::Loading,
        scroll: 4,
    });
    app.async_tx
        .send(AsyncResult::DefinitionLoaded {
            request_id: 7,
            result: Ok(Some("CREATE VIEW active_users AS SELECT 1;".into())),
        })
        .unwrap();
    app.drain_async_results();
    assert!(matches!(
        app.definition_viewer.as_ref().unwrap().content,
        DefinitionContent::Ready(_)
    ));
    assert_eq!(app.definition_viewer.as_ref().unwrap().scroll, 0);

    app.definition_viewer.as_mut().unwrap().content = DefinitionContent::Loading;
    app.async_tx
        .send(AsyncResult::DefinitionLoaded {
            request_id: 7,
            result: Err("permission denied".into()),
        })
        .unwrap();
    app.drain_async_results();
    assert!(matches!(
        &app.definition_viewer.as_ref().unwrap().content,
        DefinitionContent::Error(error) if error == "permission denied"
    ));
}

async fn wait_for_definition(app: &mut sqrit::app::App) {
    let started = std::time::Instant::now();
    loop {
        app.drain_async_results();
        if !matches!(
            app.definition_viewer.as_ref().map(|state| &state.content),
            Some(DefinitionContent::Loading)
        ) {
            return;
        }
        assert!(
            started.elapsed() < std::time::Duration::from_secs(1),
            "definition load timed out"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

#[tokio::test]
async fn explorer_d_fetches_and_reopen_retries_with_a_new_request() {
    let mut app = common::test_app();
    app.db.as_mut().unwrap().connect().await.unwrap();
    app.db
        .as_ref()
        .unwrap()
        .execute("CREATE VIEW active_users AS SELECT 1 AS id")
        .await
        .unwrap();
    let mut namespace = Namespace::empty("");
    namespace.views.push(ViewObject {
        name: "active_users".into(),
        columns: vec![],
    });
    app.explorer_state.set_schema(SchemaInfo {
        namespaces: vec![namespace],
    });
    app.explorer_state.toggle_key(NodeKey::Group {
        ns: String::new(),
        kind: ObjectKind::View,
    });
    app.explorer_state.selected = app
        .explorer_state
        .items()
        .iter()
        .position(|item| {
            item.object_ref()
                .is_some_and(|object| object.kind == ObjectKind::View)
        })
        .unwrap();
    app.mode = Mode::Explorer;

    press(&mut app, KeyCode::Char('d'));
    let first_request = app.definition_request_id;
    assert_eq!(app.mode, Mode::DefinitionViewer);
    assert!(matches!(
        app.definition_viewer.as_ref().unwrap().content,
        DefinitionContent::Loading
    ));
    wait_for_definition(&mut app).await;
    assert!(matches!(
        app.definition_viewer.as_ref().unwrap().content,
        DefinitionContent::Ready(_)
    ));

    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('d'));
    assert_eq!(app.definition_request_id, first_request.wrapping_add(1));
    assert!(matches!(
        app.definition_viewer.as_ref().unwrap().content,
        DefinitionContent::Loading
    ));
    wait_for_definition(&mut app).await;
    assert!(matches!(
        app.definition_viewer.as_ref().unwrap().content,
        DefinitionContent::Ready(_)
    ));
}
