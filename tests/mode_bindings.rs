use sqrit::mode::Mode;

const ALL_MODES: &[Mode] = &[
    Mode::Picker,
    Mode::Explorer,
    Mode::QueryNormal,
    Mode::QueryInsert,
    Mode::Results,
    Mode::ThemePicker,
    Mode::Help,
    Mode::CellViewer,
];

#[test]
fn every_mode_publishes_at_least_one_binding() {
    for mode in ALL_MODES {
        let bindings = mode.handler().bindings();
        assert!(
            !bindings.is_empty(),
            "{:?} must publish at least one KeyBinding for the help overlay",
            mode
        );
    }
}

#[test]
fn every_binding_has_non_empty_key_and_action() {
    for mode in ALL_MODES {
        for b in mode.handler().bindings() {
            assert!(!b.key.is_empty(), "{:?} has empty key in binding", mode);
            assert!(
                !b.action.is_empty(),
                "{:?} has empty action for key {}",
                mode,
                b.key
            );
        }
    }
}
