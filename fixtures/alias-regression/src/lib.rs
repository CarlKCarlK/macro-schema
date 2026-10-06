//! Regression fixture for the alias that `define!` relies on.
//!
//! `define!` emits a hidden `#[macro_export]` wrapper and `pub use wrapper as name;`.
//! Reaching a macro-expanded `#[macro_export]` macro through `crate::...` is rejected
//! (`macro_expanded_macro_exports_accessed_by_absolute_paths`); the bare-name alias
//! is not. This crate forbids that lint and denies warnings, and exercises every way
//! a library reaches its own declared macros. `tests/ui/alias_via_crate_path.rs` is
//! the control: the `crate::` form must still fail. Downstream use under a renamed
//! dependency is in `fixtures/renamed-user`.
#![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]
#![deny(warnings)]

#[doc(hidden)]
pub use macro_schema::expand as __macro_schema_expand;

/// Default widget size, used from both the schema and its generated backend.
pub const DEFAULT_WIDGET_SIZE: u32 = 1;

pub mod widgets {
    //! Declares the macro in a module.

    macro_schema::define! {
        /// Declares a widget.
        pub widget {
            /// Widget size.
            #[default_display = "1"]
            size: expr = $crate::DEFAULT_WIDGET_SIZE,
        }

        generate {
            $decl.attrs
            #[doc = $decl.doc]
            $decl.vis struct $decl.name;

            impl $decl.name {
                /// Configured size.
                pub const SIZE: u32 = $decl.size;
                /// Default from the defining library, even when renamed downstream.
                pub const DEFAULT_SIZE: u32 = $crate::DEFAULT_WIDGET_SIZE;
            }
        }
    }

    // A disabled definition with the same names detects missing cfg propagation
    // on either generated macro or either bare-name alias.
    macro_schema::define! {
        #[cfg(any())]
        pub widget { size: expr }
        generate { compile_error!("disabled definition must be absent"); }
    }

    // Distinct public names must not collide between internal wrapper names.
    macro_schema::define! {
        pub item { size: expr }
        generate {
            $decl.vis struct $decl.name;
            impl $decl.name { pub const SIZE: u32 = $decl.size; }
        }
    }
    macro_schema::define! {
        pub item_generate_impl { size: expr }
        generate {
            $decl.vis struct $decl.name;
            impl $decl.name { pub const SIZE: u32 = $decl.size; }
        }
    }

    /// Same-module call.
    pub mod same_module {
        super::widget! { pub SameModule {} }
    }
}

/// Module re-export of the alias.
pub mod reexported {
    pub use crate::widgets::widget;
}

// Crate-root re-export of the alias.
pub use widgets::widget;

/// Same-crate calls through each path.
pub mod calls {
    crate::widgets::widget! { pub ByModulePath { size: 2 } }
    crate::reexported::widget! { pub ByModuleReexport { size: 3 } }
    crate::widget! { pub ByCrateRoot { size: 4 } }
}

macro_rules! from_another_macro {
    ($name:ident) => {
        crate::widget! { pub $name { size: 5 } }
    };
}

/// Call made from inside another macro's expansion.
pub mod via_macro {
    from_another_macro!(FromAnotherMacro);
}
