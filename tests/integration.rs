use sqrit::db::mysql::MySqlAdapter;
use sqrit::db::postgres::PgAdapter;
use sqrit::db::sqlite::SqliteAdapter;
use sqrit::db::types::Value;
use sqrit::db::Database;

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

fn port_reachable(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    addr.to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
        .is_some_and(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok())
}

fn unique_table(label: &str) -> String {
    format!("integ_{}_{}", label, std::process::id())
}

async fn sqlite_db() -> Box<dyn Database> {
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();
    std::mem::forget(file);
    let mut adapter = SqliteAdapter::new(&path);
    adapter.connect().await.unwrap();
    Box::new(adapter)
}

async fn pg_db() -> Option<Box<dyn Database>> {
    if !port_reachable("localhost", 15432) {
        return None;
    }
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sqrit:sqrit@localhost:15432/sqrit_test".to_string());
    let mut adapter = PgAdapter::new(&url);
    adapter
        .connect()
        .await
        .ok()
        .map(|_| Box::new(adapter) as Box<dyn Database>)
}

async fn mysql_db() -> Option<Box<dyn Database>> {
    if !port_reachable("localhost", 13306) {
        return None;
    }
    let url = std::env::var("MYSQL_URL")
        .unwrap_or_else(|_| "mysql://sqrit:sqrit@localhost:13306/sqrit_test".to_string());
    let mut adapter = MySqlAdapter::new(&url);
    adapter
        .connect()
        .await
        .ok()
        .map(|_| Box::new(adapter) as Box<dyn Database>)
}

enum Backend {
    Sqlite,
    Postgres,
    Mysql,
}

fn quote_ident(label: &Backend, name: &str) -> String {
    match label {
        Backend::Mysql => format!("`{}`", name),
        _ => format!("\"{}\"", name),
    }
}

fn id_col(label: &Backend) -> &'static str {
    match label {
        Backend::Sqlite => "id INTEGER PRIMARY KEY",
        Backend::Postgres => "id SERIAL PRIMARY KEY",
        Backend::Mysql => "id INT AUTO_INCREMENT PRIMARY KEY",
    }
}

fn text_col(label: &Backend, name: &str, nullable: bool) -> String {
    let not_null = if nullable { "" } else { " NOT NULL" };
    match label {
        Backend::Mysql => format!("{} VARCHAR(255){}", name, not_null),
        _ => format!("{} TEXT{}", name, not_null),
    }
}

/// Run an async test closure against all available backends.
async fn for_each_adapter<F, Fut>(f: F)
where
    F: Fn(Box<dyn Database>, Backend) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    f(sqlite_db().await, Backend::Sqlite).await;
    if let Some(db) = pg_db().await {
        f(db, Backend::Postgres).await;
    }
    if let Some(db) = mysql_db().await {
        f(db, Backend::Mysql).await;
    }
}

#[tokio::test]
async fn all_adapters_execute_select_literal() {
    for_each_adapter(|db, _| async move {
        let result = db.execute("SELECT 1 AS val").await.unwrap();
        assert_eq!(result.columns, vec!["val"]);
        assert_eq!(result.rows.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn all_adapters_create_insert_select_drop() {
    for_each_adapter(|db, backend| async move {
        let table = unique_table("cis");
        let t = quote_ident(&backend, &table);

        db.execute(&format!(
            "CREATE TABLE {} ({} , {})",
            t,
            id_col(&backend),
            text_col(&backend, "name", false)
        ))
        .await
        .unwrap();

        db.execute(&format!("INSERT INTO {} (name) VALUES ('alice')", t))
            .await
            .unwrap();
        let ins = db
            .execute(&format!("INSERT INTO {} (name) VALUES ('bob')", t))
            .await
            .unwrap();
        assert_eq!(ins.rows_affected, Some(1));

        let sel = db
            .execute(&format!("SELECT id, name FROM {} ORDER BY id", t))
            .await
            .unwrap();
        assert_eq!(sel.rows.len(), 2);
        assert_eq!(sel.columns, vec!["id", "name"]);
        assert_eq!(
            sel.rows[0].get("name").unwrap(),
            &Value::Text("alice".into())
        );

        db.execute(&format!("DROP TABLE {}", t)).await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn all_adapters_paginate() {
    for_each_adapter(|db, backend| async move {
        let table = unique_table("page");
        let t = quote_ident(&backend, &table);

        db.execute(&format!(
            "CREATE TABLE {} ({} , {})",
            t,
            id_col(&backend),
            text_col(&backend, "val", true)
        ))
        .await
        .unwrap();

        for i in 0..5 {
            db.execute(&format!("INSERT INTO {} (val) VALUES ('v{}')", t, i))
                .await
                .unwrap();
        }

        let base_query = format!("SELECT * FROM {} ORDER BY id", t);

        let page = db.execute_paginated(&base_query, 2, 2).await.unwrap();
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.rows[0].get("val").unwrap(), &Value::Text("v2".into()));

        // Exercise trailing semicolon/whitespace trimming
        let trailing = format!("{base_query}; \n");
        let page2 = db.execute_paginated(&trailing, 2, 2).await.unwrap();
        assert_eq!(page2.rows.len(), 2);
        assert_eq!(page2.rows[0].get("val").unwrap(), &Value::Text("v2".into()));
    })
    .await;
}

#[tokio::test]
async fn all_adapters_list_tables_and_columns() {
    for_each_adapter(|db, backend| async move {
        let table = unique_table("tbl");
        let t = quote_ident(&backend, &table);

        db.execute(&format!(
            "CREATE TABLE {} ({} , {})",
            t,
            id_col(&backend),
            text_col(&backend, "name", false)
        ))
        .await
        .unwrap();

        let tables = db.list_tables().await.unwrap();
        assert!(tables.contains(&table));

        let columns = db.list_columns(&table).await.unwrap();
        assert!(columns.len() >= 2);

        let id_col = columns.iter().find(|c| c.name == "id").unwrap();
        assert!(id_col.is_primary_key);

        let name_col = columns.iter().find(|c| c.name == "name").unwrap();
        assert!(!name_col.nullable);
    })
    .await;
}

#[tokio::test]
async fn all_adapters_schema_info() {
    for_each_adapter(|db, backend| async move {
        let table = unique_table("sch");
        let t = quote_ident(&backend, &table);

        db.execute(&format!(
            "CREATE TABLE {} ({} , {})",
            t,
            id_col(&backend),
            text_col(&backend, "v", true)
        ))
        .await
        .unwrap();

        let info = db.schema_info().await.unwrap();
        assert!(info.tables.iter().any(|t| t.name == table));
    })
    .await;
}

#[tokio::test]
async fn all_adapters_invalid_sql_errors() {
    for_each_adapter(|db, _| async move {
        let result = db.execute("INVALID SQL STATEMENT").await;
        assert!(result.is_err());
    })
    .await;
}
