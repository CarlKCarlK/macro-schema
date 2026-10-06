//! Keeps README.md in step with the code it quotes.
//!
//! The README's code blocks also run as doctests (the crate documentation includes
//! the README), so its quick start compiles and runs. These tests check the parts a
//! doctest can't: that the quick start is the same file as the runnable example, and
//! that the quoted compiler errors are the ones the trybuild tests produce.

use std::{fs, path::Path};

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

/// The contents of the README's ```rust code block whose first line is `first_line`.
fn rust_block_starting_with(readme: &str, first_line: &str) -> String {
    let mut blocks = readme.split("```rust\n").skip(1);
    let block = blocks
        .find(|block| block.starts_with(first_line))
        .unwrap_or_else(|| panic!("README has no ```rust block starting with {first_line:?}"));
    let end = block.find("```").expect("unterminated code block");
    block[..end].to_owned()
}

#[test]
fn readme_quick_start_is_the_quick_start_example() {
    let readme = read("README.md");
    let block = rust_block_starting_with(&readme, "// examples/quick_start.rs");
    assert_eq!(block, read("examples/quick_start.rs"));
}

#[test]
fn readme_errors_match_the_ui_tests() {
    let readme = read("README.md");
    let readme_lines: Vec<&str> = readme.lines().collect();
    for stderr in [
        "fixtures/demo/tests/ui/readme_invocation_error.stderr",
        "fixtures/demo/tests/ui/readme_template_error.stderr",
    ] {
        // Every line but the file location (`--> tests/ui/...`) is quoted verbatim.
        for line in read(stderr).lines().filter(|line| !line.contains("-->")) {
            assert!(
                readme_lines.contains(&line),
                "README.md is missing this line of {stderr}:\n{line}"
            );
        }
    }
}
