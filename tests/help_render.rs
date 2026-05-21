mod common;

use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use sqrit::app::App;
use sqrit::mode::help::HelpState;
use sqrit::mode::Mode;

fn buffer_contains(terminal: &Terminal<TestBackend>, needle: &str) -> bool {
    let buffer = terminal.backend().buffer();
    for y in 0..buffer.area.height {
        let mut row = String::new();
        for x in 0..buffer.area.width {
            row.push_str(buffer[(x, y)].symbol());
        }
        if row.contains(needle) {
            return true;
        }
    }
    false
}

fn open_help(app: &mut App, origin: Mode) {
    app.help = Some(HelpState { origin });
    app.mode = Mode::Help;
}

// --- Slice 12: modal renders the origin mode's bindings ---

#[test]
fn help_modal_renders_origin_bindings_in_query_normal() {
    let mut app = common::test_app();
    open_help(&mut app, Mode::QueryNormal);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();

    assert!(
        buffer_contains(&terminal, "Help"),
        "modal must have a 'Help' title"
    );
    let any_action_rendered = Mode::QueryNormal
        .handler()
        .bindings()
        .iter()
        .any(|b| buffer_contains(&terminal, b.action));
    assert!(
        any_action_rendered,
        "at least one binding action must be rendered"
    );
}

#[test]
fn help_modal_renders_explorer_bindings_when_opened_from_explorer() {
    let mut app = common::test_app();
    open_help(&mut app, Mode::Explorer);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();

    let explorer_binding = Mode::Explorer.handler().bindings()[0].action;
    assert!(
        buffer_contains(&terminal, explorer_binding),
        "Explorer binding {:?} must appear in the modal",
        explorer_binding
    );
    let normal_only_binding = "Execute query";
    assert!(
        !buffer_contains(&terminal, normal_only_binding),
        "QueryNormal-only binding {:?} must NOT appear",
        normal_only_binding
    );
}

#[test]
fn help_modal_rect_is_centered_and_fits() {
    let area = Rect::new(0, 0, 80, 24);
    let modal = App::help_modal_rect(area, 5, 12, 30);

    assert!(modal.width <= area.width);
    assert!(modal.height <= area.height);
    let x_offset = modal.x;
    let y_offset = modal.y;
    let right_gap = area.width - (x_offset + modal.width);
    let bottom_gap = area.height - (y_offset + modal.height);
    assert!(
        x_offset.abs_diff(right_gap) <= 1,
        "modal must be horizontally centered"
    );
    assert!(
        y_offset.abs_diff(bottom_gap) <= 1,
        "modal must be vertically centered"
    );
}

#[test]
fn help_modal_does_not_panic_on_tiny_screen() {
    let mut app = common::test_app();
    open_help(&mut app, Mode::QueryNormal);
    let backend = TestBackend::new(8, 4);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
}
