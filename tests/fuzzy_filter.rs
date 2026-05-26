use sqrit::db::types::{QueryResult, ResultColumn, Row, Value};
use sqrit::filter::FuzzyFilter;

fn make_result(cols: &[&str], rows: Vec<Vec<&str>>) -> QueryResult {
    let columns: Vec<ResultColumn> = cols
        .iter()
        .map(|name| ResultColumn::untyped(*name))
        .collect();
    let row_maps: Vec<Row> = rows
        .into_iter()
        .map(|values| {
            cols.iter()
                .zip(values)
                .map(|(c, v)| (c.to_string(), Value::Text(v.to_string())))
                .collect()
        })
        .collect();
    QueryResult {
        columns,
        rows: row_maps,
        rows_affected: None,
        total_count: None,
    }
}

#[test]
fn empty_query_passes_every_row_in_original_order() {
    let result = make_result(
        &["id", "email"],
        vec![
            vec!["1", "alice@example.com"],
            vec!["2", "bob@example.com"],
            vec!["3", "carol@example.com"],
        ],
    );
    let mut filter = FuzzyFilter::new();
    let hits = filter.rank(&result, "");
    assert_eq!(hits.len(), 3);
    let order: Vec<usize> = hits.iter().map(|h| h.row_index).collect();
    assert_eq!(order, vec![0, 1, 2]);
    for hit in &hits {
        assert_eq!(hit.score, 0);
        assert!(hit.matches.is_empty());
    }
}

#[test]
fn subsequence_match_scores_nonzero_and_filters_misses() {
    let result = make_result(
        &["email"],
        vec![
            vec!["user_at_bc.io"],
            vec!["nothing_related"],
            vec!["username@abc.org"],
        ],
    );
    let mut filter = FuzzyFilter::new();
    let hits = filter.rank(&result, "usrbc");
    let matched: Vec<usize> = hits.iter().map(|h| h.row_index).collect();
    assert!(matched.contains(&0), "row 0 should match");
    assert!(matched.contains(&2), "row 2 should match");
    assert!(!matched.contains(&1), "row 1 must not appear (no subseq)");
    for hit in &hits {
        assert!(hit.score > 0);
        assert!(!hit.matches.is_empty());
    }
}

#[test]
fn match_spans_carry_column_index() {
    let result = make_result(
        &["id", "email", "note"],
        vec![vec!["1", "alice@example.com", "VIP customer"]],
    );
    let mut filter = FuzzyFilter::new();
    let hits = filter.rank(&result, "alice");
    assert_eq!(hits.len(), 1);
    let hit = &hits[0];
    let cols_matched: std::collections::HashSet<usize> =
        hit.matches.iter().map(|(c, _)| *c).collect();
    assert!(
        cols_matched.contains(&1),
        "'email' column should carry matches"
    );
}

#[test]
fn exact_substring_outranks_scattered_subsequence() {
    let result = make_result(
        &["email"],
        vec![
            vec!["scattered_letters_abcorp"],
            vec!["a-b-c-o-r-p-elsewhere"],
        ],
    );
    let mut filter = FuzzyFilter::new();
    let hits = filter.rank(&result, "abcorp");
    assert!(hits.len() >= 2);
    assert_eq!(hits[0].row_index, 0);
}

#[test]
fn unicode_haystack_does_not_panic_and_ranges_are_char_based() {
    let result = make_result(&["note"], vec![vec!["中文测试 alice"], vec!["only ascii"]]);
    let mut filter = FuzzyFilter::new();
    let hits = filter.rank(&result, "alice");
    assert!(hits.iter().any(|h| h.row_index == 0));
    if let Some(hit) = hits.iter().find(|h| h.row_index == 0) {
        let text = "中文测试 alice";
        let chars: Vec<char> = text.chars().collect();
        for (_col, range) in &hit.matches {
            assert!(range.end <= chars.len(), "range out of char bounds");
            assert!(range.start <= range.end);
        }
    }
}
