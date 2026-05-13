use sqrit::autocomplete::{AutocompleteState, current_word_prefix, suggest};
use sqrit::db::types::{SchemaInfo, TableInfo, ColumnInfo, ViewInfo};

#[test]
fn open_shows_popup_with_candidates_and_selects_first() {
    let mut state = AutocompleteState::new();
    state.open(vec!["SELECT".into(), "INSERT".into(), "UPDATE".into()]);

    assert!(state.is_visible());
    assert_eq!(state.filtered(), vec!["SELECT", "INSERT", "UPDATE"]);
    assert_eq!(state.selected_index(), 0);
}

#[test]
fn dismiss_hides_popup() {
    let mut state = AutocompleteState::new();
    state.open(vec!["SELECT".into()]);
    assert!(state.is_visible());

    state.dismiss();
    assert!(!state.is_visible());
}

#[test]
fn accept_returns_selected_and_dismisses() {
    let mut state = AutocompleteState::new();
    state.open(vec!["SELECT".into(), "INSERT".into()]);

    let accepted = state.accept();
    assert_eq!(accepted, Some("SELECT".to_string()));
    assert!(!state.is_visible());
}

#[test]
fn accept_returns_none_when_not_visible() {
    let mut state = AutocompleteState::new();
    assert!(!state.is_visible());
    assert_eq!(state.accept(), None);
}

#[test]
fn open_with_empty_candidates_stays_hidden() {
    let mut state = AutocompleteState::new();
    state.open(vec![]);
    assert!(!state.is_visible());
}

#[test]
fn next_cycles_through_suggestions() {
    let mut state = AutocompleteState::new();
    state.open(vec!["SELECT".into(), "INSERT".into(), "UPDATE".into()]);

    assert_eq!(state.selected_index(), 0);
    state.next();
    assert_eq!(state.selected_index(), 1);
    state.next();
    assert_eq!(state.selected_index(), 2);
    state.next(); // wraps to 0
    assert_eq!(state.selected_index(), 0);
}

#[test]
fn prev_cycles_backward() {
    let mut state = AutocompleteState::new();
    state.open(vec!["SELECT".into(), "INSERT".into(), "UPDATE".into()]);

    assert_eq!(state.selected_index(), 0);
    state.prev(); // wraps to last
    assert_eq!(state.selected_index(), 2);
    state.prev();
    assert_eq!(state.selected_index(), 1);
}

#[test]
fn filter_narrows_candidates_case_insensitive() {
    let mut state = AutocompleteState::new();
    state.open(vec!["SELECT".into(), "INSERT".into(), "UPDATE".into(), "DELETE".into()]);

    state.filter("se");
    assert_eq!(state.filtered(), vec!["SELECT"]);
    assert_eq!(state.selected_index(), 0);
}

#[test]
fn filter_resets_selection() {
    let mut state = AutocompleteState::new();
    state.open(vec!["SELECT".into(), "SET".into(), "SHOW".into()]);
    state.next(); // selected = 1 ("SET")
    assert_eq!(state.selected_index(), 1);

    state.filter("s");
    assert_eq!(state.filtered(), vec!["SELECT", "SET", "SHOW"]);
    assert_eq!(state.selected_index(), 0); // reset to first
}

#[test]
fn filter_with_no_match_yields_empty() {
    let mut state = AutocompleteState::new();
    state.open(vec!["SELECT".into()]);
    state.filter("xyz");
    assert!(state.filtered().is_empty());
}

// --- T19: Autocomplete engine ---

#[test]
fn current_word_prefix_extracts_partial_word_before_cursor() {
    let text = "SEL";
    assert_eq!(current_word_prefix(text, 0, 3), "SEL");
}

#[test]
fn current_word_prefix_empty_at_line_start() {
    let text = "SELECT * FROM";
    assert_eq!(current_word_prefix(text, 0, 0), "");
}

#[test]
fn current_word_prefix_extracts_mid_word() {
    let text = "SELECT * FR";
    assert_eq!(current_word_prefix(text, 0, 11), "FR");
}

#[test]
fn current_word_prefix_after_space_is_empty() {
    let text = "SELECT ";
    assert_eq!(current_word_prefix(text, 0, 7), "");
}

#[test]
fn current_word_prefix_with_underscore() {
    let text = "user_ta";
    assert_eq!(current_word_prefix(text, 0, 7), "user_ta");
}

#[test]
fn suggest_returns_keywords_matching_prefix() {
    let results = suggest("SEL", None);
    assert!(results.contains(&"SELECT".to_string()));
}

#[test]
fn suggest_keywords_case_insensitive() {
    let results = suggest("sel", None);
    assert!(results.contains(&"SELECT".to_string()));
}

#[test]
fn suggest_no_match_returns_empty() {
    let results = suggest("ZZZ", None);
    assert!(results.is_empty());
}

#[test]
fn suggest_empty_prefix_returns_nothing() {
    let results = suggest("", None);
    assert!(results.is_empty());
}

#[test]
fn suggest_includes_table_names_from_schema() {
    let schema = SchemaInfo {
        tables: vec![
            TableInfo { name: "users".into(), columns: vec![] },
            TableInfo { name: "orders".into(), columns: vec![] },
        ],
        views: vec![],
    };
    let results = suggest("us", Some(&schema));
    assert!(results.contains(&"users".to_string()));
    assert!(!results.contains(&"orders".to_string()));
}

#[test]
fn suggest_includes_view_names_from_schema() {
    let schema = SchemaInfo {
        tables: vec![],
        views: vec![ViewInfo { name: "active_users".into(), columns: vec![] }],
    };
    let results = suggest("act", Some(&schema));
    assert!(results.contains(&"active_users".to_string()));
}

#[test]
fn suggest_includes_column_names_from_all_tables() {
    let schema = SchemaInfo {
        tables: vec![
            TableInfo {
                name: "users".into(),
                columns: vec![
                    ColumnInfo { name: "id".into(), data_type: "INTEGER".into(), nullable: false, is_primary_key: true },
                    ColumnInfo { name: "email".into(), data_type: "TEXT".into(), nullable: false, is_primary_key: false },
                ],
            },
            TableInfo {
                name: "orders".into(),
                columns: vec![
                    ColumnInfo { name: "order_id".into(), data_type: "INTEGER".into(), nullable: false, is_primary_key: true },
                ],
            },
        ],
        views: vec![],
    };
    let results = suggest("em", Some(&schema));
    assert!(results.contains(&"email".to_string()));
}

#[test]
fn suggest_deduplicates_column_and_keyword() {
    // "IN" is both a keyword and a column prefix — keyword appears once
    let schema = SchemaInfo {
        tables: vec![TableInfo {
            name: "t".into(),
            columns: vec![ColumnInfo { name: "insert_time".into(), data_type: "TEXT".into(), nullable: false, is_primary_key: false }],
        }],
        views: vec![],
    };
    let results = suggest("in", Some(&schema));
    let count = results.iter().filter(|r| *r == "insert_time").count();
    assert_eq!(count, 1);
}
