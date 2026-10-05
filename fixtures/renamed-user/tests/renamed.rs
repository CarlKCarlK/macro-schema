#[test]
fn renamed_dependency_resolves_generator_and_defaults() {
    assert_eq!(renamed_user::Imported::DEBOUNCE_MS, 20);
    assert_eq!(renamed_user::First::DMA, "DMA0");
}
