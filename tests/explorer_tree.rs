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
