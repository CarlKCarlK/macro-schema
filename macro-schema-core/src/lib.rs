//! `proc_macro2` implementation of `macro-schema`.
//!
//! Depend on the `macro-schema` crate, not this one. This crate holds the
//! implementation behind its `define!` and `expand!` macros as ordinary Rust
//! functions, [`define`] and [`expand`], so it can be unit-tested and stepped
//! through in a debugger without going through the compiler's macro expander.
//! The repository's `specs/MACRO_SCHEMA_SPEC.md` explains how the pieces fit
//! together.

mod instance;
mod schema;
mod template;
mod value;

pub use instance::expand;
pub use schema::define;
pub use syn::{Error, Result};

#[cfg(test)]
mod tests;
