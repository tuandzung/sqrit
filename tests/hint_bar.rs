use ratatui::backend::TestBackend;
use ratatui::style::Modifier;
use ratatui::text::Span;
use ratatui::Terminal;
use sqrit::hint_bar::{compose, render, MIN_WIDTH, PALETTE_SUFFIX};
use sqrit::mode::KeyBinding;
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
    let layout = compose(SAMPLE, &theme, 120);

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
        let layout = compose(SAMPLE, &theme, width);
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
    let layout = compose(SAMPLE, &theme, MIN_WIDTH);

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
    let layout = compose(LONG_BINDINGS, &theme, MIN_WIDTH);

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
    let layout = compose(BINDINGS, &theme, MIN_WIDTH);

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
    let layout = compose(TOO_WIDE, &theme, MIN_WIDTH);

    assert_eq!(text(&layout.left), "…");
    assert!(layout.right.is_empty());
    assert_eq!(layout.gap, MIN_WIDTH - 1);
}

#[test]
fn empty_bindings_render_only_the_palette_suffix_when_supported() {
    let theme = Theme::default_theme();
    let layout = compose(&[], &theme, MIN_WIDTH);

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
        .draw(|frame| render(frame, frame.area(), SAMPLE, &theme))
        .unwrap();

    let buffer = terminal.backend().buffer();
    for x in 0..buffer.area.width {
        let cell = &buffer[(x, 0)];
        assert_eq!(cell.symbol(), " ");
        assert_eq!(cell.bg, theme.hint_bar_bg);
    }
}
