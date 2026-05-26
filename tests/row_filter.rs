use std::collections::HashMap;

use sqrit::db::types::QueryResult;
use sqrit::db::types::{ResultColumn, Value};
use sqrit::filter::FuzzyFilter;

fn row(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn cols(names: &[&str]) -> Vec<ResultColumn> {
    names.iter().map(|n| ResultColumn::untyped(*n)).collect()
}

fn rank_one(row: HashMap<String, Value>, columns: Vec<ResultColumn>, query: &str) -> bool {
    let result = QueryResult {
        columns,
        rows: vec![row],
        rows_affected: None,
        total_count: None,
    };
    !FuzzyFilter::new().rank(&result, query).is_empty()
}

#[test]
fn empty_filter_matches_anything() {
    let r = row(&[("name", Value::Text("alice".into()))]);
    assert!(rank_one(r, cols(&["name"]), ""));
}

#[test]
fn subsequence_match_is_case_insensitive() {
    let r = row(&[("name", Value::Text("Alice".into()))]);
    let c = cols(&["name"]);
    assert!(rank_one(r.clone(), c.clone(), "ali"));
    assert!(rank_one(r.clone(), c.clone(), "ALI"));
    assert!(rank_one(r, c, "aie"));
}

#[test]
fn match_spans_all_columns() {
    let r = row(&[
        ("name", Value::Text("bob".into())),
        ("city", Value::Text("Berlin".into())),
    ]);
    let c = cols(&["name", "city"]);

    assert!(rank_one(r.clone(), c.clone(), "brl"));
    assert!(rank_one(r.clone(), c.clone(), "bob"));
    assert!(!rank_one(r, c, "paris"));
}

#[test]
fn matches_against_non_text_value_renderings() {
    let r = row(&[("id", Value::Integer(42)), ("active", Value::Boolean(true))]);
    let c = cols(&["id", "active"]);

    assert!(rank_one(r.clone(), c.clone(), "42"));
    assert!(rank_one(r, c, "true"));
}

#[test]
fn null_renders_to_null_for_matching() {
    let r = row(&[("note", Value::Null)]);
    let c = cols(&["note"]);
    assert!(rank_one(r, c, "null"));
}
