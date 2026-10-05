//! Named-field, schema-checked declaration macros.
//!
//! A library defines each declaration macro once, inside its own proc-macro
//! crate, with [`define!`]. The schema lists each field's name, kind, default,
//! and doc comment. From that one description come parsing, validation,
//! defaulting, error messages, and rustdoc. After validation, the generated
//! macro calls the library's code generator (an ordinary `macro_rules!`)
//! with every field present, in schema order:
//!
//! ```text
//! generator! {
//!     attrs: [<attributes>],
//!     vis: [<visibility>],
//!     name: <Name>,
//!     doc: "<generated rustdoc for this instance>",
//!     <field>: <value>, ...
//! }
//! ```
//!
//! See `specs/CONST_STRUCTURES_SPEC.md` for the design.

pub use const_structures_derive::define;

#[doc(hidden)]
pub use const_structures_core as __core;
