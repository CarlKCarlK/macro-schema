//! Named-field, schema-checked declaration macros.
//!
//! A library declares each macro once, next to the code it generates, with
//! [`define!`]: a schema followed by a `generate { ... }` template. The schema
//! lists each field's name, kind, default, and doc comment. From that one
//! description come parsing, validation, defaulting, error messages, and rustdoc.
//! The template is ordinary Rust tokens plus four constructs:
//!
//! - `$decl.VALUE` substitutes a value of the declaration: its built-ins
//!   `$decl.name`, `$decl.vis` (`pub(self)` when none was written), `$decl.doc`, and
//!   `$decl.attrs`, or a schema field (`$decl.enabled`; nested, `$decl.range.min`).
//! - `$for member in $decl.members { ... }` repeats per member; inside, the loop
//!   variable reaches the member's built-ins `$member.name`, `$member.vis`,
//!   `$member.doc`, `$member.attrs`, and `$member.index`, and its fields
//!   (`$member.pin`).
//! - `$if let Some(x) = $decl.optional { ... } else { ... }` reads an optional field
//!   or block through `$x`.
//! - `$ident(a, b, ...)` concatenates identifier parts exactly; `$snake(...)` and
//!   `$upper(...)` concatenate and then convert to `snake_case` or
//!   `SCREAMING_SNAKE_CASE`. Parts are identifiers, integers, `ident` fields,
//!   `$decl.name`, `$member.name`, or `$member.index`.
//!
//! A compound `expr` value is substituted in parentheses, so it keeps its precedence:
//! with `x: 1 + 2`, `$decl.x * 2` is `(1 + 2) * 2`. Literals, paths, calls, and other
//! self-delimiting expressions are substituted bare.
//!
//! Everything belonging to the declaration is reached through `$decl`; everything
//! belonging to a loop or `$if let` variable, through that variable. `$crate` refers
//! to the defining library. Templates are type-checked against the schema when
//! [`define!`] runs, so an unknown value, a missing field, or an optional read without
//! `$if let` is reported at the template. A schema field may not reuse a built-in name
//! of its namespace (`name`, `vis`, `doc`, `attrs`, `members` at the top level;
//! `name`, `vis`, `doc`, `attrs`, `index` in a member), and a variable may not be named
//! `decl` or `crate`.
//!
//! `$decl.doc` and `$member.doc` hold a generated description of the instance (its
//! macro, group, and field values), meant for `#[doc = $decl.doc]` after
//! `$decl.attrs`. If the caller writes their own doc text (`///` or
//! `#[doc = "..."]`) on that declaration or member, it arrives through `attrs` and the
//! generated doc is empty, so the caller's text replaces it rather than being appended
//! to. `#[doc(hidden)]` and other `doc(...)` attributes don't count as doc text.
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
//!         $decl.attrs
//!         #[doc = $decl.doc]
//!         $decl.vis struct $decl.name;
//!         impl $decl.name {
//!             pub const ENABLED: bool = $decl.enabled;
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
