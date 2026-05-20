mod common;

#[test]
fn app_constructs_with_a_theme() {
    let app = common::test_app();
    // test_app() uses the hardcoded default fallback; name is "default"
    assert_eq!(app.theme.name, "default");
}
