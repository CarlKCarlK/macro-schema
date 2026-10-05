//! Named-field, schema-checked declaration macros.
//!
//! A library declares each macro once, next to the code it generates, with
//! [`define!`]. The schema lists each field's name, kind, default, and doc
//! comment. From that one description come parsing, validation, defaulting,
//! error messages, and rustdoc.
//!
//! The library must re-export [`expand!`] at its crate root under this exact name:
//!
//! ```text
//! #[doc(hidden)]
//! pub use const_structures::expand as __const_structures_expand;
//! ```
//!
//! Each declared macro forwards the user's tokens, its schema, and its generator
//! path to `expand!`, which validates them and calls the library's code generator
//! (an ordinary `macro_rules!`, at a path relative to the crate root) with every
//! field present, in schema order:
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

#![no_std]

pub use const_structures_derive::{define, expand};
