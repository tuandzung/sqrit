use std::collections::HashMap;

use sqrit::db::types::{ResultColumn, Value};
use sqrit::results::row_matches;

fn row(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn cols(names: &[&str]) -> Vec<ResultColumn> {
    names.iter().map(|n| ResultColumn::untyped(*n)).collect()
}

#[test]
fn empty_filter_matches_anything() {
    let r = row(&[("name", Value::Text("alice".into()))]);
    assert!(row_matches(&r, &cols(&["name"]), ""));
}

#[test]
fn substring_match_is_case_insensitive() {
    let r = row(&[("name", Value::Text("Alice".into()))]);
    let c = cols(&["name"]);
    assert!(row_matches(&r, &c, "ali"));
    assert!(row_matches(&r, &c, "ALI"));
    assert!(row_matches(&r, &c, "LIC"));
}

#[test]
fn match_spans_all_columns() {
    let r = row(&[
        ("name", Value::Text("bob".into())),
        ("city", Value::Text("Berlin".into())),
    ]);
    let c = cols(&["name", "city"]);

    assert!(row_matches(&r, &c, "ber"));
    assert!(row_matches(&r, &c, "bob"));
    assert!(!row_matches(&r, &c, "paris"));
}

#[test]
fn matches_against_non_text_value_renderings() {
    let r = row(&[("id", Value::Integer(42)), ("active", Value::Boolean(true))]);
    let c = cols(&["id", "active"]);

    assert!(row_matches(&r, &c, "42"));
    assert!(row_matches(&r, &c, "true"));
}

#[test]
fn null_renders_to_null_for_matching() {
    let r = row(&[("note", Value::Null)]);
    let c = cols(&["note"]);
    assert!(row_matches(&r, &c, "null"));
}
