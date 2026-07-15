mod common;

use ratatui::backend::TestBackend;
use ratatui::style::Modifier;
use ratatui::text::Span;
use ratatui::Terminal;
use sqrit::app::{App, FocusedPane};
use sqrit::hint_bar::{compose, render, MIN_WIDTH, PALETTE_SUFFIX};
use sqrit::mode::{KeyBinding, Mode};
use sqrit::theme::Theme;

const SAMPLE: &[KeyBinding] = &[
    KeyBinding {
        key: "i",
        action: "insert",
    },
    KeyBinding {
        key: "Enter",
        action: "run",
    },
    KeyBinding {
        key: "yy",
        action: "yank",
    },
];

fn text(spans: &[Span<'_>]) -> String {
    spans.iter().map(|span| span.content.as_ref()).collect()
}

#[test]
fn wide_terminal_renders_bindings_and_palette_suffix() {
    let theme = Theme::default_theme();
    let layout = compose(SAMPLE, true, &theme, 120);

    assert_eq!(text(&layout.left), "i insert  Enter run  yy yank");
    assert_eq!(text(&layout.right), format!(" │ {PALETTE_SUFFIX}"));
    assert_eq!(layout.left[0].style.fg, Some(theme.hint_bar_key));
    assert!(layout.left[0].style.add_modifier.contains(Modifier::BOLD));
    assert_eq!(layout.right[0].style.fg, Some(theme.hint_bar_separator));
}

#[test]
fn widths_below_minimum_yield_an_empty_layout() {
    let theme = Theme::default_theme();

    for width in 0..MIN_WIDTH {
        let layout = compose(SAMPLE, true, &theme, width);
        assert!(layout.left.is_empty(), "left was nonempty at width {width}");
        assert!(
            layout.right.is_empty(),
            "right was nonempty at width {width}"
        );
        assert_eq!(layout.gap, 0, "gap was nonzero at width {width}");
    }
}

#[test]
fn bindings_take_priority_over_palette_suffix() {
    let theme = Theme::default_theme();
    let layout = compose(SAMPLE, true, &theme, MIN_WIDTH);

    assert_eq!(text(&layout.left), "i insert  Enter run  yy yank");
    assert!(layout.right.is_empty());
}

#[test]
fn trailing_bindings_are_dropped_at_supported_widths() {
    const LONG_BINDINGS: &[KeyBinding] = &[
        KeyBinding {
            key: "one",
            action: "first-binding",
        },
        KeyBinding {
            key: "two",
            action: "second-binding",
        },
        KeyBinding {
            key: "three",
            action: "third-binding",
        },
    ];

    let theme = Theme::default_theme();
    let layout = compose(LONG_BINDINGS, true, &theme, MIN_WIDTH);

    assert_eq!(text(&layout.left), "one first-binding  two second-binding");
    assert!(layout.right.is_empty());
}

#[test]
fn truncation_keeps_suffix_when_it_fits_the_largest_prefix() {
    const BINDINGS: &[KeyBinding] = &[
        KeyBinding {
            key: "a",
            action: "go",
        },
        KeyBinding {
            key: "really-long-key",
            action: "this-action-forces-truncation",
        },
    ];

    let theme = Theme::default_theme();
    let layout = compose(BINDINGS, true, &theme, MIN_WIDTH);

    assert_eq!(text(&layout.left), "a go");
    assert_eq!(text(&layout.right), format!(" │ {PALETTE_SUFFIX}"));
}

#[test]
fn one_binding_wider_than_the_row_becomes_an_ellipsis() {
    const TOO_WIDE: &[KeyBinding] = &[KeyBinding {
        key: "x",
        action: "this-single-binding-is-far-too-wide-for-a-forty-column-row",
    }];

    let theme = Theme::default_theme();
    let layout = compose(TOO_WIDE, true, &theme, MIN_WIDTH);

    assert_eq!(text(&layout.left), "…");
    assert!(layout.right.is_empty());
    assert_eq!(layout.gap, MIN_WIDTH - 1);
}

#[test]
fn empty_bindings_render_only_the_palette_suffix_when_supported() {
    let theme = Theme::default_theme();
    let layout = compose(&[], true, &theme, MIN_WIDTH);

    assert!(layout.left.is_empty());
    assert_eq!(text(&layout.right), PALETTE_SUFFIX);
    assert_eq!(layout.gap, MIN_WIDTH - PALETTE_SUFFIX.len() as u16);
}

#[test]
fn render_still_paints_the_row_below_minimum_width() {
    let theme = Theme::default_theme();
    let backend = TestBackend::new(MIN_WIDTH - 1, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render(frame, frame.area(), SAMPLE, true, &theme))
        .unwrap();

    let buffer = terminal.backend().buffer();
    for x in 0..buffer.area.width {
        let cell = &buffer[(x, 0)];
        assert_eq!(cell.symbol(), " ");
        assert_eq!(cell.bg, theme.hint_bar_bg);
    }
}

fn render_app(app: &mut App, width: u16, height: u16) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    terminal
}

fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for x in 0..buffer.area.width {
        text.push_str(buffer[(x, y)].symbol());
    }
    text
}

#[test]
fn app_renders_hint_row_above_status_in_normal_and_maximized_layouts() {
    const WIDTH: u16 = 100;
    const HEIGHT: u16 = 10;

    for maximized in [None, Some(FocusedPane::Query)] {
        let mut app = common::test_app();
        app.maximized = maximized;

        let terminal = render_app(&mut app, WIDTH, HEIGHT);

        assert!(
            row_text(&terminal, HEIGHT - 2).starts_with("i Enter Insert mode"),
            "missing hint row for {maximized:?}",
        );
        assert!(row_text(&terminal, HEIGHT - 1).starts_with(" NORMAL"));
    }
}

#[test]
fn picker_renders_common_hint_and_status_rows() {
    const WIDTH: u16 = 100;
    const HEIGHT: u16 = 10;

    let mut app = common::test_app();
    app.mode = Mode::Picker;

    let terminal = render_app(&mut app, WIDTH, HEIGHT);

    assert!(row_text(&terminal, HEIGHT - 2).starts_with("Up / Down Move selection"));
    assert!(row_text(&terminal, HEIGHT - 1).starts_with(" PICKER"));
}

#[test]
fn global_suffix_is_rendered_only_where_both_shortcuts_are_active() {
    const WIDTH: u16 = 500;
    const HEIGHT: u16 = 10;

    for (mode, expected) in [
        (Mode::QueryNormal, true),
        (Mode::Explorer, true),
        (Mode::Results, true),
        (Mode::QueryInsert, false),
        (Mode::Picker, false),
        (Mode::ThemePicker, false),
        (Mode::Help, false),
        (Mode::CellViewer, false),
        (Mode::HistoryPicker, false),
        (Mode::ResultsFilter, false),
    ] {
        let mut app = common::test_app();
        app.mode = mode;

        let terminal = render_app(&mut app, WIDTH, HEIGHT);
        let hint = row_text(&terminal, HEIGHT - 2);

        assert_eq!(
            hint.contains(PALETTE_SUFFIX),
            expected,
            "unexpected global suffix visibility in {mode:?}: {hint:?}",
        );
    }
}

#[test]
fn tiny_heights_prioritize_status_over_hint() {
    const WIDTH: u16 = 100;

    let mut app = common::test_app();
    let _ = render_app(&mut app, WIDTH, 0);

    let terminal = render_app(&mut app, WIDTH, 1);
    let only_row = row_text(&terminal, 0);

    assert!(
        only_row.starts_with(" NORMAL"),
        "missing status: {only_row:?}"
    );
    assert!(!only_row.contains("Insert mode"), "hint stole status row");
}

#[test]
fn hidden_hint_row_has_zero_height_in_normal_and_maximized_layouts() {
    const HEIGHT: u16 = 10;

    for (width, enabled, auto_hide_narrow) in [(100, false, false), (MIN_WIDTH - 1, true, true)] {
        for maximized in [None, Some(FocusedPane::Query)] {
            let mut app = common::test_app();
            app.app_config.hint_bar.enabled = enabled;
            app.app_config.hint_bar.auto_hide_narrow = auto_hide_narrow;
            app.maximized = maximized;

            let terminal = render_app(&mut app, width, HEIGHT);

            assert_eq!(
                terminal.backend().buffer()[(0, HEIGHT - 2)].symbol(),
                "└",
                "main pane did not reclaim hidden hint row for {maximized:?}",
            );
            assert!(row_text(&terminal, HEIGHT - 1).starts_with(" NORMAL"));
        }
    }
}
