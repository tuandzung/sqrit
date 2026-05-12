pub struct ResultsState {
    pub selected_row: usize,
    pub selected_col: usize,
    pub scroll_row: usize,
    pub visible_rows: usize,
}

impl ResultsState {
    pub fn new() -> Self {
        Self {
            selected_row: 0,
            selected_col: 0,
            scroll_row: 0,
            visible_rows: 20,
        }
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

    fn adjust_scroll(&mut self) {
        let bottom = self.scroll_row + self.visible_rows;
        if self.selected_row >= bottom {
            self.scroll_row = self.selected_row - self.visible_rows + 1;
        } else if self.selected_row < self.scroll_row {
            self.scroll_row = self.selected_row;
        }
    }
}
