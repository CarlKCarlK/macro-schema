//! Named-field, schema-checked declaration macros.
//!
//! A library declares each macro once, next to the code it generates, with
//! [`define!`]: a schema followed by a `generate { ... }` template. The schema
//! lists each field's name, kind, default, and doc comment. From that one
//! description come parsing, validation, defaulting, error messages, and rustdoc.
//! The template is ordinary Rust tokens plus four constructs:
//!
//! - `$name`, `$vis`, `$doc`, `$attrs`, and each top-level field (`$enabled`)
//!   substitute the declaration's values; a nested field is `$range.min`.
//! - `$for member in $members { ... }` repeats per member; inside, use
//!   `$member.name`, `$member.vis`, `$member.doc`, `$member.attrs`,
//!   `$member.index`, and its fields (`$member.pin`).
//! - `$if let Some(x) = $optional { ... } else { ... }` reads an optional field or block.
//! - `$ident(a, b, ...)` concatenates identifier parts exactly; `$snake(...)` and
//!   `$upper(...)` concatenate and then convert to `snake_case` or
//!   `SCREAMING_SNAKE_CASE`. Parts are identifiers, integers, `ident` fields,
//!   `$name`, or `$member.index`.
//!
//! `$crate` refers to the defining library. Templates are type-checked against the
//! schema when [`define!`] runs, so an unknown value, a missing field, or an optional
//! read without `$if let` is reported at the template. Field names `name`, `vis`,
//! `doc`, `attrs`, `members`, and `index` are reserved for these built-ins.
//!
//! The library must re-export [`expand!`] at its crate root under this exact name:
//!
//! ```text
//! #[doc(hidden)]
//! pub use const_structures::expand as __const_structures_expand;
//! ```
//!
//! ```rust,no_run
//! #![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]
//! #[doc(hidden)]
//! pub use const_structures::expand as __const_structures_expand;
//!
//! const_structures::define! {
//!     /// A configured indicator.
//!     pub indicators {
//!         /// Whether the indicator starts enabled.
//!         enabled: expr = true,
//!     }
//!
//!     generate {
//!         $attrs
//!         #[doc = $doc]
//!         $vis struct $name;
//!         impl $name {
//!             pub const ENABLED: bool = $enabled;
//!         }
//!     }
//! }
//!
//! # fn main() {
//! indicators! { pub Status {} }
//! assert!(Status::ENABLED);
//! # }
//! ```
//!
//! Run the larger example, with members and optional nested fields, with
//! `cargo run --example normalized` from the repository.
//!
//! A library that needs output a template cannot express can instead write
//! `pub NAME => GENERATOR_PATH { BODY }`: [`expand!`] then calls the library's own
//! `macro_rules!` generator (at a path relative to the crate root) with every field
//! present, in schema order, as `attrs: [...], vis: [...], name: ..., doc: "...",
//! field: value, ...`.

#![no_std]

/// Declare a schema-backed macro in the library that owns its output.
/// See the [crate-level example](crate).
pub use const_structures_derive::define;

/// Validate and render an invocation forwarded by a macro declared with
/// [`define`]. See the [crate-level example](crate).
pub use const_structures_derive::expand;
