use crate::db::types::{QueryResult, ResultColumn, Row};

pub fn row_matches(row: &Row, columns: &[ResultColumn], filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    row_matches_lowered(row, columns, &filter.to_lowercase())
}

fn row_matches_lowered(row: &Row, columns: &[ResultColumn], needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    columns.iter().any(|c| {
        row.get(&c.name)
            .map(|v| v.to_string().to_lowercase().contains(needle_lower))
            .unwrap_or(false)
    })
}

pub struct ResultsState {
    pub selected_row: usize,
    pub selected_col: usize,
    pub scroll_row: usize,
    pub visible_rows: usize,
    pub page_size: usize,
    pub page_offset: usize,
    pub has_next_page: bool,
    pub pending_yank: bool,
    pub pending_comma: bool,
    pub filter: Option<String>,
}

impl Default for ResultsState {
    fn default() -> Self {
        Self {
            selected_row: 0,
            selected_col: 0,
            scroll_row: 0,
            visible_rows: 20,
            page_size: 50,
            page_offset: 0,
            has_next_page: false,
            pending_yank: false,
            pending_comma: false,
            filter: None,
        }
    }
}

impl ResultsState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn page_down(&mut self) {
        self.page_offset += self.page_size;
        self.selected_row = 0;
        self.selected_col = 0;
        self.scroll_row = 0;
    }

    pub fn page_up(&mut self) {
        if self.page_offset == 0 {
            return;
        }
        self.page_offset = self.page_offset.saturating_sub(self.page_size);
        self.selected_row = 0;
        self.selected_col = 0;
        self.scroll_row = 0;
    }

    pub fn reset_pagination(&mut self) {
        self.page_offset = 0;
        self.selected_row = 0;
        self.selected_col = 0;
        self.scroll_row = 0;
        self.has_next_page = false;
    }

    pub fn move_down(&mut self, total_rows: usize) {
        if total_rows == 0 {
            return;
        }
        if self.selected_row + 1 < total_rows {
            self.selected_row += 1;
        }
        self.adjust_scroll();
    }

    pub fn move_up(&mut self) {
        if self.selected_row > 0 {
            self.selected_row -= 1;
        }
        self.adjust_scroll();
    }

    pub fn move_right(&mut self, total_cols: usize) {
        if total_cols == 0 {
            return;
        }
        if self.selected_col + 1 < total_cols {
            self.selected_col += 1;
        }
    }

    pub fn move_left(&mut self) {
        if self.selected_col > 0 {
            self.selected_col -= 1;
        }
    }

    pub fn visible_row_indices(&self, result: &QueryResult) -> Vec<usize> {
        let Some(term) = self.filter.as_deref() else {
            return (0..result.rows.len()).collect();
        };
        let needle = term.to_lowercase();
        result
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| row_matches_lowered(r, &result.columns, &needle))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn move_down_visible(&mut self, result: &QueryResult) {
        let visible = self.visible_row_indices(result);
        if let Some(pos) = visible.iter().position(|&i| i == self.selected_row) {
            if pos + 1 < visible.len() {
                self.selected_row = visible[pos + 1];
            }
        } else if let Some(&first) = visible.first() {
            self.selected_row = first;
        }
        self.adjust_scroll_to(&visible);
    }

    pub fn move_up_visible(&mut self, result: &QueryResult) {
        let visible = self.visible_row_indices(result);
        if let Some(pos) = visible.iter().position(|&i| i == self.selected_row) {
            if pos > 0 {
                self.selected_row = visible[pos - 1];
            }
        } else if let Some(&first) = visible.first() {
            self.selected_row = first;
        }
        self.adjust_scroll_to(&visible);
    }

    pub fn snap_selection_to_visible(&mut self, result: &QueryResult) {
        let visible = self.visible_row_indices(result);
        if visible.is_empty() {
            self.selected_row = 0;
            self.scroll_row = 0;
            return;
        }
        if !visible.contains(&self.selected_row) {
            self.selected_row = visible[0];
            self.scroll_row = 0;
        }
        self.adjust_scroll_to(&visible);
    }

    fn adjust_scroll_to(&mut self, visible: &[usize]) {
        let Some(pos) = visible.iter().position(|&i| i == self.selected_row) else {
            self.scroll_row = 0;
            return;
        };
        let max_scroll = visible.len().saturating_sub(self.visible_rows);
        let bottom = self.scroll_row + self.visible_rows;
        if pos >= bottom {
            self.scroll_row = pos + 1 - self.visible_rows;
        } else if pos < self.scroll_row {
            self.scroll_row = pos;
        }
        if self.scroll_row > max_scroll {
            self.scroll_row = max_scroll;
        }
    }

    fn adjust_scroll(&mut self) {
        let bottom = self.scroll_row + self.visible_rows;
        if self.selected_row >= bottom {
            self.scroll_row = self.selected_row - self.visible_rows + 1;
        } else if self.selected_row < self.scroll_row {
            self.scroll_row = self.selected_row;
        }
    }
}
