mod common;

use ratatui::backend::TestBackend;
use ratatui::Terminal;
use sqrit::mode::theme_picker::ThemePickerState;
use sqrit::mode::Mode;
use sqrit::theme::Theme;

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

fn enter_picker(app: &mut sqrit::app::App, names: Vec<&str>) {
    app.theme_picker = Some(ThemePickerState::open(
        names.into_iter().map(String::from).collect(),
        Theme::default_theme(),
        Mode::QueryNormal,
    ));
    app.mode = Mode::ThemePicker;
}

// T1 #3: empty available list doesn't panic
#[test]
fn empty_available_does_not_panic() {
    let mut app = common::test_app();
    enter_picker(&mut app, vec![]);
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
}

// T1 #4: small terminal area still renders without panicking
#[test]
fn tiny_terminal_does_not_panic() {
    let mut app = common::test_app();
    enter_picker(&mut app, vec!["rose-pine", "nord"]);
    let backend = TestBackend::new(8, 4);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
}

// T1 #5: modal rect stays within area bounds for any input
#[test]
fn modal_rect_within_bounds() {
    use ratatui::layout::Rect;
    use sqrit::app::App;

    let area = Rect::new(0, 0, 80, 24);
    let r = App::theme_picker_modal_rect(area, 5, 20);
    assert!(r.x + r.width <= area.x + area.width);
    assert!(r.y + r.height <= area.y + area.height);

    // Long names should clamp to area
    let r = App::theme_picker_modal_rect(area, 5, 200);
    assert!(r.width <= area.width);

    // Many items should clamp to area
    let r = App::theme_picker_modal_rect(area, 200, 10);
    assert!(r.height <= area.height);
}

// T1 #2: selected entry uses theme.selection_bg
#[test]
fn selected_row_uses_selection_bg() {
    let mut app = common::test_app();
    enter_picker(&mut app, vec!["alpha", "beta", "gamma"]);
    if let Some(p) = app.theme_picker.as_mut() {
        p.selected = 1; // "beta"
    }
    let selection_bg = app.theme.selection_bg;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();

    let buffer = terminal.backend().buffer();
    // Find 'beta' row and confirm at least one of its cells has selection_bg
    let mut beta_y: Option<u16> = None;
    for y in 0..buffer.area.height {
        let mut row = String::new();
        for x in 0..buffer.area.width {
            row.push_str(buffer[(x, y)].symbol());
        }
        if row.contains("beta") && !row.contains("alpha") {
            beta_y = Some(y);
            break;
        }
    }
    let y = beta_y.expect("beta row not found");
    let mut has_selection_bg = false;
    for x in 0..buffer.area.width {
        if buffer[(x, y)].bg == selection_bg {
            has_selection_bg = true;
            break;
        }
    }
    assert!(has_selection_bg, "selected row missing selection_bg");

    // And non-selected rows should not carry selection_bg
    let mut alpha_y: Option<u16> = None;
    for y in 0..buffer.area.height {
        let mut row = String::new();
        for x in 0..buffer.area.width {
            row.push_str(buffer[(x, y)].symbol());
        }
        if row.contains("alpha") {
            alpha_y = Some(y);
            break;
        }
    }
    let y = alpha_y.expect("alpha row not found");
    for x in 0..buffer.area.width {
        assert_ne!(
            buffer[(x, y)].bg,
            selection_bg,
            "non-selected row had selection_bg at x={}",
            x
        );
    }
}

// T1 #1: modal renders the list of available theme names
#[test]
fn renders_available_theme_names() {
    let mut app = common::test_app();
    enter_picker(
        &mut app,
        vec![
            "catppuccin-macchiato",
            "gruvbox",
            "nord",
            "rose-pine",
            "tokyo-night",
        ],
    );

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();

    assert!(
        buffer_contains(&terminal, "nord"),
        "expected 'nord' in buffer"
    );
    assert!(
        buffer_contains(&terminal, "gruvbox"),
        "expected 'gruvbox' in buffer"
    );
    assert!(
        buffer_contains(&terminal, "rose-pine"),
        "expected 'rose-pine' in buffer"
    );
}
