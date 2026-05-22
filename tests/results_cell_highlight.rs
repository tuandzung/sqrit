mod common;

use std::collections::HashMap;

use ratatui::backend::TestBackend;
use ratatui::style::Modifier;
use ratatui::Terminal;

use sqrit::app::{App, FocusedPane};
use sqrit::db::types::{QueryResult, ResultColumn, Value};
use sqrit::mode::Mode;

// Build a 3×3 result fixture so we can index in the buffer reliably.
fn small_grid_app() -> App {
    let mut app = common::test_app();
    let cols = ["a", "b", "c"];
    let columns: Vec<ResultColumn> = cols.iter().map(|n| ResultColumn::untyped(*n)).collect();
    let mut rows: Vec<HashMap<String, Value>> = Vec::with_capacity(3);
    for r in 0..3 {
        let mut row: HashMap<String, Value> = HashMap::new();
        for (c, name) in cols.iter().enumerate() {
            row.insert(name.to_string(), Value::Text(format!("v{}{}", r, c)));
        }
        rows.push(row);
    }
    app.results = Some(QueryResult {
        columns,
        rows,
        rows_affected: Some(3),
        total_count: Some(3),
    });
    app.mode = Mode::Results;
    app.focused_pane = FocusedPane::Results;
    app
}

/// Find the first buffer cell whose symbol matches `needle`. Returns (x, y).
fn locate(terminal: &Terminal<TestBackend>, needle: &str) -> Option<(u16, u16)> {
    let buffer = terminal.backend().buffer();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer[(x, y)].symbol() == needle {
                return Some((x, y));
            }
        }
    }
    None
}

fn render(app: &mut App) -> Terminal<TestBackend> {
    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    terminal
}

// T8: the active body cell renders with Modifier::REVERSED layered over the
// row tint.
#[test]
fn active_body_cell_carries_reverse_modifier() {
    let mut app = small_grid_app();
    app.results_state.selected_row = 1;
    app.results_state.selected_col = 1;
    let terminal = render(&mut app);

    // locate() finds the first "v" — which may be in row 0 — so search the
    // 3-char "v11" sequence directly to land on the active cell.
    let (x, y) = (|| {
        let b = terminal.backend().buffer();
        for y in 0..b.area.height {
            for x in 0..b.area.width.saturating_sub(2) {
                if b[(x, y)].symbol() == "v"
                    && b[(x + 1, y)].symbol() == "1"
                    && b[(x + 2, y)].symbol() == "1"
                {
                    return (x, y);
                }
            }
        }
        panic!("could not find active-cell value 'v11' in buffer");
    })();

    let cell = &terminal.backend().buffer()[(x, y)];
    assert!(
        cell.modifier.contains(Modifier::REVERSED),
        "active body cell must carry REVERSED, got {:?}",
        cell.modifier,
    );
}

// T8: the header cell for the active column carries REVERSED.
#[test]
fn active_column_header_carries_reverse_modifier() {
    let mut app = small_grid_app();
    app.results_state.selected_col = 1;
    let terminal = render(&mut app);

    // Header "b" is the active column header. Find it.
    let (x, y) = locate(&terminal, "b").expect("header 'b' not found");
    let cell = &terminal.backend().buffer()[(x, y)];
    assert!(
        cell.modifier.contains(Modifier::REVERSED),
        "active header cell must carry REVERSED, got {:?}",
        cell.modifier,
    );
}

// T8: a body cell that is neither in the selected row nor the selected
// column does NOT carry REVERSED.
#[test]
fn inactive_body_cell_lacks_reverse_modifier() {
    let mut app = small_grid_app();
    app.results_state.selected_row = 1;
    app.results_state.selected_col = 1;
    let terminal = render(&mut app);

    let (x, y) = (|| {
        let b = terminal.backend().buffer();
        // v00 — row 0, col 0; unrelated to selection (1,1).
        for y in 0..b.area.height {
            for x in 0..b.area.width.saturating_sub(2) {
                if b[(x, y)].symbol() == "v"
                    && b[(x + 1, y)].symbol() == "0"
                    && b[(x + 2, y)].symbol() == "0"
                {
                    return (x, y);
                }
            }
        }
        panic!("could not find non-active cell value 'v00' in buffer");
    })();

    let cell = &terminal.backend().buffer()[(x, y)];
    assert!(
        !cell.modifier.contains(Modifier::REVERSED),
        "non-active body cell must NOT carry REVERSED, got {:?}",
        cell.modifier,
    );
}

// T8: a non-active column's header does NOT carry REVERSED.
#[test]
fn inactive_column_header_lacks_reverse_modifier() {
    let mut app = small_grid_app();
    app.results_state.selected_col = 1;
    let terminal = render(&mut app);

    // Header "a" is column 0; unrelated to selected_col=1.
    let (x, y) = locate(&terminal, "a").expect("header 'a' not found");
    let cell = &terminal.backend().buffer()[(x, y)];
    assert!(
        !cell.modifier.contains(Modifier::REVERSED),
        "non-active header cell must NOT carry REVERSED, got {:?}",
        cell.modifier,
    );
}

// T8: a body cell in the selected row but a different column keeps the row
// tint (selection_bg) and does NOT carry REVERSED — row tint and cell
// reverse are independent layers.
#[test]
fn selected_row_non_selected_col_has_row_tint_no_reverse() {
    let mut app = small_grid_app();
    app.results_state.selected_row = 1;
    app.results_state.selected_col = 0;
    let terminal = render(&mut app);

    // v12 — row 1 (selected), col 2 (not selected).
    let (x, y) = (|| {
        let b = terminal.backend().buffer();
        for y in 0..b.area.height {
            for x in 0..b.area.width.saturating_sub(2) {
                if b[(x, y)].symbol() == "v"
                    && b[(x + 1, y)].symbol() == "1"
                    && b[(x + 2, y)].symbol() == "2"
                {
                    return (x, y);
                }
            }
        }
        panic!("could not find selected-row-other-col cell 'v12' in buffer");
    })();

    let cell = &terminal.backend().buffer()[(x, y)];
    assert!(
        !cell.modifier.contains(Modifier::REVERSED),
        "selected-row non-selected-col cell must NOT carry REVERSED, got {:?}",
        cell.modifier,
    );
    assert_eq!(
        cell.bg, app.theme.selection_bg,
        "selected-row cell must keep row tint, got bg {:?}",
        cell.bg,
    );
}

// T8: cell highlight persists when focus is on another pane (border may
// change colour, but the active cell's REVERSED modifier stays).
#[test]
fn cell_highlight_persists_when_focus_leaves_results() {
    let mut app = small_grid_app();
    app.results_state.selected_row = 1;
    app.results_state.selected_col = 1;
    // Switch focus to Query pane; do NOT change selected_row/col.
    app.focused_pane = FocusedPane::Query;
    app.mode = Mode::QueryNormal;
    let terminal = render(&mut app);

    let (x, y) = (|| {
        let b = terminal.backend().buffer();
        for y in 0..b.area.height {
            for x in 0..b.area.width.saturating_sub(2) {
                if b[(x, y)].symbol() == "v"
                    && b[(x + 1, y)].symbol() == "1"
                    && b[(x + 2, y)].symbol() == "1"
                {
                    return (x, y);
                }
            }
        }
        panic!("could not find active-cell value 'v11' after focus change");
    })();

    let cell = &terminal.backend().buffer()[(x, y)];
    assert!(
        cell.modifier.contains(Modifier::REVERSED),
        "active cell must keep REVERSED across focus changes, got {:?}",
        cell.modifier,
    );
}
