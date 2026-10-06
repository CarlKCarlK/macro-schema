# const-structures Specification

<!-- TODO0 consider deleting this spec once the work below is implemented and released. (may no longer apply: this is now the authoritative description of the implementation) -->

This document is for contributors. It explains how `const-structures` works
inside and why it is designed the way it is. Where this document and the code
disagree, the code is right and this document has a bug.

To *use* the crate, start elsewhere:

| Document | Contents |
| --- | --- |
| `README.md` (also the crate's rustdoc front page) | Why the crate exists, a quick start, the template language by example, errors, generated docs |
| `src/define.md` (the rustdoc of `define!`) | The complete language reference: schema, invocation syntax, every template construct, documentation rules, the error catalog, the escape hatch |
| The rustdoc of `expand!` (in `src/lib.rs`) | What `expand!` is and the required re-export |
| `examples/quick_start.rs`, `examples/commands.rs` | Runnable programs; the README quotes the first verbatim |
| [DE_MACRO_SURVEY.md](DE_MACRO_SURVEY.md) | Device Envoy's migration, the first real client |

This document does not repeat the language reference. It refers to sections of
`src/define.md` by name.

## What the framework replaces

A declaration macro has two halves: a **parser** that accepts the caller's
syntax and a **generator** that emits code. Before this framework, Device Envoy
wrote both halves by hand in `macro_rules!`, separately for each macro.

The original goal was to replace the parsers: declare each macro's fields once,
and get parsing, validation, defaults, diagnostics, and documentation from that
declaration. That goal is met.

The result went further. The generators were replaced too:

- **Parsers:** none remain. Every Device Envoy declaration macro (33 across
  the core, RP, and ESP crates) is declared with `define!`. The caller's syntax
  is parsed and validated by the framework, never by a hand-written matcher.
- **Generators:** every one of those 33 macros renders its output from a
  `generate { ... }` template written next to its schema. No Device Envoy macro
  uses the `=> generator` escape hatch. The hand-written `macro_rules!` that
  remain are small internal helpers a template calls (for example, choosing an
  output engine by value) and macros that are not declarations at all (`tone!`,
  `combine!`, `tga!`, `init_and_start!`). The survey lists them and explains why
  each one stays.

So a library author writes three things in one place: the schema, its docs,
and a template. The framework supplies everything else.

## Crates

```text
const-structures            facade (#![no_std]): re-exports the two proc macros; holds the user docs
const-structures-derive     proc-macro shim: converts TokenStreams and errors, nothing else
const-structures-core       the implementation, over proc_macro2, unit-testable
    schema.rs               define!: parse a definition, generate macro docs, emit the wrapper and alias
    instance.rs             expand!: parse a call, validate, fill defaults, build instance docs, dispatch
    template.rs             the template language: parse and type-check, embed, render
    value.rs                field kinds (ident, expr, ty) and value pretty-printing for docs
    tests.rs                unit tests, including template rendering and diagnostics
```

The facade is `#![no_std]` so that embedded `no_std` libraries can depend on
it. Everything is implemented in `const-structures-core` as ordinary functions
from `proc_macro2::TokenStream` to `syn::Result<TokenStream>`, so it can be
unit tested and debugged without the compiler's macro expander. The derive
crate turns an `Err` into `compile_error!` with `syn::Error::into_compile_error`.

## Architecture

### What `define!` emits

For

```text
const_structures::define! {
    /// Hand-written docs.
    pub indicators { /* schema */ }
    generate { /* template */ }
}
```

`define!` emits two items:

```text
#[doc = "<generated Syntax block and field tables>"]
#[doc(hidden)]
#[macro_export]
macro_rules! __const_structures_wrapper_indicators {
    ($($input:tt)*) => {
        $crate::__const_structures_expand! {
            macro_name: "indicators",
            template: { <the template, with `$` re-marked as `#`> },
            schema: { <the schema body tokens> },
            input: { $($input)* },
        }
    };
}

<the define!'s own attributes, including its hand-written docs>
#[doc(inline)]
pub use __const_structures_wrapper_indicators as indicators;
```

- **Why a generated `macro_rules!`.** A proc macro must live in a proc-macro
  crate. A `macro_rules!` wrapper is an ordinary item, so it can be emitted in
  the library's own module, beside the API it generates. The wrapper has one
  catch-all rule: all parsing happens in `expand!`.
- **How the schema reaches the expander.** The schema body is embedded in the
  wrapper verbatim, and `expand!` re-parses it on every call. Keeping the
  original tokens keeps their spans, so errors about the schema point into the
  library's source.
- **Why `$` becomes `#`.** Inside a `macro_rules!` body, `$name` would be read
  as a metavariable. The template's construct sigil is re-marked as `#` when it
  is embedded (`template::embed`), and `expand!` parses the embedded template
  with `#` as its sigil (`EMBEDDED_SIGIL`). `$crate` is left alone, so
  `macro_rules!` resolves it to the defining crate.
- **Attributes.** The `define!`'s visibility applies to the alias. Its non-doc
  attributes, such as `#[cfg(...)]`, go on both items. Its doc attributes go
  only on the alias.
- **Docs placement.** The generated syntax block and field tables go on the
  hidden wrapper, and the hand-written docs on the alias. Rustdoc shows a
  re-export's own docs followed by the original item's, so the macro's page
  shows both. Rustdoc drops the docs of intermediate re-exports across crates,
  so tables placed on the alias would vanish when another crate re-exports the
  macro with its own docs.
- **Escape hatch.** In the form `pub NAME => PATH { ... }`, `template: { ... }`
  is replaced by `generator: { $crate::PATH }`.

### Why the bare-name alias

rustc rejects absolute-path access to a `#[macro_export]` macro that was itself
produced by macro expansion (`macro_expanded_macro_exports_accessed_by_absolute_paths`,
deny-by-default and slated to become a hard error). A
`pub use crate::__const_structures_wrapper_NAME` would trigger it. A bare
`pub use __const_structures_wrapper_NAME as NAME;` in the same expansion
re-exports the macro from textual scope instead, which gives it an ordinary
path. Other modules and crates then reach the macro through that alias:
`crate::indicator::indicators`, a crate-root `pub use indicator::indicators;`,
downstream imports, and full paths all work.

**Never refer to the hidden wrapper by its crate path.** Refer to the alias.

An earlier design generated one `#[proc_macro]` per schema. That forced every
client to keep its schemas in a separate proc-macro crate (Device Envoy had a
`device-envoy-macros` crate). The alias removed that crate.

### `$crate`

Every reference the generated code makes to the client goes through `$crate`:
the wrapper calls `$crate::__const_structures_expand!`, templates write
`$crate::...`, and schema defaults may too. `$crate` survives because the
template and schema tokens are embedded in the client's own `macro_rules!`
wrapper, where `$crate` means the client crate. So a client still works when a
user renames it in `Cargo.toml`. (`define!` itself reads `$crate` in a default
as `crate` while validating and documenting, in `dollar_crate_as_crate`; the
wrapper keeps the original tokens.) That is also why each client must re-export
`expand!` at its crate root under the fixed name `__const_structures_expand`.

## How a call is expanded

### When the library compiles: `define!`

`schema::define`:

1. Parses the definition (`Definition`): attributes, visibility, name, the
   optional `=> PATH`, the schema body, and the `generate` block.
2. Parses the schema into a `BodySpec` (fields, kinds, defaults, blocks, one
   optional `MembersSpec`) and reports schema errors.
3. Parses and type-checks the template against the `BodySpec`
   (`Template::parse` with sigil `$`). This is why template mistakes are
   reported when the library compiles. Checking resolves every path against
   the schema, tracking `$for` and `$if let` variables in scopes.
4. Generates the macro docs (`macro_doc`).
5. Emits the wrapper and alias shown above.

### When a user calls the macro: `expand!`

The wrapper forwards the call to `instance::expand`, which:

1. Parses its input (`ExpandInput`): the macro name, the template or generator
   path, the schema tokens (re-parsed into a `BodySpec`), and the user's tokens.
2. Parses the user's declaration (`Declaration`). Values are parsed by their
   schema kind, so a comma inside `Vec<A, B>` stays inside the value. Members
   are recognized by parsing ahead for `{attrs} [vis] Name {`, which also
   accepts a visibility forwarded as a `macro_rules!` `$vis`. Structural errors,
   such as a `:` after a name or a visibility on a member, stop here.
3. Resolves fields (`resolve_fields`): walks the schema in order, takes the
   user's value or the default (`by_index` uses the member's position), and
   marks optional fields absent or present. Unknown, duplicate, and missing
   fields, wrong member counts, and duplicate members are collected in an
   `Errors` value and combined with `syn::Error::combine`, so the user sees
   them all at once.
4. Builds instance docs (`instance_doc`), unless the declaration or member
   carries written doc text.
5. Produces output:
   - **Template form:** re-parses the embedded template (sigil `#`), builds a
     `Data` tree (built-ins, fields, and members with their `index`), and
     renders it. Rendering copies tokens, substitutes values, repeats `$for`
     bodies, picks `$if let` branches, and builds identifiers.
   - **Generator form:** emits `PATH! { attrs: [...], vis: [...], name: ...,
     doc: "...", fields..., member_count: N, members: [...] }`. The format is
     documented under "Escape hatch" in `src/define.md`.

### Spans and diagnostics

- Values are the user's own tokens, inserted unchanged, so a type error in
  generated code points at the user's value. The `alias-regression` UI test
  `template_caller_span` checks this.
- A compound `expr` value is wrapped in a parenthesis group with the call-site
  span, around the user's tokens (see the rationale below).
- An identifier built by `$ident`, `$snake`, or `$upper` takes the span of the
  first value inserted into it, so errors about it point at the user's name.
- Template tokens come from the library's wrapper, so errors in them point
  into the library's `generate` block.
- Framework errors are `syn::Error`s spanned on the offending token. The full
  catalog is under "Errors" in `src/define.md`.

## Framework versus library

The framework owns everything that follows from the schema: syntax, parsing,
validation, defaults, member handling, documentation, and the mechanics of
substitution. The library owns everything that gives a declaration meaning:
which items are generated, their visibility and representation, their
constructors and tasks, and any domain rules beyond the schema (for example,
Device Envoy's unique LCD addresses, checked by a `const` assertion its
template emits). The framework never names a client concept.

## Design rationale

### Why these four template constructs

Each construct corresponds to one structural feature of a schema:

| Schema feature | Template construct |
| --- | --- |
| fields and built-ins | value substitution |
| members | `$for` |
| optional fields and blocks | `$if let` |
| generated item names (previously `paste!`) | `$ident` / `$snake` / `$upper` |

Because every construct is tied to the schema, the whole template can be
type-checked when the library compiles. The language deliberately has no
comparison or branching on *what* a value is, no arithmetic, no string
operations, and no user-defined functions. Computation belongs in Rust (consts,
`const fn`, traits), and the template passes values to it. When output truly
depends on a value's spelling, a template calls a small client `macro_rules!`
helper. Device Envoy's ESP `led_strip!` does this: it forwards
`[$if let Some(chosen) = $decl.engine { $chosen }]` to a helper that reduces
`Engine::Spi` or `Engine::Rmt` to a choice.

This boundary was tested against the whole Device Envoy corpus. Every
declaration macro was ported without needing a fifth construct.

The first prototype took a different approach. It generated a `macro_rules!`
matcher from the schema, and the author wrote output with native
metavariables: flattened names such as `$field_range_min` and
`$member_field_input`, inside `$( ... )?` and `$( ... )*` repetitions that
mirrored the schema's nesting. That was dropped for four reasons. Authors had
to learn the flattening rules. Repetitions could express "present" but not
"absent" without extra helper macros to simulate `else`. Identifier
construction still needed `paste!`. And mistakes surfaced only when a caller
happened to hit them.

### Why `$decl`

One rule replaces a list of special top-level names: the declaration's values
are on `$decl`, and a variable's values are on the variable. Schema fields
cannot collide with built-ins outside their own namespace, so a top-level field
may be named `index` and a member field `members`.

### Why compound expressions are parenthesized

`macro_rules!` keeps an `$x:expr` capture's precedence by wrapping it in an
invisible group, which the parser honors. rustc flattens invisible groups that
a proc macro emits, so they don't protect anything here. This was verified
against rustc: with invisible groups, `x: 1 + 2` still gave `$decl.x * 2 == 5`.
The `fixtures/demo` test `expression_values_keep_their_precedence` checks the
current behavior (`instance::template_tokens`).

Wrapping every expression would break forms Rust requires to be bare, such as
`concat!("a")`, `include_bytes!("f")`, and a literal const generic argument,
and could trigger `unused_parens`. So only compound expressions are wrapped
(`instance::is_self_delimiting`). The exact rule is under "Expressions keep
their precedence" in `src/define.md`.

### Why `generate` is required

A schema defines the accepted declaration language; a template defines its
meaning. Because schemas specify syntactic kinds (`ident`, `expr`, `ty`) rather
than Rust types, the framework cannot generally infer useful generated code.
Therefore `generate` remains required. Built-in generators may be reconsidered
if a compelling general use case emerges.

This was tested against all 33 Device Envoy schemas (October 2026). An implicit
default, built-in generators such as `generate: struct`, a default with
customization, and a trait-impl representation of the declaration were each
evaluated. None removed the template of any schema. The evidence:

- 63 of 144 leaf fields are `ident`s, mostly names pasted into type paths
  (`peripherals::PIN_3`), not values.
- Generated structs hold runtime resources (`&'static` state, handles), never
  the schema's fields. Schema values size types and feed constructors.
- Per-instance `static`s and Embassy tasks cannot be generic, so a trait-impl
  representation consumed by ordinary generic Rust still needs a template.
- For plain constant data, Rust already suffices:
  `const CFG: Config = Config { a: 1, ..Config::DEFAULT }`.

**Deferred: deriving a single-item macro from its group macro.** About ten
Device Envoy single/group pairs (for example `lcd_text!` as a one-member
`i2cs!`) share structure. Deriving one schema from the other would save
roughly 200 template lines across 33 schemas, which does not justify the
estimated 300–400 lines of framework machinery. Revisit if such pairs become
common.

### Why schemas live beside their APIs

A schema, its docs, its template, and the Rust items the template calls change
together. Keeping them in one place means one review sees all of them. The
generated code also names the library's own types, which may be private or
platform-specific, so the declaration belongs in the crate that owns them. A
separate schema crate cannot see them. The wrapper-plus-alias design puts
`define!` in an ordinary library module, so Device Envoy's
`crates/device-envoy-rp/src/led_strip.rs` holds the `led_strips!` schema, its
template, and the types it instantiates.

### Why the escape hatch remains

A `macro_rules!` generator can do what a template deliberately can't, such as
choose output by matching a value's tokens. It is kept for that, with the same
validation and normalization in front of it. Device Envoy no longer uses it; it
is covered by the core tests, the `fixtures/demo` crate, and a `define!`
doctest.

## Testing

- `const-structures-core` unit tests cover parsing, defaults, members, generated
  docs, template rendering and type-checking, expression parenthesization,
  embedding, snake case, and the written-doc rule.
- The facade's doctests run every Rust example in `README.md` and
  `src/define.md`. `tests/readme.rs` checks that the README's quick start is
  byte-for-byte `examples/quick_start.rs`, and that the compiler errors the
  README quotes match the `readme_*` UI tests.
- `fixtures/demo` is a client crate with a downstream test and trybuild UI cases
  under `tests/ui/` for rejected input: unknown field, duplicate and missing
  fields, wrong kind, too many members, member visibility, template mistakes,
  and the two errors the README shows.
- `fixtures/alias-regression` and `fixtures/renamed-user` set
  `#![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]` and
  `#![deny(warnings)]`. They cover internal, module, and root calls, calls from
  another macro, downstream imports and full paths, further re-exports, and a
  renamed dependency. The alias fixture also builds rustdoc and checks the
  alias page and the generated field docs. Its UI control
  `alias_via_crate_path` must fail specifically when an alias refers to
  `crate::__const_structures_wrapper_widget`.
- `cargo run --example quick_start` and `cargo run --example commands` run the
  examples, which assert their own results.

Device Envoy's own suites (`cargo check-all`, RP compile-only tests and demos,
ESP embedded compile tests, and `just docs`) exercise every client macro on
hardware targets.
