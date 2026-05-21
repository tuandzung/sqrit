#[derive(Debug, Clone)]
struct Snapshot {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
}

#[derive(Debug)]
pub struct EditorBuffer {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    undo_stack: Vec<Snapshot>,
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            undo_stack: Vec::new(),
        }
    }
}

impl EditorBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    pub fn replace_all(&mut self, s: &str) {
        self.save_undo();
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        for c in s.chars() {
            if c == '\n' {
                self.lines.push(String::new());
                self.cursor_row += 1;
                self.cursor_col = 0;
            } else {
                self.lines[self.cursor_row].push(c);
                self.cursor_col += 1;
            }
        }
    }

    pub fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\n' {
                self.insert_newline();
            } else {
                self.insert_char(c);
            }
        }
    }

    fn save_undo(&mut self) {
        let snapshot = Snapshot {
            lines: self.lines.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
        };
        self.undo_stack.push(snapshot);
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some(snapshot) = self.undo_stack.pop() {
            self.lines = snapshot.lines;
            self.cursor_row = snapshot.cursor_row;
            self.cursor_col = snapshot.cursor_col;
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.save_undo();
        self.lines[self.cursor_row].insert(self.cursor_col, c);
        self.cursor_col += 1;
    }

    pub fn backspace(&mut self) {
        self.save_undo();
        if self.cursor_col > 0 {
            self.lines[self.cursor_row].remove(self.cursor_col - 1);
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            let prev_len = self.lines[self.cursor_row - 1].len();
            let current_line = self.lines.remove(self.cursor_row);
            self.lines[self.cursor_row - 1].push_str(&current_line);
            self.cursor_row -= 1;
            self.cursor_col = prev_len;
        }
    }

    pub fn delete_backwards(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        let take = n.min(self.cursor_col);
        if take == 0 {
            return;
        }
        self.save_undo();
        let start = self.cursor_col - take;
        self.lines[self.cursor_row].drain(start..self.cursor_col);
        self.cursor_col = start;
    }

    pub fn cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    pub fn cursor_right(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        }
    }

    pub fn cursor_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.clamp_col();
        }
    }

    pub fn cursor_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.clamp_col();
        }
    }

    pub fn home(&mut self) {
        self.cursor_col = 0;
    }

    pub fn end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].len();
    }

    pub fn insert_newline(&mut self) {
        self.save_undo();
        let rest: String = self.lines[self.cursor_row]
            .drain(self.cursor_col..)
            .collect();
        self.lines.insert(self.cursor_row + 1, rest);
        self.cursor_row += 1;
        self.cursor_col = 0;
    }

    pub fn word_forward(&mut self) {
        let line = &self.lines[self.cursor_row];
        let mut col = self.cursor_col;
        let chars: Vec<char> = line.chars().collect();

        // Skip current word
        while col < chars.len() && !chars[col].is_whitespace() {
            col += 1;
        }
        // Skip whitespace
        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }

        self.cursor_col = col;
    }

    pub fn word_backward(&mut self) {
        let line = &self.lines[self.cursor_row];
        let mut col = self.cursor_col;
        let chars: Vec<char> = line.chars().collect();

        if col == 0 {
            return;
        }

        // Skip whitespace before cursor
        col -= 1;
        while col > 0 && chars[col].is_whitespace() {
            col -= 1;
        }
        // Skip back over word
        while col > 0 && !chars[col - 1].is_whitespace() {
            col -= 1;
        }

        self.cursor_col = col;
    }

    pub fn delete_char(&mut self) {
        self.save_undo();
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.lines[self.cursor_row].remove(self.cursor_col);
        }
    }

    pub fn delete_line(&mut self) -> Option<String> {
        if self.lines.len() == 1 {
            let line = self.lines[0].clone();
            if line.is_empty() {
                return None;
            }
            self.save_undo();
            self.lines[0] = String::new();
            self.cursor_col = 0;
            return Some(line);
        }
        self.save_undo();
        let line = self.lines.remove(self.cursor_row);
        if self.cursor_row >= self.lines.len() {
            self.cursor_row = self.lines.len() - 1;
        }
        self.clamp_col();
        Some(line)
    }

    pub fn yank_line(&self) -> String {
        self.lines[self.cursor_row].clone()
    }

    pub fn paste_below(&mut self, line: &str) {
        self.save_undo();
        self.lines.insert(self.cursor_row + 1, line.to_string());
        self.cursor_row += 1;
        self.cursor_col = 0;
    }

    pub fn go_top(&mut self) {
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    pub fn go_bottom(&mut self) {
        self.cursor_row = self.lines.len() - 1;
        self.cursor_col = 0;
    }

    fn clamp_col(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }
}
