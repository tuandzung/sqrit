use std::collections::HashSet;

use crate::db::types::SchemaInfo;

#[derive(Debug, Clone, PartialEq)]
pub enum TreeItem {
    Table {
        name: String,
        expanded: bool,
    },
    Column {
        table: String,
        name: String,
        data_type: String,
    },
    View {
        name: String,
        expanded: bool,
    },
    ViewColumn {
        view: String,
        name: String,
        data_type: String,
    },
}

pub struct ExplorerState {
    pub schema: Option<SchemaInfo>,
    pub expanded: HashSet<String>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub visible_rows: usize,
}

impl Default for ExplorerState {
    fn default() -> Self {
        Self {
            schema: None,
            expanded: HashSet::new(),
            selected: 0,
            scroll_offset: 0,
            visible_rows: 20,
        }
    }
}

impl ExplorerState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sync viewport size and reconcile `selected`/`scroll_offset` against the
    /// current item count. Call before rendering so layout-driven state lives
    /// outside the render path.
    pub fn set_viewport(&mut self, visible_rows: usize) {
        self.visible_rows = visible_rows;
        self.clamp_selection();
        self.adjust_scroll();
    }

    fn clamp_selection(&mut self) {
        let len = self.items().len();
        if len == 0 {
            self.selected = 0;
            self.scroll_offset = 0;
            return;
        }
        if self.selected >= len {
            self.selected = len - 1;
        }
    }

    pub fn adjust_scroll(&mut self) {
        let len = self.items().len();
        let max_scroll = len.saturating_sub(self.visible_rows);

        if self.visible_rows == 0 {
            self.scroll_offset = self.selected;
            return;
        }

        let bottom = self.scroll_offset.saturating_add(self.visible_rows);
        if self.selected >= bottom {
            self.scroll_offset = self.selected + 1 - self.visible_rows;
        } else if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }

        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
    }

    pub fn items(&self) -> Vec<TreeItem> {
        let mut items = Vec::new();
        if let Some(ref schema) = self.schema {
            for table in &schema.tables {
                let expanded = self.expanded.contains(&table.name);
                items.push(TreeItem::Table {
                    name: table.name.clone(),
                    expanded,
                });
                if expanded {
                    for col in &table.columns {
                        items.push(TreeItem::Column {
                            table: table.name.clone(),
                            name: col.name.clone(),
                            data_type: col.data_type.clone(),
                        });
                    }
                }
            }
            for view in &schema.views {
                let expanded = self.expanded.contains(&view.name);
                items.push(TreeItem::View {
                    name: view.name.clone(),
                    expanded,
                });
                if expanded {
                    for col in &view.columns {
                        items.push(TreeItem::ViewColumn {
                            view: view.name.clone(),
                            name: col.name.clone(),
                            data_type: col.data_type.clone(),
                        });
                    }
                }
            }
        }
        items
    }

    pub fn toggle(&mut self, name: &str) {
        if self.expanded.contains(name) {
            self.expanded.remove(name);
        } else {
            self.expanded.insert(name.to_string());
        }
    }

    pub fn move_down(&mut self) {
        let len = self.items().len();
        if len > 0 && self.selected + 1 < len {
            self.selected += 1;
        }
        self.adjust_scroll();
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.adjust_scroll();
    }
}
