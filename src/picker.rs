use crate::app::App;

#[derive(Default)]
pub struct PickerState {
    pub selected: usize,
    pub filter: String,
}

impl PickerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn filtered_indices(&self, app: &App) -> Vec<usize> {
        app.config
            .connections
            .iter()
            .enumerate()
            .filter(|(_, conn)| {
                if self.filter.is_empty() {
                    true
                } else {
                    conn.name
                        .to_lowercase()
                        .contains(&self.filter.to_lowercase())
                }
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn selected_connection(&self, app: &App) -> Option<usize> {
        self.filtered_indices(app).into_iter().nth(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self, filtered_count: usize) {
        if self.selected + 1 < filtered_count {
            self.selected += 1;
        }
    }

    pub fn type_char(&mut self, c: char, filtered_count: usize) {
        self.filter.push(c);
        self.clamp_selected(filtered_count);
    }

    pub fn backspace(&mut self, filtered_count: usize) {
        self.filter.pop();
        self.clamp_selected(filtered_count);
    }

    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.selected = 0;
    }

    pub fn clamp_selected(&mut self, count: usize) {
        if count == 0 {
            self.selected = 0;
        } else if self.selected >= count {
            self.selected = count - 1;
        }
    }
}
