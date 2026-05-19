mod common;

use sqrit::app::{App, FocusedPane};
use sqrit::db::types::{ColumnInfo, SchemaInfo, TableInfo};
use sqrit::explorer::{ExplorerState, TreeItem};
use sqrit::mode::Mode;

fn make_schema() -> SchemaInfo {
    SchemaInfo {
        tables: vec![
            TableInfo {
                name: "users".to_string(),
                columns: vec![
                    ColumnInfo {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        is_primary_key: true,
                    },
                    ColumnInfo {
                        name: "name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        is_primary_key: false,
                    },
                ],
            },
            TableInfo {
                name: "orders".to_string(),
                columns: vec![ColumnInfo {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    is_primary_key: true,
                }],
            },
        ],
        views: vec![],
    }
}

// T16 #1: items() returns table names collapsed
#[test]
fn items_shows_collapsed_tables() {
    let mut state = ExplorerState::new();
    state.schema = Some(make_schema());

    let items = state.items();
    assert_eq!(items.len(), 2);
    assert!(
        matches!(&items[0], TreeItem::Table { name, expanded } if name == "users" && !expanded)
    );
    assert!(
        matches!(&items[1], TreeItem::Table { name, expanded } if name == "orders" && !expanded)
    );
}

// T16 #2: toggle expands table, items shows columns
#[test]
fn toggle_expand_shows_columns() {
    let mut state = ExplorerState::new();
    state.schema = Some(make_schema());
    state.toggle("users");

    let items = state.items();
    assert_eq!(items.len(), 4); // users + 2 cols + orders
    assert!(
        matches!(&items[0], TreeItem::Table { name, expanded } if name == "users" && *expanded)
    );
    assert!(matches!(&items[1], TreeItem::Column { name, .. } if name == "id"));
    assert!(matches!(&items[2], TreeItem::Column { name, .. } if name == "name"));
    assert!(matches!(&items[3], TreeItem::Table { name, .. } if name == "orders"));
}

// T16 #3: toggle collapse hides columns
#[test]
fn toggle_collapse_hides_columns() {
    let mut state = ExplorerState::new();
    state.schema = Some(make_schema());
    state.toggle("users");
    state.toggle("users");

    let items = state.items();
    assert_eq!(items.len(), 2);
    assert!(matches!(&items[0], TreeItem::Table { expanded, .. } if !expanded));
}

// T16 #4: move_down/move_up navigates selection
#[test]
fn move_down_up_navigates() {
    let mut state = ExplorerState::new();
    state.schema = Some(make_schema());

    assert_eq!(state.selected, 0);
    state.move_down();
    assert_eq!(state.selected, 1);
    state.move_down();
    assert_eq!(state.selected, 1); // clamp at last item

    state.move_up();
    assert_eq!(state.selected, 0);
    state.move_up();
    assert_eq!(state.selected, 0); // clamp at 0
}

fn make_explorer_app() -> App {
    let mut app = common::test_app();
    app.mode = Mode::Explorer;
    app.focused_pane = FocusedPane::Explorer;
    app.explorer_state.schema = Some(make_schema());
    app
}

// T16 #5: Enter toggles expand on selected table
#[test]
fn enter_toggles_expand() {
    let mut app = make_explorer_app();

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );
    let mode = app.mode;
    mode.handle_key(key, &mut app);

    let items = app.explorer_state.items();
    assert_eq!(items.len(), 4); // users expanded (2 cols) + orders
}

// T25 #1: initial scroll offset is 0, default visible_rows > 0
#[test]
fn scroll_default_zero() {
    let state = ExplorerState::new();
    assert_eq!(state.scroll_offset, 0);
    assert!(state.visible_rows > 0);
}

fn large_schema(n: usize) -> SchemaInfo {
    SchemaInfo {
        tables: (0..n)
            .map(|i| TableInfo {
                name: format!("t{}", i),
                columns: vec![],
            })
            .collect(),
        views: vec![],
    }
}

// T25 #2: move_down past viewport scrolls offset forward
#[test]
fn scroll_advances_past_viewport() {
    let mut state = ExplorerState::new();
    state.schema = Some(large_schema(30));
    state.visible_rows = 10;

    for _ in 0..9 {
        state.move_down();
    }
    // selected = 9 (last visible row), still within viewport
    assert_eq!(state.selected, 9);
    assert_eq!(state.scroll_offset, 0);

    state.move_down();
    // selected = 10, scroll forward by 1
    assert_eq!(state.selected, 10);
    assert_eq!(state.scroll_offset, 1);

    state.move_down();
    assert_eq!(state.selected, 11);
    assert_eq!(state.scroll_offset, 2);
}

// T25 #3: move_up back into viewport keeps scroll, going above scrolls back
#[test]
fn scroll_reverses_on_move_up() {
    let mut state = ExplorerState::new();
    state.schema = Some(large_schema(30));
    state.visible_rows = 10;

    for _ in 0..15 {
        state.move_down();
    }
    assert_eq!(state.selected, 15);
    assert_eq!(state.scroll_offset, 6); // 15 - 10 + 1

    // move up within viewport — no scroll change
    state.move_up();
    assert_eq!(state.selected, 14);
    assert_eq!(state.scroll_offset, 6);

    // move up to top of viewport
    for _ in 0..8 {
        state.move_up();
    }
    assert_eq!(state.selected, 6);
    assert_eq!(state.scroll_offset, 6);

    // one more — scroll up
    state.move_up();
    assert_eq!(state.selected, 5);
    assert_eq!(state.scroll_offset, 5);
}

// T25 #4: adjust_scroll clamps when selection drops below viewport (e.g., collapse)
#[test]
fn adjust_scroll_clamps_after_external_selection_change() {
    let mut state = ExplorerState::new();
    state.schema = Some(large_schema(30));
    state.visible_rows = 10;
    state.scroll_offset = 15;
    state.selected = 3;

    state.adjust_scroll();
    assert_eq!(state.scroll_offset, 3);
}

// T25 #5: zero visible_rows does not panic
#[test]
fn adjust_scroll_zero_viewport_no_panic() {
    let mut state = ExplorerState::new();
    state.schema = Some(large_schema(5));
    state.visible_rows = 0;
    state.selected = 2;
    state.adjust_scroll();
    assert_eq!(state.scroll_offset, 2);
}

// T25 #6: adjust_scroll clamps scroll_offset to max_scroll (len - visible_rows)
#[test]
fn adjust_scroll_clamps_to_max_scroll() {
    let mut state = ExplorerState::new();
    state.schema = Some(large_schema(30));
    state.visible_rows = 10;
    state.scroll_offset = 25; // past max_scroll = 30 - 10 = 20
    state.selected = 22;
    state.adjust_scroll();
    assert!(state.scroll_offset <= 20);
}

// T25 #7: set_viewport clamps selected when item count shrinks below it
#[test]
fn set_viewport_clamps_selected_on_shrink() {
    let mut state = ExplorerState::new();
    state.schema = Some(large_schema(30));
    state.visible_rows = 10;
    state.selected = 25;
    state.adjust_scroll();

    // Schema shrinks to 5 items (e.g., reload)
    state.schema = Some(large_schema(5));
    state.set_viewport(10);

    assert_eq!(state.selected, 4); // clamped to len - 1
    assert_eq!(state.scroll_offset, 0); // len <= visible_rows
}

// T25 #8: set_viewport on empty schema resets selected and scroll_offset
#[test]
fn set_viewport_empty_resets_state() {
    let mut state = ExplorerState::new();
    state.selected = 7;
    state.scroll_offset = 3;
    state.set_viewport(10);
    assert_eq!(state.selected, 0);
    assert_eq!(state.scroll_offset, 0);
}

// T25 #9: adjust_scroll with usize::MAX scroll_offset does not overflow
#[test]
fn adjust_scroll_saturates_on_overflow() {
    let mut state = ExplorerState::new();
    state.schema = Some(large_schema(30));
    state.visible_rows = 10;
    state.scroll_offset = usize::MAX;
    state.selected = 5;
    state.adjust_scroll(); // must not panic
    assert!(state.scroll_offset <= 20);
}
