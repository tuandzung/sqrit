mod common;

use ratatui::{
    backend::TestBackend,
    layout::{Position, Rect},
    Terminal,
};
use sqrit::{
    app::{App, FocusedPane},
    mode::Mode,
};

fn inner(x: u16, y: u16, w: u16, h: u16) -> Rect {
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}

// V8: cursor at row 0, col 0 — no scroll, term coords = inner origin
#[test]
fn cursor_at_origin_no_scroll() {
    let (scroll, tx, ty) = App::insert_cursor_position(0, 0, inner(2, 3, 80, 20));
    assert_eq!(scroll, 0);
    assert_eq!(tx, 2);
    assert_eq!(ty, 3);
}

// V8: cursor in middle of viewport — no scroll
#[test]
fn cursor_mid_viewport_no_scroll() {
    let (scroll, tx, ty) = App::insert_cursor_position(5, 10, inner(2, 3, 80, 20));
    assert_eq!(scroll, 0);
    assert_eq!(tx, 12); // 2 + 10
    assert_eq!(ty, 8); // 3 + 5
}

// V8: cursor exactly at last row — no scroll (row 19 in 20-row viewport)
#[test]
fn cursor_at_last_row_no_scroll() {
    let (scroll, tx, ty) = App::insert_cursor_position(19, 0, inner(0, 0, 80, 20));
    assert_eq!(scroll, 0);
    assert_eq!(tx, 0);
    assert_eq!(ty, 19);
}

// V8: cursor one row past viewport — scroll offset 1
#[test]
fn cursor_past_viewport_scrolls() {
    let (scroll, tx, ty) = App::insert_cursor_position(20, 0, inner(0, 0, 80, 20));
    assert_eq!(scroll, 1);
    assert_eq!(tx, 0);
    assert_eq!(ty, 19); // 0 + 20 - 1 = 19
}

// V8: cursor several rows past viewport
#[test]
fn cursor_far_past_viewport_scrolls() {
    let (scroll, tx, ty) = App::insert_cursor_position(30, 5, inner(1, 2, 80, 20));
    assert_eq!(scroll, 11); // 30 + 1 - 20
    assert_eq!(tx, 6); // 1 + 5
    assert_eq!(ty, 21); // 2 + 30 - 11 = 21
}

// V8: zero-height inner does not panic, scroll = 0, tx/ty do not underflow
#[test]
fn zero_height_inner_no_scroll() {
    let inner_rect = inner(0, 0, 80, 0);
    let (scroll, tx, ty) = App::insert_cursor_position(0, 0, inner_rect);
    assert_eq!(scroll, 0);
    // tx must stay within horizontal bounds of inner rect [x, x+width)
    assert!(
        tx < inner_rect.x + inner_rect.width,
        "tx {tx} outside inner rect horizontal bounds"
    );
    // ty must not underflow below inner rect top
    assert!(
        ty >= inner_rect.y,
        "ty {ty} underflows inner rect top {}",
        inner_rect.y
    );
}

fn rendered_cursor(mode: Mode, width: u16, height: u16, initial: Position) -> Position {
    let mut app = common::test_app();
    app.mode = mode;
    app.maximized = Some(FocusedPane::Query);
    app.editor.insert_str("one\ntwo");

    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.set_cursor_position(initial).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    terminal.get_cursor_position().unwrap()
}

#[test]
fn query_modes_render_cursor_at_editor_position() {
    let expected = Position::new(4, 2);
    for mode in [Mode::QueryNormal, Mode::QueryInsert] {
        assert_eq!(rendered_cursor(mode, 40, 12, Position::new(9, 9)), expected);
    }
}

#[test]
fn non_query_modes_do_not_render_editor_cursor() {
    let initial = Position::new(9, 9);
    assert_eq!(rendered_cursor(Mode::Results, 40, 12, initial), initial);
}

#[test]
fn zero_sized_query_inner_does_not_render_cursor() {
    let initial = Position::new(1, 1);
    assert_eq!(rendered_cursor(Mode::QueryNormal, 2, 2, initial), initial);
}
