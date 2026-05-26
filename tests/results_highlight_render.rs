use ratatui::style::Modifier;
use sqrit::results_render::render_cell;
use sqrit::theme::Theme;

#[test]
fn no_matches_single_raw_span() {
    let theme = Theme::default_theme();

    let spans = render_cell("abcdef", Vec::new(), &theme);

    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content, "abcdef");
    assert_eq!(spans[0].style, ratatui::style::Style::default());
}

#[test]
fn middle_match_three_spans_and_bold_underlined() {
    let theme = Theme::default_theme();

    let spans = render_cell("abcdef", std::iter::once(2..4), &theme);

    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].content, "ab");
    assert_eq!(spans[1].content, "cd");
    assert_eq!(spans[2].content, "ef");
    assert_eq!(spans[1].style.fg, Some(theme.border_focused));
    assert!(spans[1].style.add_modifier.contains(Modifier::BOLD));
    assert!(spans[1].style.add_modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn leading_and_trailing_match_omit_empty_spans() {
    let theme = Theme::default_theme();

    let leading = render_cell("abcdef", std::iter::once(0..2), &theme);
    assert_eq!(leading.len(), 2);
    assert_eq!(leading[0].content, "ab");
    assert_eq!(leading[1].content, "cdef");

    let trailing = render_cell("abcdef", std::iter::once(4..6), &theme);
    assert_eq!(trailing.len(), 2);
    assert_eq!(trailing[0].content, "abcd");
    assert_eq!(trailing[1].content, "ef");
}
