use const_structures::const_structure;

#[derive(Debug, PartialEq)]
struct Button {
    pin: u8,
    debounce_ms: u32,
}

const_structure! {
    BUTTON: Button { debounce_ms: 20, pin: 13 }
}

#[test]
fn builds_const() {
    assert_eq!(
        BUTTON,
        Button {
            pin: 13,
            debounce_ms: 20
        }
    );
}

#[test]
fn ui() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
