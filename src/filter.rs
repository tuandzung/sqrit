//! Fuzzy filter for Results rows.
//!
//! Replaces the v0.2 case-insensitive substring filter with `nucleo-matcher`'s
//! subsequence scorer. Operates on the currently loaded page only — V6 holds.

use std::ops::Range;

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{chars::graphemes, Config, Matcher, Utf32Str};

use crate::db::types::{QueryResult, ResultColumn, Row};

/// Cached matcher reused across keystrokes so the scratch buffers inside
/// `nucleo_matcher::Matcher` aren't reallocated per character.
pub struct FuzzyFilter {
    matcher: Matcher,
    haystack_buf: Vec<char>,
    indices: Vec<u32>,
}

impl Default for FuzzyFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl FuzzyFilter {
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
            haystack_buf: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Score every row in `result` against `query`. Returns hits ordered by
    /// descending score; ties broken by original row index for stability.
    ///
    /// An empty `query` returns one `FilterHit` per row in original order with
    /// `score = 0` and `matches = []`.
    pub fn rank(&mut self, result: &QueryResult, query: &str) -> Vec<FilterHit> {
        let query = query.trim();
        if query.is_empty() {
            return (0..result.rows.len())
                .map(|row_index| FilterHit {
                    row_index,
                    score: 0,
                    matches: Vec::new(),
                })
                .collect();
        }

        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut hits = Vec::new();
        for (row_index, row) in result.rows.iter().enumerate() {
            if let Some(hit) = self.score_row(row_index, row, &result.columns, &pattern) {
                hits.push(hit);
            }
        }
        hits.sort_by(|a, b| b.score.cmp(&a.score).then(a.row_index.cmp(&b.row_index)));
        hits
    }

    fn score_row(
        &mut self,
        row_index: usize,
        row: &Row,
        columns: &[ResultColumn],
        pattern: &Pattern,
    ) -> Option<FilterHit> {
        let mut total: u32 = 0;
        let mut matches: Vec<(usize, Range<usize>)> = Vec::new();
        for (col_idx, col) in columns.iter().enumerate() {
            let Some(value) = row.get(&col.name) else {
                continue;
            };
            let text = value.to_string();
            self.haystack_buf.clear();
            let haystack = if text.is_ascii() {
                Utf32Str::Ascii(text.as_bytes())
            } else {
                self.haystack_buf.extend(graphemes(&text));
                Utf32Str::Unicode(&self.haystack_buf)
            };
            self.indices.clear();
            if let Some(score) = pattern.indices(haystack, &mut self.matcher, &mut self.indices) {
                total = total.saturating_add(score);
                self.indices.sort_unstable();
                self.indices.dedup();
                for range in contiguous_ranges(&self.indices) {
                    matches.push((col_idx, range));
                }
            }
        }
        if total == 0 {
            return None;
        }
        Some(FilterHit {
            row_index,
            score: total,
            matches,
        })
    }
}

/// One scored row. `matches` is `(column index, grapheme range)`; the range is
/// in grapheme units of the column's `Display` rendering.
#[derive(Debug, Clone)]
pub struct FilterHit {
    pub row_index: usize,
    pub score: u32,
    pub matches: Vec<(usize, Range<usize>)>,
}

/// Collapse a sorted vector of single-char positions into contiguous ranges.
/// e.g. `[0,1,2,5,6]` → `[0..3, 5..7]`.
fn contiguous_ranges(positions: &[u32]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut iter = positions.iter().copied();
    let Some(first) = iter.next() else {
        return out;
    };
    let mut start = first as usize;
    let mut prev = first;
    for p in iter {
        if p == prev + 1 {
            prev = p;
        } else {
            out.push(start..(prev as usize + 1));
            start = p as usize;
            prev = p;
        }
    }
    out.push(start..(prev as usize + 1));
    out
}
