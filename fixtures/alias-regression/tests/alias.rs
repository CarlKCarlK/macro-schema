use alias_regression::{calls, via_macro, widgets::same_module};

#[test]
fn every_same_crate_path_expands() {
    assert_eq!(same_module::SameModule::SIZE, 1);
    assert_eq!(calls::ByModulePath::SIZE, 2);
    assert_eq!(calls::ByModuleReexport::SIZE, 3);
    assert_eq!(calls::ByCrateRoot::SIZE, 4);
    assert_eq!(via_macro::FromAnotherMacro::SIZE, 5);
}

alias_regression::widget! { pub DownstreamRoot { size: 6 } }
alias_regression::reexported::widget! { pub DownstreamModule { size: 7 } }

#[test]
fn downstream_paths_expand() {
    assert_eq!(DownstreamRoot::SIZE, 6);
    assert_eq!(DownstreamModule::SIZE, 7);
}

#[test]
fn ui() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
