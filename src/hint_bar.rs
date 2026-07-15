//! Single-row, mode-aware keybinding hints above the status bar.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::mode::KeyBinding;
use crate::theme::Theme;

pub const PALETTE_SUFFIX: &str = "<sp> cmd  ? help";
pub const MIN_WIDTH: u16 = 40;

pub struct Layout {
    pub left: Vec<Span<'static>>,
    pub right: Vec<Span<'static>>,
    pub gap: u16,
}

pub fn compose(
    bindings: &[KeyBinding],
    show_global_suffix: bool,
    theme: &Theme,
    width: u16,
) -> Layout {
    if width < MIN_WIDTH {
        return Layout {
            left: vec![],
            right: vec![],
            gap: 0,
        };
    }

    let key_style = Style::default()
        .fg(theme.hint_bar_key)
        .add_modifier(Modifier::BOLD);
    let body_style = Style::default().fg(theme.hint_bar_fg);
    let separator_style = Style::default().fg(theme.hint_bar_separator);
    let width = usize::from(width);
    let suffix_width = if show_global_suffix {
        PALETTE_SUFFIX.chars().count()
    } else {
        0
    };

    if bindings.is_empty() {
        return Layout {
            left: vec![],
            right: if show_global_suffix {
                vec![Span::styled(PALETTE_SUFFIX.to_owned(), body_style)]
            } else {
                vec![]
            },
            gap: if show_global_suffix {
                (width - suffix_width) as u16
            } else {
                0
            },
        };
    }

    let Some(take) = (1..=bindings.len())
        .rev()
        .find(|&take| render_left_text(&bindings[..take]).chars().count() <= width)
    else {
        return Layout {
            left: vec![Span::styled("…".to_owned(), body_style)],
            right: vec![],
            gap: (width - 1) as u16,
        };
    };

    let bindings = &bindings[..take];
    let left_width = render_left_text(bindings).chars().count();
    let with_suffix = show_global_suffix && left_width + 3 + suffix_width <= width;

    Layout {
        left: spans_for_bindings(bindings, key_style, body_style),
        right: if with_suffix {
            vec![
                Span::styled(" │ ".to_owned(), separator_style),
                Span::styled(PALETTE_SUFFIX.to_owned(), body_style),
            ]
        } else {
            vec![]
        },
        gap: if with_suffix {
            (width - left_width - 3 - suffix_width) as u16
        } else {
            (width - left_width) as u16
        },
    }
}

fn render_left_text(bindings: &[KeyBinding]) -> String {
    bindings
        .iter()
        .map(|binding| format!("{} {}", binding.key, binding.action))
        .collect::<Vec<_>>()
        .join("  ")
}

fn spans_for_bindings(
    bindings: &[KeyBinding],
    key_style: Style,
    body_style: Style,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (index, binding) in bindings.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("  ".to_owned(), body_style));
        }
        spans.push(Span::styled(binding.key.to_owned(), key_style));
        spans.push(Span::styled(" ".to_owned(), body_style));
        spans.push(Span::styled(binding.action.to_owned(), body_style));
    }
    spans
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    bindings: &[KeyBinding],
    show_global_suffix: bool,
    theme: &Theme,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let layout = compose(bindings, show_global_suffix, theme, area.width);
    let mut spans = layout.left;
    if !layout.right.is_empty() {
        if layout.gap > 0 {
            spans.push(Span::raw(" ".repeat(layout.gap as usize)));
        }
        spans.extend(layout.right);
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.hint_bar_bg)),
        area,
    );
}
