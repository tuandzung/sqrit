use sqrit::autocomplete::AutocompleteState;

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
