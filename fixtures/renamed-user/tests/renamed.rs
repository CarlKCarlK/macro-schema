#![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]
#![deny(warnings)]

#[test]
fn renamed_dependency_resolves_generator_and_defaults() {
    assert_eq!(renamed_user::Imported::DEBOUNCE_MS, 20);
    assert_eq!(renamed_user::First::DMA, "DMA0");
}

renamed_alias::widget! { pub RenamedRoot { size: 8 } }
renamed_alias::reexported::widget! { pub RenamedModule {} }

#[test]
fn renamed_dependency_reaches_aliases() {
    assert_eq!(RenamedRoot::SIZE, 8);
    assert_eq!(RenamedModule::SIZE, 1);
}
