mod common;

use ratatui::backend::TestBackend;
use ratatui::Terminal;
use sqrit::app::{App, FocusedPane};
use sqrit::db::types::{
    ColumnInfo, IndexObject, Namespace, ObjectKind, SchemaInfo, TableObject, TriggerObject,
    ViewObject,
};
use sqrit::explorer::{ExplorerState, NodeKey, TreeItem};
use sqrit::mode::Mode;

fn col(name: &str) -> ColumnInfo {
    ColumnInfo {
        name: name.to_string(),
        data_type: "TEXT".to_string(),
        nullable: true,
        is_primary_key: false,
    }
}

fn single_ns_schema() -> SchemaInfo {
    SchemaInfo {
        namespaces: vec![Namespace {
            name: String::new(),
            tables: vec![TableObject {
                name: "users".to_string(),
                columns: vec![col("id"), col("email")],
            }],
            views: vec![ViewObject {
                name: "logs".to_string(),
                columns: vec![col("id")],
            }],
            materialized_views: vec![],
            indexes: vec![IndexObject {
                name: "idx_email".to_string(),
                table: "users".to_string(),
                unique: true,
            }],
            triggers: vec![TriggerObject {
                name: "trg".to_string(),
                table: "users".to_string(),
                event: "UPDATE".to_string(),
            }],
            functions: vec![],
            procedures: vec![],
            sequences: vec![],
        }],
    }
}

fn multi_ns_schema() -> SchemaInfo {
    SchemaInfo {
        namespaces: vec![Namespace::empty("public"), Namespace::empty("analytics")],
    }
}

#[test]
fn single_namespace_hides_namespace_row() {
    let mut state = ExplorerState::default();
    state.set_schema(single_ns_schema());

    let items = state.items();
    assert!(!matches!(items.first(), Some(TreeItem::Namespace { .. })));
    assert!(matches!(
        items.first(),
        Some(TreeItem::Group {
            kind: ObjectKind::Table,
            ..
        })
    ));
}

#[test]
fn multi_namespace_shows_namespace_rows() {
    let mut state = ExplorerState::default();
    state.set_schema(multi_ns_schema());

    let items = state.items();
    assert!(matches!(
        &items[0],
        TreeItem::Namespace { name, .. } if name == "public"
    ));
    assert!(matches!(
        &items[1],
        TreeItem::Namespace { name, .. } if name == "analytics"
    ));
}

#[test]
fn default_namespace_keeps_raw_empty_key() {
    let mut state = ExplorerState::default();
    state.set_schema(SchemaInfo {
        namespaces: vec![Namespace::empty(""), Namespace::empty("other")],
    });

    let item = &state.items()[0];
    assert!(matches!(item, TreeItem::Namespace { name, .. } if name.is_empty()));
    assert_eq!(item.key(), Some(NodeKey::Namespace(String::new())));
}

#[test]
fn default_namespace_renders_friendly_label() {
    let mut app = common::test_app();
    app.mode = Mode::Explorer;
    app.focused_pane = FocusedPane::Explorer;
    app.explorer_state.set_schema(SchemaInfo {
        namespaces: vec![Namespace::empty(""), Namespace::empty("other")],
    });

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    let buffer = terminal.backend().buffer();
    let rendered = (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        rendered.contains("(default)"),
        "rendered explorer: {rendered}"
    );
}

#[test]
fn empty_groups_are_hidden() {
    let mut state = ExplorerState::default();
    state.set_schema(single_ns_schema());

    for item in state.items() {
        if let TreeItem::Group { kind, .. } = item {
            assert!(!matches!(
                kind,
                ObjectKind::MaterializedView
                    | ObjectKind::Function
                    | ObjectKind::Procedure
                    | ObjectKind::Sequence
            ));
        }
    }
}

#[test]
fn group_header_count_matches_member_count() {
    let mut state = ExplorerState::default();
    state.set_schema(single_ns_schema());

    assert!(matches!(
        state.items().iter().find(|item| matches!(
            item,
            TreeItem::Group {
                kind: ObjectKind::Table,
                ..
            }
        )),
        Some(TreeItem::Group { count: 1, .. })
    ));
}

#[test]
fn expanding_table_reveals_columns() {
    let mut state = ExplorerState::default();
    state.set_schema(single_ns_schema());
    state.toggle_key(NodeKey::Group {
        ns: String::new(),
        kind: ObjectKind::Table,
    });
    state.toggle_key(NodeKey::Object {
        ns: String::new(),
        kind: ObjectKind::Table,
        name: "users".to_string(),
    });

    let columns = state
        .items()
        .into_iter()
        .filter(|item| matches!(item, TreeItem::Column { .. }))
        .count();
    assert_eq!(columns, 2);
}

#[test]
fn leaf_objects_have_no_toggle_key() {
    let mut state = ExplorerState::default();
    state.set_schema(single_ns_schema());
    state.toggle_key(NodeKey::Group {
        ns: String::new(),
        kind: ObjectKind::Index,
    });

    let index = state
        .items()
        .into_iter()
        .find(|item| {
            matches!(
                item,
                TreeItem::Object {
                    kind: ObjectKind::Index,
                    ..
                }
            )
        })
        .unwrap();
    assert_eq!(index.key(), None);
}

fn make_explorer_app() -> App {
    let mut app = common::test_app();
    app.mode = Mode::Explorer;
    app.focused_pane = FocusedPane::Explorer;
    app.explorer_state.set_schema(single_ns_schema());
    app
}

fn enter(app: &mut App) {
    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, app);
}

#[test]
fn enter_toggles_group_and_expandable_object() {
    let mut app = make_explorer_app();

    enter(&mut app);
    assert!(matches!(
        &app.explorer_state.items()[0],
        TreeItem::Group {
            kind: ObjectKind::Table,
            expanded: true,
            ..
        }
    ));

    app.explorer_state.move_down();
    enter(&mut app);
    assert_eq!(
        app.explorer_state
            .items()
            .iter()
            .filter(|item| matches!(item, TreeItem::Column { .. }))
            .count(),
        2
    );
}

#[test]
fn enter_on_leaf_object_does_nothing() {
    let mut app = make_explorer_app();
    app.explorer_state.toggle_key(NodeKey::Group {
        ns: String::new(),
        kind: ObjectKind::Index,
    });
    app.explorer_state.selected = app
        .explorer_state
        .items()
        .iter()
        .position(|item| {
            matches!(
                item,
                TreeItem::Object {
                    kind: ObjectKind::Index,
                    ..
                }
            )
        })
        .unwrap();
    let before = app.explorer_state.expanded.clone();

    enter(&mut app);

    assert_eq!(app.explorer_state.expanded, before);
}

#[test]
fn move_down_up_navigates_visible_items() {
    let mut state = ExplorerState::default();
    state.set_schema(single_ns_schema());
    let last = state.items().len() - 1;

    for _ in 0..state.items().len() + 2 {
        state.move_down();
    }
    assert_eq!(state.selected, last);
    for _ in 0..state.items().len() + 2 {
        state.move_up();
    }
    assert_eq!(state.selected, 0);
}

#[test]
fn scroll_default_zero() {
    let state = ExplorerState::new();
    assert_eq!(state.scroll_offset, 0);
    assert!(state.visible_rows > 0);
}

fn large_schema(n: usize) -> SchemaInfo {
    let mut namespace = Namespace::empty("");
    namespace.tables = (0..n)
        .map(|i| TableObject {
            name: format!("t{i}"),
            columns: vec![],
        })
        .collect();
    SchemaInfo {
        namespaces: vec![namespace],
    }
}

fn large_state(n: usize) -> ExplorerState {
    let mut state = ExplorerState::new();
    state.set_schema(large_schema(n));
    state.toggle_key(NodeKey::Group {
        ns: String::new(),
        kind: ObjectKind::Table,
    });
    state
}

#[test]
fn scroll_advances_past_viewport() {
    let mut state = large_state(30);
    state.visible_rows = 10;

    for _ in 0..9 {
        state.move_down();
    }
    assert_eq!(state.selected, 9);
    assert_eq!(state.scroll_offset, 0);

    state.move_down();
    assert_eq!(state.selected, 10);
    assert_eq!(state.scroll_offset, 1);

    state.move_down();
    assert_eq!(state.selected, 11);
    assert_eq!(state.scroll_offset, 2);
}

#[test]
fn scroll_reverses_on_move_up() {
    let mut state = large_state(30);
    state.visible_rows = 10;

    for _ in 0..15 {
        state.move_down();
    }
    assert_eq!(state.selected, 15);
    assert_eq!(state.scroll_offset, 6);

    state.move_up();
    assert_eq!(state.selected, 14);
    assert_eq!(state.scroll_offset, 6);

    for _ in 0..8 {
        state.move_up();
    }
    assert_eq!(state.selected, 6);
    assert_eq!(state.scroll_offset, 6);

    state.move_up();
    assert_eq!(state.selected, 5);
    assert_eq!(state.scroll_offset, 5);
}

#[test]
fn adjust_scroll_clamps_after_external_selection_change() {
    let mut state = large_state(30);
    state.visible_rows = 10;
    state.scroll_offset = 15;
    state.selected = 3;

    state.adjust_scroll();
    assert_eq!(state.scroll_offset, 3);
}

#[test]
fn adjust_scroll_zero_viewport_no_panic() {
    let mut state = large_state(5);
    state.visible_rows = 0;
    state.selected = 2;
    state.adjust_scroll();
    assert_eq!(state.scroll_offset, 2);
}

#[test]
fn adjust_scroll_clamps_to_max_scroll() {
    let mut state = large_state(30);
    state.visible_rows = 10;
    state.scroll_offset = 25;
    state.selected = 22;
    state.adjust_scroll();
    assert!(state.scroll_offset <= 21);
}

#[test]
fn set_viewport_clamps_selected_on_shrink() {
    let mut state = large_state(30);
    state.visible_rows = 10;
    state.selected = 25;
    state.adjust_scroll();

    state.schema = Some(large_schema(5));
    state.set_viewport(10);

    assert_eq!(state.selected, 5);
    assert_eq!(state.scroll_offset, 0);
}

#[test]
fn set_viewport_empty_resets_state() {
    let mut state = ExplorerState::new();
    state.selected = 7;
    state.scroll_offset = 3;
    state.set_viewport(10);
    assert_eq!(state.selected, 0);
    assert_eq!(state.scroll_offset, 0);
}

#[test]
fn adjust_scroll_saturates_on_overflow() {
    let mut state = large_state(30);
    state.visible_rows = 10;
    state.scroll_offset = usize::MAX;
    state.selected = 5;
    state.adjust_scroll();
    assert!(state.scroll_offset <= 21);
}
