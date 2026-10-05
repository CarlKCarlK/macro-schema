//! `proc_macro2` implementation of `const-structures`.
//!
//! Everything here is ordinary Rust, so it can be unit-tested and stepped
//! through in a debugger without going through the compiler's macro expander.

mod instance;
mod schema;
mod value;

pub use instance::expand;
pub use schema::define;
pub use syn::{Error, Result};

#[cfg(test)]
mod tests;
