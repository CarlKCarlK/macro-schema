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
//! The macros are general purpose: a library supplies its own schema and
//! generator. Add a `generate { ... }` block after the schema to have [`define!`]
//! create the backend matcher and export; write only its output template.
//! The backend must be reachable downstream through the declared generator path;
//! re-export it through a public path if its schema module is private.
//! The repository README includes a standalone walkthrough and
//! points to a small normalized backend example.
//!
//! Run that complete example with `cargo run --example normalized` from the
//! repository. It demonstrates a required field, a default, attributes,
//! visibility, members, and optional nested fields.
//!
//! Templates receive `$name`, `$vis`, `$doc`, and repeated `$attrs` metadata.
//! A field `enabled` binds `$field_enabled`; a nested field `range.min` binds
//! `$field_range_min`. Members bind `$member_name`, `$member_vis`, `$member_doc`,
//! repeated `$member_attrs`, and `$member_index`; their fields use the
//! `$member_field_` prefix. `$member_count` is available for member schemas.
//! Use `$( ... )*` for members and `$( ... )?` for optional fields/blocks,
//! following the schema's nesting. Blocks bind their leaf fields. An optional
//! block with no leaf fields has no presence binding; use an explicit backend
//! when its presence matters. Flattened
//! binding-name collisions are rejected at the schema field.
//!
//! The generator path names the alias from the defining crate's root. For a
//! schema in an `indicators` module, write `indicators::__indicators_generate`.
//! Omitting `generate` keeps an explicitly authored backend, useful for
//! generators with multiple matcher arms.
//!
//! ```rust,no_run
//! #![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]
//! #[doc(hidden)]
//! pub use const_structures::expand as __const_structures_expand;
//!
//! const_structures::define! {
//!     /// A configured indicator.
//!     pub indicators => __indicators_generate {
//!         /// Whether the indicator starts enabled.
//!         enabled: expr = true,
//!     }
//!
//!     generate {
//!         $(#[$attrs])*
//!         #[doc = $doc]
//!         $vis struct $name;
//!         impl $name {
//!             pub const ENABLED: bool = $field_enabled;
//!         }
//!     }
//! }
//!
//! # fn main() {
//! indicators! { pub Status {} }
//! assert!(Status::ENABLED);
//! # }
//! ```

#![no_std]

/// Declare a schema-backed macro in the library that owns its generator.
/// See the [crate-level example](crate) and the repository's `normalized`
/// example for the complete pattern.
pub use const_structures_derive::define;

/// Expand and normalize an invocation forwarded by a macro declared with
/// [`define`]. See the [crate-level example](crate) and the repository's
/// `normalized` example for the complete pattern.
pub use const_structures_derive::expand;
