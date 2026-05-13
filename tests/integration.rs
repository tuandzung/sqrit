use sqrit::db::mysql::MySqlAdapter;
use sqrit::db::postgres::PgAdapter;
use sqrit::db::sqlite::SqliteAdapter;
use sqrit::db::types::Value;
use sqrit::db::Database;

use std::net::{TcpStream, SocketAddr};
use std::time::Duration;

fn port_reachable(host: &str, port: u16) -> bool {
    use std::net::ToSocketAddrs;
    let addr = format!("{}:{}", host, port);
    let addr: SocketAddr = addr.to_socket_addrs().unwrap().next().unwrap();
    TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok()
}

/// Wrapper to hold a connected Database impl + optional temp file handle.
struct TestDb {
    db: Box<dyn Database>,
    _file: Option<tempfile::NamedTempFile>,
}

fn sqlite_ctx() -> TestDb {
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut adapter = SqliteAdapter::new(&path);
    rt.block_on(adapter.connect()).unwrap();
    TestDb {
        db: Box::new(adapter),
        _file: Some(file),
    }
}

fn pg_ctx() -> Option<TestDb> {
    if !port_reachable("localhost", 15432) {
        return None;
    }
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sqrit:sqrit@localhost:15432/sqrit_test".to_string());
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut adapter = PgAdapter::new(&url);
    match rt.block_on(adapter.connect()) {
        Ok(()) => Some(TestDb {
            db: Box::new(adapter),
            _file: None,
        }),
        Err(_) => None,
    }
}

fn mysql_ctx() -> Option<TestDb> {
    if !port_reachable("localhost", 13306) {
        return None;
    }
    let url = std::env::var("MYSQL_URL")
        .unwrap_or_else(|_| "mysql://sqrit:sqrit@localhost:13306/sqrit_test".to_string());
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut adapter = MySqlAdapter::new(&url);
    match rt.block_on(adapter.connect()) {
        Ok(()) => Some(TestDb {
            db: Box::new(adapter),
            _file: None,
        }),
        Err(_) => None,
    }
}

fn unique_table(label: &str) -> String {
    format!("integ_{}_{}", label, std::process::id())
}

/// Run a closure against all available backends. Skip PG/MySQL if unavailable.
fn for_each_adapter<F>(f: F)
where
    F: Fn(&Box<dyn Database>, &str),
{
    let rt = tokio::runtime::Runtime::new().unwrap();

    // SQLite always available
    let ctx = sqlite_ctx();
    f(&ctx.db, "sqlite");

    // PG — skip if no server
    if let Some(ctx) = pg_ctx() {
        f(&ctx.db, "postgres");
    }

    // MySQL — skip if no server
    if let Some(ctx) = mysql_ctx() {
        f(&ctx.db, "mysql");
    }

    drop(rt);
}

#[test]
fn all_adapters_execute_select_literal() {
    for_each_adapter(|db, _label| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(db.execute("SELECT 1 AS val")).unwrap();
        assert_eq!(result.columns, vec!["val"]);
        assert_eq!(result.rows.len(), 1);
    });
}

#[test]
fn all_adapters_create_insert_select_drop() {
    for_each_adapter(|db, label| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let table = unique_table(&format!("cis_{}", label));

        let (create, insert, select) = match label {
            "sqlite" => (
                format!("CREATE TABLE \"{}\" (id INTEGER PRIMARY KEY, name TEXT NOT NULL)", table),
                format!("INSERT INTO \"{}\" (name) VALUES ('alice')", table),
                format!("SELECT id, name FROM \"{}\"", table),
            ),
            "postgres" => (
                format!("CREATE TABLE \"{}\" (id SERIAL PRIMARY KEY, name TEXT NOT NULL)", table),
                format!("INSERT INTO \"{}\" (name) VALUES ('alice')", table),
                format!("SELECT id, name FROM \"{}\" ORDER BY id", table),
            ),
            "mysql" => (
                format!("CREATE TABLE `{}` (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(255) NOT NULL)", table),
                format!("INSERT INTO `{}` (name) VALUES ('alice')", table),
                format!("SELECT id, name FROM `{}` ORDER BY id", table),
            ),
            _ => return,
        };

        rt.block_on(db.execute(&create)).unwrap();
        let ins = rt.block_on(db.execute(&insert)).unwrap();
        assert_eq!(ins.rows_affected, Some(1));

        let sel = rt.block_on(db.execute(&select)).unwrap();
        assert_eq!(sel.rows.len(), 1);
        assert_eq!(sel.columns, vec!["id", "name"]);

        let name = sel.rows[0].get("name").unwrap();
        assert_eq!(name, &Value::Text("alice".into()));

        let drop = match label {
            "mysql" => format!("DROP TABLE `{}`", table),
            _ => format!("DROP TABLE \"{}\"", table),
        };
        rt.block_on(db.execute(&drop)).unwrap();
    });
}

#[test]
fn all_adapters_paginate() {
    for_each_adapter(|db, label| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let table = unique_table(&format!("page_{}", label));

        let create = match label {
            "sqlite" => format!("CREATE TABLE \"{}\" (id INTEGER PRIMARY KEY, val TEXT)", table),
            "postgres" => format!("CREATE TABLE \"{}\" (id SERIAL PRIMARY KEY, val TEXT)", table),
            "mysql" => format!("CREATE TABLE `{}` (id INT AUTO_INCREMENT PRIMARY KEY, val VARCHAR(255))", table),
            _ => return,
        };

        rt.block_on(db.execute(&create)).unwrap();

        for i in 0..5 {
            let insert = match label {
                "mysql" => format!("INSERT INTO `{}` (val) VALUES ('v{}')", table, i),
                _ => format!("INSERT INTO \"{}\" (val) VALUES ('v{}')", table, i),
            };
            rt.block_on(db.execute(&insert)).unwrap();
        }

        let base_query = match label {
            "mysql" => format!("SELECT * FROM `{}` ORDER BY id", table),
            _ => format!("SELECT * FROM \"{}\" ORDER BY id", table),
        };

        let page = rt
            .block_on(db.execute_paginated(&base_query, 2, 2))
            .unwrap();
        assert_eq!(page.rows.len(), 2);

        let val = page.rows[0].get("val").unwrap();
        assert_eq!(val, &Value::Text("v2".into()));
    });
}

#[test]
fn all_adapters_list_tables_and_columns() {
    for_each_adapter(|db, label| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let table = unique_table(&format!("tbl_{}", label));

        let create = match label {
            "sqlite" => format!(
                "CREATE TABLE \"{}\" (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
                table
            ),
            "postgres" => format!(
                "CREATE TABLE \"{}\" (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
                table
            ),
            "mysql" => format!(
                "CREATE TABLE `{}` (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(255) NOT NULL)",
                table
            ),
            _ => return,
        };

        rt.block_on(db.execute(&create)).unwrap();

        let tables = rt.block_on(db.list_tables()).unwrap();
        assert!(tables.contains(&table), "{} should contain {}", label, table);

        let columns = rt.block_on(db.list_columns(&table)).unwrap();
        assert!(columns.len() >= 2, "{} columns should have >= 2, got {}", label, columns.len());

        let id_col = columns.iter().find(|c| c.name == "id").unwrap();
        assert!(id_col.is_primary_key);

        let name_col = columns.iter().find(|c| c.name == "name").unwrap();
        assert!(!name_col.nullable);
    });
}

#[test]
fn all_adapters_schema_info() {
    for_each_adapter(|db, label| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let table = unique_table(&format!("sch_{}", label));

        let create = match label {
            "sqlite" => format!("CREATE TABLE \"{}\" (id INTEGER PRIMARY KEY, v TEXT)", table),
            "postgres" => format!("CREATE TABLE \"{}\" (id SERIAL PRIMARY KEY, v TEXT)", table),
            "mysql" => format!("CREATE TABLE `{}` (id INT AUTO_INCREMENT PRIMARY KEY, v VARCHAR(255))", table),
            _ => return,
        };

        rt.block_on(db.execute(&create)).unwrap();

        let info = rt.block_on(db.schema_info()).unwrap();
        assert!(
            info.tables.iter().any(|t| t.name == table),
            "{} schema_info should contain {}",
            label,
            table
        );
    });
}

#[test]
fn all_adapters_invalid_sql_errors() {
    for_each_adapter(|db, _label| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(db.execute("INVALID SQL STATEMENT"));
        assert!(result.is_err());
    });
}
