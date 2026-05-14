mod common;

use sqrit::app::{App, QueryStatus};
use sqrit::mode::Mode;

fn make_app() -> App {
    common::test_app()
}

#[test]
fn status_bar_shows_mode_label() {
    let mut app = make_app();

    app.mode = Mode::QueryNormal;
    assert!(app.status_bar_text().contains("NORMAL"));

    app.mode = Mode::QueryInsert;
    assert!(app.status_bar_text().contains("INSERT"));

    app.mode = Mode::Explorer;
    assert!(app.status_bar_text().contains("EXPLORER"));

    app.mode = Mode::Results;
    assert!(app.status_bar_text().contains("RESULTS"));
}

#[test]
fn status_bar_shows_active_connection_name() {
    let mut app = make_app();
    assert!(app.status_bar_text().contains("no connection"));

    app.active_connection = Some("mydb".to_string());
    assert!(app.status_bar_text().contains("mydb"));
    assert!(!app.status_bar_text().contains("no connection"));
}

#[test]
fn status_bar_shows_query_status_idle_when_idle() {
    let app = make_app();
    let text = app.status_bar_text();
    // Idle: empty status section after final pipe
    assert!(text.ends_with("| ") || text.ends_with("|"));
    assert!(!text.contains("running"));
    assert!(!text.contains("ERR"));
}

#[test]
fn status_bar_shows_running_when_query_running() {
    let mut app = make_app();
    app.query_status = QueryStatus::Running;
    assert!(app.status_bar_text().contains("running..."));
}

#[test]
fn status_bar_shows_ok_on_success() {
    let mut app = make_app();
    app.query_status = QueryStatus::Success;
    assert!(app.status_bar_text().contains("ok"));
}

#[test]
fn status_bar_shows_error_message() {
    let mut app = make_app();
    app.query_status = QueryStatus::Error("table not found".to_string());
    let text = app.status_bar_text();
    assert!(text.contains("ERR:"));
    assert!(text.contains("table not found"));
}

#[test]
fn status_bar_shows_status_message_when_idle() {
    let mut app = make_app();
    app.status_message = "3 rows".to_string();
    assert!(app.status_bar_text().contains("3 rows"));
}

#[test]
fn status_bar_combines_query_status_and_message() {
    let mut app = make_app();
    app.query_status = QueryStatus::Success;
    app.status_message = "3 rows".to_string();
    let text = app.status_bar_text();
    assert!(text.contains("ok"));
    assert!(text.contains("3 rows"));
}
