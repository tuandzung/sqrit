pub struct AutocompleteState {
    candidates: Vec<String>,
    filtered_indices: Vec<usize>,
    selected: usize,
    visible: bool,
}

impl AutocompleteState {
    pub fn new() -> Self {
        Self {
            candidates: Vec::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            visible: false,
        }
    }

    pub fn open(&mut self, candidates: Vec<String>) {
        if candidates.is_empty() {
            return;
        }
        self.candidates = candidates;
        self.filtered_indices = (0..self.candidates.len()).collect();
        self.selected = 0;
        self.visible = true;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn filtered(&self) -> Vec<&str> {
        self.filtered_indices
            .iter()
            .map(|&i| self.candidates[i].as_str())
            .collect()
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn dismiss(&mut self) {
        self.visible = false;
    }

    pub fn accept(&mut self) -> Option<String> {
        if !self.visible {
            return None;
        }
        let choice = self
            .filtered_indices
            .get(self.selected)
            .map(|&i| self.candidates[i].clone());
        self.visible = false;
        choice
    }

    pub fn next(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.filtered_indices.len();
    }

    pub fn prev(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        self.selected = (self.selected + self.filtered_indices.len() - 1) % self.filtered_indices.len();
    }

    pub fn filter(&mut self, prefix: &str) {
        let lower = prefix.to_lowercase();
        self.filtered_indices = self
            .candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| c.to_lowercase().starts_with(&lower))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }
}
