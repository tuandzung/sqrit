use sqrit::mode::Mode;

#[test]
fn every_mode_has_at_least_one_binding() {
    for mode in [
        Mode::Picker,
        Mode::Explorer,
        Mode::QueryNormal,
        Mode::QueryInsert,
        Mode::Results,
        Mode::ThemePicker,
        Mode::Help,
        Mode::CellViewer,
        Mode::HistoryPicker,
        Mode::ResultsFilter,
    ] {
        let bindings = mode.handler().bindings();
        assert!(
            !bindings.is_empty(),
            "Mode {:?} returned an empty bindings() slice — hint bar would have nothing to show",
            mode,
        );
    }
}
