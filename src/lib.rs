// These link definitions come before the README, so rustdoc uses them for the
// README's reference links; GitHub and crates.io use the README's own definitions.
//! [`define!`]: macro@define
//! [`expand!`]: macro@expand
#![doc = include_str!("../README.md")]
#![no_std]

#[doc = include_str!("define.md")]
pub use macro_schema_derive::define;

/// The procedural macro that every macro defined with [`define!`](macro@define)
/// calls. You re-export it once and never call it yourself.
///
/// Each crate that uses [`define!`](macro@define) must re-export `expand!` at
/// its crate root, under exactly this name:
///
/// ```text
/// #[doc(hidden)]
/// pub use macro_schema::expand as __macro_schema_expand;
/// ```
///
/// A defined macro is a small generated `macro_rules!` macro that forwards its
/// caller's tokens, together with its schema and template, to
/// `$crate::__macro_schema_expand!`. Because the forwarding goes through
/// `$crate`, the generated macros work from other crates, including ones that
/// rename yours.
///
/// `expand!` parses the call, checks it against the schema, fills in defaults,
/// builds the generated documentation, and renders the template (or calls the
/// [escape-hatch generator](macro@define#escape-hatch-a-macro_rules-generator)).
/// The repository's `specs/MACRO_SCHEMA_SPEC.md` describes these steps in
/// detail.
///
/// Its input format is internal and may change; only generated macros should
/// call it.
pub use macro_schema_derive::expand;
