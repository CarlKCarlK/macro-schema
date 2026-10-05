use demo::button::{button_watch, button_watch_generated::ButtonWatchGenerated};

button_watch! {
    #[derive(Debug, PartialEq)]
    pub(crate) Imported { debounce_ms: 50, pin: PIN_3 }
}

demo::button::button_watch! {
    ByPath { pin: PIN_4, }
}

#[test]
fn defaults_and_overrides() {
    assert_eq!(Imported::PIN, "PIN_3");
    assert_eq!(Imported::DEBOUNCE_MS, 50);
    assert_eq!(Imported, Imported);
    assert_eq!(ByPath::DEBOUNCE_MS, 20);
}

#[test]
fn internal_invocation() {
    assert_eq!(ButtonWatchGenerated::PIN, "PIN_13");
}

#[test]
fn ui() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
