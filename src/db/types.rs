use std::collections::HashMap;

pub type Row = HashMap<String, Value>;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Integer(i64),
    Float(f64),
    Text(String),
    Blob(Vec<u8>),
    Boolean(bool),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "NULL"),
            Value::Integer(i) => write!(f, "{i}"),
            Value::Float(fl) => write!(f, "{fl}"),
            Value::Text(s) => write!(f, "{s}"),
            Value::Blob(b) => write!(f, "<blob {} bytes>", b.len()),
            Value::Boolean(b) => write!(f, "{b}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResultColumn {
    pub name: String,
    pub data_type: Option<String>,
}

impl ResultColumn {
    pub fn untyped(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<ResultColumn>,
    pub rows: Vec<Row>,
    pub rows_affected: Option<u64>,
    pub total_count: Option<u64>,
}

impl QueryResult {
    pub fn empty() -> Self {
        Self {
            columns: vec![],
            rows: vec![],
            rows_affected: None,
            total_count: None,
        }
    }

    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(|c| c.name.as_str()).collect()
    }
}

pub fn sqlx_result_columns<C>(cols: &[C]) -> Vec<ResultColumn>
where
    C: sqlx::Column,
{
    use sqlx::TypeInfo;
    cols.iter()
        .map(|c| ResultColumn {
            name: c.name().to_string(),
            data_type: Some(c.type_info().name().to_string()),
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone)]
pub struct SchemaInfo {
    pub namespaces: Vec<Namespace>,
}

#[derive(Debug, Clone)]
pub struct Namespace {
    /// PostgreSQL schema, MySQL database, or an empty string for SQLite.
    pub name: String,
    pub tables: Vec<TableObject>,
    pub views: Vec<ViewObject>,
    pub materialized_views: Vec<ViewObject>,
    pub indexes: Vec<IndexObject>,
    pub triggers: Vec<TriggerObject>,
    pub functions: Vec<RoutineObject>,
    pub procedures: Vec<RoutineObject>,
    pub sequences: Vec<SequenceObject>,
}

impl Namespace {
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tables: vec![],
            views: vec![],
            materialized_views: vec![],
            indexes: vec![],
            triggers: vec![],
            functions: vec![],
            procedures: vec![],
            sequences: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableObject {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone)]
pub struct ViewObject {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone)]
pub struct IndexObject {
    pub name: String,
    pub table: String,
    pub unique: bool,
}

#[derive(Debug, Clone)]
pub struct TriggerObject {
    pub name: String,
    pub table: String,
    /// Adapter-specific event text, such as `INSERT` or `BEFORE UPDATE`.
    pub event: String,
}

#[derive(Debug, Clone)]
pub struct RoutineObject {
    pub name: String,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SequenceObject {
    pub name: String,
    pub last_value: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectKind {
    Table,
    View,
    MaterializedView,
    Index,
    Trigger,
    Function,
    Procedure,
    Sequence,
}

impl ObjectKind {
    pub fn group_label(self) -> &'static str {
        match self {
            Self::Table => "Tables",
            Self::View => "Views",
            Self::MaterializedView => "Materialized Views",
            Self::Index => "Indexes",
            Self::Trigger => "Triggers",
            Self::Function => "Functions",
            Self::Procedure => "Procedures",
            Self::Sequence => "Sequences",
        }
    }

    pub fn supports_select_star(self) -> bool {
        matches!(self, Self::Table | Self::View | Self::MaterializedView)
    }
}

pub type TableRow = Row;
