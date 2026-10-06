// These link definitions come before the README, so rustdoc uses them for the
// README's reference links; GitHub uses the README's own definitions.
//! [`define!`]: macro@define
//! [`expand!`]: macro@expand
#![doc = include_str!("../README.md")]
#![no_std]

#[doc = include_str!("define.md")]
pub use const_structures_derive::define;

/// The procedural macro that every macro defined with [`define!`](macro@define)
/// calls. You re-export it once and never call it yourself.
///
/// Each crate that uses [`define!`](macro@define) must re-export `expand!` at
/// its crate root, under exactly this name:
///
/// ```text
/// #[doc(hidden)]
/// pub use const_structures::expand as __const_structures_expand;
/// ```
///
/// A defined macro is a small generated `macro_rules!` macro that forwards its
/// caller's tokens, together with its schema and template, to
/// `$crate::__const_structures_expand!`. Because the forwarding goes through
/// `$crate`, the generated macros work from other crates, including ones that
/// rename yours.
///
/// `expand!` parses the call, checks it against the schema, fills in defaults,
/// builds the generated documentation, and renders the template (or calls the
/// [escape-hatch generator](macro@define#escape-hatch-a-macro_rules-generator)).
/// The repository's `specs/CONST_STRUCTURES_SPEC.md` describes these steps in
/// detail.
///
/// Its input format is internal and may change; only generated macros should
/// call it.
pub use const_structures_derive::expand;
