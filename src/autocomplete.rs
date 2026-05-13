pub fn current_word_prefix(text: &str, row: usize, col: usize) -> String {
    let line = match text.lines().nth(row) {
        Some(l) => l,
        None => return String::new(),
    };
    let chars: Vec<char> = line.chars().collect();
    if col == 0 || col > chars.len() {
        return String::new();
    }
    let mut start = col;
    while start > 0 && (chars[start - 1].is_ascii_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    chars[start..col].iter().collect()
}

fn prefix_matches(candidate: &str, prefix: &str) -> bool {
    candidate.len() >= prefix.len()
        && candidate[..prefix.len()].eq_ignore_ascii_case(prefix)
}

pub fn suggest(prefix: &str, schema: Option<&crate::db::types::SchemaInfo>) -> Vec<String> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut results: Vec<String> = Vec::new();

    for kw in crate::sql::keywords() {
        if prefix_matches(kw, prefix) && seen.insert(kw.to_string()) {
            results.push(kw.to_string());
        }
    }
    for ty in crate::sql::types() {
        if prefix_matches(ty, prefix) && seen.insert(ty.to_string()) {
            results.push(ty.to_string());
        }
    }

    if let Some(schema) = schema {
        add_schema_items(&schema.tables.iter().map(|t| (t.name.as_str(), t.columns.as_slice())).collect::<Vec<_>>(), prefix, &mut seen, &mut results);
        add_schema_items(&schema.views.iter().map(|v| (v.name.as_str(), v.columns.as_slice())).collect::<Vec<_>>(), prefix, &mut seen, &mut results);
    }

    results
}

fn add_schema_items<'a>(
    items: &[(&'a str, &'a [crate::db::types::ColumnInfo])],
    prefix: &str,
    seen: &mut std::collections::HashSet<String>,
    results: &mut Vec<String>,
) {
    for (name, columns) in items {
        if prefix_matches(name, prefix) && seen.insert(name.to_string()) {
            results.push(name.to_string());
        }
        for col in *columns {
            if prefix_matches(&col.name, prefix) && seen.insert(col.name.clone()) {
                results.push(col.name.clone());
            }
        }
    }
}

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
