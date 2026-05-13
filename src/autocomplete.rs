pub struct AutocompleteState {
    candidates: Vec<String>,
    filtered: Vec<String>,
    selected: usize,
    visible: bool,
}

impl AutocompleteState {
    pub fn new() -> Self {
        Self {
            candidates: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            visible: false,
        }
    }

    pub fn open(&mut self, candidates: Vec<String>) {
        self.candidates = candidates.clone();
        self.filtered = candidates;
        self.selected = 0;
        self.visible = true;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn filtered(&self) -> &[String] {
        &self.filtered
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
        let choice = self.filtered.get(self.selected).cloned();
        self.visible = false;
        choice
    }

    pub fn next(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.filtered.len();
    }

    pub fn prev(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected = (self.selected + self.filtered.len() - 1) % self.filtered.len();
    }

    pub fn filter(&mut self, prefix: &str) {
        let lower = prefix.to_lowercase();
        self.filtered = self
            .candidates
            .iter()
            .filter(|c| c.to_lowercase().starts_with(&lower))
            .cloned()
            .collect();
        self.selected = 0;
    }
}
