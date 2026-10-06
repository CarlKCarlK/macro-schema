//! Procedural macro entry points for `macro-schema`.
//!
//! Depend on the `macro-schema` crate, not this one: it re-exports these macros
//! as `macro_schema::define!` and `macro_schema::expand!`, with their
//! documentation. Each macro here converts its input and errors and forwards to
//! `macro-schema-core`.

// The macros below carry no doc comments on purpose: rustdoc appends an item's own
// docs to every re-export of it, so they would trail the facade's documentation.

use proc_macro::TokenStream;

#[proc_macro]
pub fn define(input: TokenStream) -> TokenStream {
    macro_schema_core::define(input.into())
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}

#[proc_macro]
pub fn expand(input: TokenStream) -> TokenStream {
    macro_schema_core::expand(input.into())
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}
