#![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]
#![deny(warnings)]

use std::error::Error;

use alias_regression::{calls, via_macro, widget, widgets::same_module};

pub use alias_regression::widgets::widget as exported_widget;

widget! { #[derive(Debug, PartialEq)] pub DownstreamImported {} }
exported_widget! { pub FurtherExport { size: 9 } }
alias_regression::widgets::item! { pub FirstItem { size: 10 } }
alias_regression::widgets::item_generate_impl! { pub SecondItem { size: 11 } }

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
    assert_eq!(DownstreamImported::SIZE, 1);
    assert_eq!(DownstreamImported::DEFAULT_SIZE, 1);
    assert_eq!(DownstreamImported, DownstreamImported);
    assert_eq!(FurtherExport::SIZE, 9);
    assert_eq!(FirstItem::SIZE, 10);
    assert_eq!(SecondItem::SIZE, 11);
}

#[test]
fn ui() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}

#[test]
fn rustdoc_keeps_public_alias_in_its_module() -> Result<(), Box<dyn Error>> {
    use std::{fs, path::Path, process::Command};

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = manifest.join("../../target/alias-rustdoc");
    let status = Command::new(env!("CARGO"))
        .args(["doc", "--offline", "--no-deps", "-p", "alias-regression"])
        .arg("--manifest-path")
        .arg(manifest.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .env("RUSTDOCFLAGS", "-D warnings")
        .status()?;
    assert!(status.success(), "alias fixture rustdoc failed");
    let docs = target.join("doc/alias_regression");
    let macro_page = fs::read_to_string(docs.join("widgets/macro.widget.html"))?;
    assert!(macro_page.contains("Declares a widget."));
    assert!(macro_page.contains("Fields:"));
    assert!(macro_page.contains("Widget size."));
    assert!(
        !docs
            .join("macro.__const_structures_wrapper_widget.html")
            .exists()
    );
    assert!(
        !docs
            .join("macro.__const_structures_backend_widget.html")
            .exists()
    );
    assert!(!docs.join("widgets/macro.__widget_generate.html").exists());
    Ok(())
}
