mod common;

use ratatui::backend::TestBackend;
use ratatui::Terminal;
use sqrit::db::types::{ObjectKind, ObjectRef};
use sqrit::mode::definition_viewer::{DefinitionContent, DefinitionViewerState};
use sqrit::mode::Mode;

fn render(content: DefinitionContent, scroll: u16) -> Terminal<TestBackend> {
    let mut app = common::test_app();
    app.mode = Mode::DefinitionViewer;
    app.definition_viewer = Some(DefinitionViewerState {
        object: ObjectRef {
            namespace: "public".into(),
            kind: ObjectKind::View,
            name: "active_users".into(),
            relation: None,
            identity_arguments: None,
        },
        content,
        scroll,
    });
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    terminal
}

fn contains(terminal: &Terminal<TestBackend>, needle: &str) -> bool {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height).any(|y| {
        let row = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>();
        row.contains(needle)
    })
}

#[test]
fn renders_loading_ready_and_error_states() {
    assert!(contains(&render(DefinitionContent::Loading, 0), "Loading"));
    assert!(contains(
        &render(
            DefinitionContent::Ready("CREATE VIEW active_users AS SELECT 1;".into()),
            0,
        ),
        "CREATE VIEW"
    ));
    assert!(contains(
        &render(DefinitionContent::Error("permission denied".into()), 0),
        "permission denied"
    ));
}

#[test]
fn title_and_border_are_rendered_at_the_centered_modal_placement() {
    let terminal = render(DefinitionContent::Loading, 0);
    let buffer = terminal.backend().buffer();

    assert!(contains(&terminal, "Definition"));
    assert!(contains(&terminal, "active_users"));
    assert_eq!(buffer[(16, 2)].symbol(), "┌");
    assert_eq!(buffer[(63, 18)].symbol(), "┘");
}

#[test]
fn ready_content_respects_scroll() {
    let content = (0..20)
        .map(|line| format!("line-{line:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    let terminal = render(DefinitionContent::Ready(content), 2);

    assert!(!contains(&terminal, "line-00"));
    assert!(contains(&terminal, "line-02"));
}
