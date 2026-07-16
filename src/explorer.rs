use std::collections::HashSet;

use crate::db::types::{ColumnInfo, ObjectKind, SchemaInfo};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeKey {
    Namespace(String),
    Group {
        ns: String,
        kind: ObjectKind,
    },
    Object {
        ns: String,
        kind: ObjectKind,
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeItem {
    Namespace {
        name: String,
        expanded: bool,
    },
    Group {
        ns: String,
        kind: ObjectKind,
        expanded: bool,
        count: usize,
    },
    Object {
        ns: String,
        kind: ObjectKind,
        name: String,
        expanded: bool,
    },
    Column {
        ns: String,
        kind: ObjectKind,
        parent: String,
        name: String,
        data_type: String,
    },
}

impl TreeItem {
    pub fn key(&self) -> Option<NodeKey> {
        match self {
            Self::Namespace { name, .. } => Some(NodeKey::Namespace(name.clone())),
            Self::Group { ns, kind, .. } => Some(NodeKey::Group {
                ns: ns.clone(),
                kind: *kind,
            }),
            Self::Object { ns, kind, name, .. } if kind.supports_select_star() => {
                Some(NodeKey::Object {
                    ns: ns.clone(),
                    kind: *kind,
                    name: name.clone(),
                })
            }
            Self::Object { .. } | Self::Column { .. } => None,
        }
    }
}

pub struct ExplorerState {
    pub schema: Option<SchemaInfo>,
    pub expanded: HashSet<NodeKey>,
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

    pub fn set_schema(&mut self, schema: SchemaInfo) {
        self.expanded.clear();
        if schema.namespaces.len() == 1 {
            self.expanded
                .insert(NodeKey::Namespace(schema.namespaces[0].name.clone()));
        }
        self.schema = Some(schema);
        self.selected = 0;
        self.scroll_offset = 0;
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
        let Some(schema) = self.schema.as_ref() else {
            return items;
        };
        let single_namespace = schema.namespaces.len() == 1;

        for namespace in &schema.namespaces {
            let namespace_key = NodeKey::Namespace(namespace.name.clone());
            let namespace_expanded = single_namespace || self.expanded.contains(&namespace_key);
            if !single_namespace {
                items.push(TreeItem::Namespace {
                    name: namespace.name.clone(),
                    expanded: namespace_expanded,
                });
            }
            if !namespace_expanded {
                continue;
            }

            self.push_column_group(
                &mut items,
                &namespace.name,
                ObjectKind::Table,
                namespace
                    .tables
                    .iter()
                    .map(|object| (object.name.as_str(), object.columns.as_slice())),
            );
            self.push_column_group(
                &mut items,
                &namespace.name,
                ObjectKind::View,
                namespace
                    .views
                    .iter()
                    .map(|object| (object.name.as_str(), object.columns.as_slice())),
            );
            self.push_column_group(
                &mut items,
                &namespace.name,
                ObjectKind::MaterializedView,
                namespace
                    .materialized_views
                    .iter()
                    .map(|object| (object.name.as_str(), object.columns.as_slice())),
            );
            self.push_leaf_group(
                &mut items,
                &namespace.name,
                ObjectKind::Index,
                namespace.indexes.iter().map(|object| object.name.as_str()),
            );
            self.push_leaf_group(
                &mut items,
                &namespace.name,
                ObjectKind::Trigger,
                namespace.triggers.iter().map(|object| object.name.as_str()),
            );
            self.push_leaf_group(
                &mut items,
                &namespace.name,
                ObjectKind::Function,
                namespace
                    .functions
                    .iter()
                    .map(|object| object.name.as_str()),
            );
            self.push_leaf_group(
                &mut items,
                &namespace.name,
                ObjectKind::Procedure,
                namespace
                    .procedures
                    .iter()
                    .map(|object| object.name.as_str()),
            );
            self.push_leaf_group(
                &mut items,
                &namespace.name,
                ObjectKind::Sequence,
                namespace
                    .sequences
                    .iter()
                    .map(|object| object.name.as_str()),
            );
        }
        items
    }

    fn push_column_group<'a>(
        &self,
        items: &mut Vec<TreeItem>,
        namespace: &str,
        kind: ObjectKind,
        objects: impl IntoIterator<Item = (&'a str, &'a [ColumnInfo])>,
    ) {
        let objects = objects.into_iter().collect::<Vec<_>>();
        if objects.is_empty() {
            return;
        }
        let group_key = NodeKey::Group {
            ns: namespace.to_string(),
            kind,
        };
        let group_expanded = self.expanded.contains(&group_key);
        items.push(TreeItem::Group {
            ns: namespace.to_string(),
            kind,
            expanded: group_expanded,
            count: objects.len(),
        });
        if !group_expanded {
            return;
        }

        for (name, columns) in objects {
            let object_key = NodeKey::Object {
                ns: namespace.to_string(),
                kind,
                name: name.to_string(),
            };
            let object_expanded = self.expanded.contains(&object_key);
            items.push(TreeItem::Object {
                ns: namespace.to_string(),
                kind,
                name: name.to_string(),
                expanded: object_expanded,
            });
            if object_expanded {
                items.extend(columns.iter().map(|column| TreeItem::Column {
                    ns: namespace.to_string(),
                    kind,
                    parent: name.to_string(),
                    name: column.name.clone(),
                    data_type: column.data_type.clone(),
                }));
            }
        }
    }

    fn push_leaf_group<'a>(
        &self,
        items: &mut Vec<TreeItem>,
        namespace: &str,
        kind: ObjectKind,
        names: impl IntoIterator<Item = &'a str>,
    ) {
        let names = names.into_iter().collect::<Vec<_>>();
        if names.is_empty() {
            return;
        }
        let group_key = NodeKey::Group {
            ns: namespace.to_string(),
            kind,
        };
        let group_expanded = self.expanded.contains(&group_key);
        items.push(TreeItem::Group {
            ns: namespace.to_string(),
            kind,
            expanded: group_expanded,
            count: names.len(),
        });
        if group_expanded {
            items.extend(names.into_iter().map(|name| TreeItem::Object {
                ns: namespace.to_string(),
                kind,
                name: name.to_string(),
                expanded: false,
            }));
        }
    }

    pub fn toggle_key(&mut self, key: NodeKey) {
        if !self.expanded.remove(&key) {
            self.expanded.insert(key);
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
