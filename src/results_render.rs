use std::ops::Range;

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use crate::filter::FilterHit;
use crate::theme::Theme;

pub fn matched_ranges_for(
    hits: &[FilterHit],
    row_index: usize,
    col_index: usize,
) -> impl Iterator<Item = Range<usize>> + '_ {
    hits.iter()
        .find(move |hit| hit.row_index == row_index)
        .into_iter()
        .flat_map(move |hit| {
            hit.matches
                .iter()
                .filter(move |(hit_col_index, _)| *hit_col_index == col_index)
                .map(|(_, range)| range.clone())
        })
}

pub fn render_cell(
    text: &str,
    matched_ranges: impl IntoIterator<Item = Range<usize>>,
    theme: &Theme,
) -> Vec<Span<'static>> {
    let ranges: Vec<Range<usize>> = matched_ranges.into_iter().collect();
    if ranges.is_empty() {
        return vec![Span::raw(text.to_string())];
    }

    let chars: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();
    let mut cursor = 0usize;
    let highlight_style = Style::default()
        .fg(theme.border_focused)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

    for range in ranges {
        let start = range.start.min(chars.len());
        let end = range.end.min(chars.len());
        if start >= end {
            continue;
        }
        if cursor < start {
            spans.push(Span::raw(chars[cursor..start].iter().collect::<String>()));
        }
        spans.push(Span::styled(
            chars[start..end].iter().collect::<String>(),
            highlight_style,
        ));
        cursor = end;
    }

    if cursor < chars.len() {
        spans.push(Span::raw(chars[cursor..].iter().collect::<String>()));
    }

    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }
    spans
}
