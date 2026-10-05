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
pub use const_structures::expand as __const_structures_expand;

pub mod widgets {
    //! Declares the macro in a module.

    const_structures::define! {
        /// Declares a widget.
        pub widget => __widget_generate {
            /// Widget size.
            size: expr = 1,
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

#[doc(hidden)]
#[macro_export]
macro_rules! __widget_generate {
    (
        attrs: [$(#[$attr:meta])*],
        vis: [$vis:vis],
        name: $name:ident,
        doc: $doc:literal,
        size: $size:expr,
    ) => {
        $(#[$attr])*
        #[doc = $doc]
        $vis struct $name;

        impl $name {
            /// Configured size.
            pub const SIZE: u32 = $size;
        }
    };
}
