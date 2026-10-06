# const-structures Specification

<!-- TODO0 consider deleting this spec once the work below is implemented and released. (may no longer apply: this is now the authoritative description of the implementation) -->

This document describes `const-structures` as implemented. It is the
authoritative reference for the schema language, the invocation syntax, the
`generate { ... }` template language, documentation generation, diagnostics,
and the macro export architecture. Where this document and the code disagree,
the code is right and this document has a bug.

Device Envoy is the first client. Its migration, including its pre-migration
baseline and the choices specific to it, is recorded in
[DE_MACRO_SURVEY.md](DE_MACRO_SURVEY.md).

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

## Architecture

```text
const-structures            facade crate (#![no_std]); re-exports the two proc macros
const-structures-derive     thin proc-macro shim
const-structures-core       parsing, validation, normalization, templates, docs (proc_macro2)

<library crate>
    #[doc(hidden)]
    pub use const_structures::expand as __const_structures_expand;    // once, at the crate root

    pub mod indicator {
        const_structures::define! { /// docs
                                    pub indicators { /* schema */ }
                                    generate { /* template */ } }
    }

<user crate>
    indicators! { pub Status {} }
```

The facade is `#![no_std]`, so embedded `no_std` libraries can depend on it.
Each client crate must re-export `expand!` at its crate root under the exact
name `__const_structures_expand`, because the generated wrappers call
`$crate::__const_structures_expand!`.

### What `define!` emits

For `pub indicators { ... } generate { ... }`, `define!` emits two items:

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

- The wrapper has a single catch-all rule. All parsing happens in `expand!`.
- The schema body is embedded verbatim, so diagnostics from `expand!` point at
  the schema and at the caller's input with their original spans.
- The `define!`'s visibility applies to the alias. Its non-doc attributes (for
  example `#[cfg(...)]`) go on both the wrapper and the alias. Its doc
  attributes go only on the alias.
- Inside a `macro_rules!` body, `$name` would be read as a metavariable, so the
  template's construct sigil `$` is re-marked as `#` when it is embedded.
  `$crate` is left alone, so `macro_rules!` resolves it to the defining crate.
- In the escape-hatch form (`pub NAME => PATH { ... }`), `template: { ... }` is
  replaced by `generator: { $crate::PATH }`.

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
`device-envoy-macros` crate). The alias removed that crate, which is what lets
a schema live beside the API it generates (see
[Why schemas live beside their APIs](#why-schemas-live-beside-their-apis)).

Because every generated reference goes through `$crate`, a client still works
when it is renamed in a user's `Cargo.toml`. Schema defaults can also use
`$crate::...` paths.

## Schema language

A definition takes one of two forms:

```text
{ATTR} VIS NAME { BODY } generate { TEMPLATE }      // template form
{ATTR} VIS NAME => GENERATOR_PATH { BODY }          // escape hatch (see below)
```

`BODY` is a comma-separated list of fields and at most one members section:

```text
{/// doc} [#[default_display = "..."]] NAME : KIND [= DEFAULT]     leaf field
{/// doc}                             NAME ?: KIND                 optional leaf
{/// doc}                             NAME [?]: { BODY }           nested block, optionally optional
{/// doc}                             members MIN..=MAX { BODY }   bounded members
{/// doc}                             members MIN.. { BODY }       members without an upper limit
```

- **Kinds:** `ident`, `expr`, and `ty`. A value is parsed with the parser for
  its kind, so commas inside a generic type or an expression stay part of that
  value. An `ident` is pasted into paths and identifiers (Device Envoy uses it
  for peripheral names like `PIN_3`). An `expr` can be any Rust expression,
  including a block expression.
- **Required, optional, defaulted:** a leaf with no default is required.
  `name?:` makes it optional, and the template sees it as absent or present.
  `= DEFAULT` supplies a value when the caller omits the field. A field cannot
  be both optional and defaulted.
- **`#[default_display = "..."]`** changes only how a single default value is
  shown in docs, for example `"Current::Milliamps(250)"` instead of
  `$crate::led_strip::CURRENT_DEFAULT`. The template still receives the real
  default tokens.
- **`by_index[A, B, ...]`** is a default for member fields, and for nested
  blocks inside members. The member at index `i` gets the `i`th value. A member
  past the end of the list must give the field explicitly.
- **Nested blocks** contain fields, never members.
- **Members** are allowed only at the top level of a schema, at most one
  section per schema. A field that is itself named `members` is written
  `members: KIND`. `members` followed by an integer starts a members section.
- **Field attributes** may be only doc comments and `#[default_display]`.

Example (this is [`examples/normalized.rs`](../examples/normalized.rs)'s schema):

```text
const_structures::define! {
    /// A named collection rendered by a template.
    pub channels {
        /// Required device address.
        address: ident,
        /// Whether polling is enabled by default.
        enabled: expr = true,
        /// One or more named channel members.
        members 1..=2 {
            /// The channel input.
            input: ident,
            /// Optional nested range limits.
            limits?: {
                /// Inclusive lower bound.
                min: expr,
                /// Inclusive upper bound.
                max: expr,
            },
        }
    }

    generate { /* see the template section */ }
}
```

## Invocation syntax

A caller writes one declaration:

```text
macro! { {ATTR} [VIS] Name { ITEM, ... } }

ITEM   = field: VALUE                 // leaf field
       | field: { field: VALUE, ... } // nested block
       | {ATTR} Member { ITEM, ... }  // a member, when the schema has members
```

```text
channels! {
    #[derive(Debug)]
    pub SensorChannels {
        address: ADDRESS_0,
        Temperature { input: INPUT_1, limits: { min: -40, max: 125 } },
        Humidity { input: INPUT_2 },
    }
}
```

- Fields and members may appear in any order, interleaved. A trailing comma is
  optional.
- Fields are keyword-only. Delimiters have exactly one form: a declaration or
  member is `Name { ... }` and a block is `field: { ... }`.
- Members inherit the group's visibility. Writing a visibility on a member is
  an error.
- Both the declaration and its members accept outer attributes, including doc
  comments.

After validation, each declaration and member has every schema field in schema
order. Defaults and `by_index` defaults are filled in, and optional fields are
marked present or absent.

## Template language

A template is ordinary Rust tokens plus four constructs, each introduced by `$`:

| Construct | Meaning |
| --- | --- |
| `$decl.VALUE`, `$var.VALUE` | Substitute a value |
| `$for var in $decl.members { ... }` | Repeat once per member |
| `$if let Some(var) = $optional { ... } else { ... }` | Branch on an optional field or block; `else` is optional |
| `$ident(...)`, `$snake(...)`, `$upper(...)` | Build an identifier from parts |

`$crate` passes through unchanged. Any other `$` that does not start one of
these constructs is an error.

### Namespaces

Everything that belongs to the declaration is reached through `$decl`.
Everything that belongs to a loop or `$if let` variable is reached through that
variable.

| Path | Value |
| --- | --- |
| `$decl.name` | The declared identifier |
| `$decl.vis` | Its visibility; `pub(self)` when none was written |
| `$decl.doc` | Its generated instance doc, as a string literal (see [Documentation](#documentation)) |
| `$decl.attrs` | Its outer attributes, as written |
| `$decl.members` | The member list; valid only as the target of `$for` |
| `$decl.FIELD`, `$decl.BLOCK.FIELD` | Schema fields |
| `$m.name`, `$m.vis`, `$m.doc`, `$m.attrs` | The same built-ins for a member `$m`; `vis` is the group's |
| `$m.index` | The member's zero-based position, as an unsuffixed integer literal |
| `$m.FIELD`, `$m.BLOCK.FIELD` | The member's fields |
| `$x`, `$x.FIELD` | An `$if let` binding: the optional leaf itself, or the optional block's fields |

The built-in names are reserved only within their own namespace. A top-level
schema field may not be named `name`, `vis`, `doc`, `attrs`, or `members`. A
member field may not be named `name`, `vis`, `doc`, `attrs`, or `index`. A
top-level field named `index`, or a member field named `members`, is allowed.
A variable may not be named `decl` or `crate`, or reuse a name already in scope.
`ident`, `snake`, and `upper` are constructs only when followed by `(`, so they
remain usable as variable names.

### Paths stop at leaves

A path consumes `.field` only while the current value has fields. Once it
reaches a leaf, a following `.` is ordinary Rust. So
`$panel.led_layout.width()` substitutes `led_layout` and keeps `.width()`.

### Expressions keep their precedence

A leaf renders the caller's tokens (or the default's tokens). An `expr` value
that is a compound expression is wrapped in parentheses, so it stays one
operand wherever the template puts it:

| Value of `x` | `$decl.x * 2` renders | `$decl.x.pow(2)` renders |
| --- | --- | --- |
| `1 + 2` | `(1 + 2) * 2` | `(1 + 2).pow(2)` |
| `-4` | `(-4) * 2` | `(-4).pow(2)` |
| `7` | `7 * 2` | `7.pow(2)` |
| `LIMIT` | `LIMIT * 2` | `LIMIT.pow(2)` |

Self-delimiting expressions stay bare: literals, paths, parenthesized and
tuple expressions, arrays, struct literals, macro calls, blocks, and calls,
method calls, field accesses, indexing, `?`, and `.await` on any of these. They
are already one operand, and staying bare keeps them valid where Rust requires
that exact form, such as `concat!($decl.label)`, `include_bytes!($decl.file)`,
or a literal as a bare const generic argument (`Holder::<$decl.len>`).
Everything else is parenthesized, including unary, binary, cast, range, and
closure expressions. `ident` and `ty` values are never wrapped.

Why parentheses: `macro_rules!` keeps an `$x:expr` capture's precedence by
wrapping it in an invisible group, which the parser honors. rustc flattens
invisible groups that a proc macro emits, so they don't protect anything here.
The `fixtures/demo` test `expression_values_keep_their_precedence` checks this
against rustc: without the parentheses, `x: 1 + 2` gave `$decl.x * 2 == 5`.

Two consequences follow:

- `stringify!($decl.x)` shows the parentheses for a compound value (`"(1 + 2)"`).
- A compound expression used as a const generic argument still needs braces in
  the template (`Holder::<{ $decl.n }>`), exactly as with `macro_rules!`.

The escape hatch is unaffected: a generator matches `$x:expr`, which gives it
the usual `macro_rules!` grouping.

### `$for`

`$for var in $decl.members { BODY }` renders `BODY` once per member, in
declaration order, with `$var` bound to that member. Members exist only at the
top level, so `$decl.members` is the only valid target. A `$for` may appear
anywhere tokens may, including inside a parameter list, a tuple type, or an
array expression:

```text
pub const MEMBER_COUNT: usize = 0 $for channel in $decl.members { + 1 };
```

### `$if let`

`$if let Some(var) = PATH { THEN } else { OTHERWISE }` requires `PATH` to be
an optional field or optional block. When the value is present, `THEN` renders
with `$var` bound to it. When it is absent, `OTHERWISE` renders, or nothing
does if there is no `else`. `$var` is in scope only in `THEN`. Reading an
optional value any other way is an error.

```text
pub const LIMITS: Option<(i32, i32)> = $if let Some(limits) = $channel.limits {
    Some(($limits.min, $limits.max))
} else {
    None
};
```

### Identifier construction

`$ident(PART, ...)` concatenates its parts into one identifier. `$snake(...)`
and `$upper(...)` concatenate, then convert the result to `snake_case` or
`SCREAMING_SNAKE_CASE`. Each part is one of:

- a plain identifier or an all-digit integer literal;
- an identifier-valued path: an `ident`-kind field, `$decl.name`, `$m.name`,
  `$m.index`, or an `$if let` binding of an optional `ident` field.

The resulting identifier takes the span of the first substituted value, so
errors on it point at the caller's input.

Snake case starts a new word at an uppercase letter that follows a lowercase
letter, at an uppercase letter that follows digits which themselves follow a
lowercase letter, and before the last capital of an acronym followed by a
lowercase letter. It never doubles an underscore. So `Gpio0LedStrip` becomes
`gpio0_led_strip`, `HTTPServer` becomes `http_server`, `Ir15Receiver` becomes
`ir15_receiver`, and all-caps text keeps its words (`LED2D` becomes `led2d`).

```text
static $upper($decl.name, _STATIC): ... ;          // Gpio0Led -> GPIO0_LED_STATIC
fn $snake($m.name, _pin)() { ... }                  // First -> first_pin
let ::embassy_rp::pio::Pio { $for s in $decl.members { $ident(sm, $s.index), } .. } = ...;   // sm0, sm1
```

### Checking

A template is parsed and type-checked against its schema when `define!` runs,
that is, when the library compiles, before any caller exists. Unknown values,
misspelled fields, reading an optional value without `$if let`, looping over
something other than `$decl.members`, using a non-identifier as an identifier
part, and bad construct syntax are all reported at the template. The
[diagnostics](#diagnostics) section lists the messages.

### A complete template

This is the template of [`examples/normalized.rs`](../examples/normalized.rs),
for the schema shown earlier:

```text
generate {
    $decl.attrs
    #[doc = $decl.doc]
    $decl.vis struct $decl.name;

    impl $decl.name {
        pub const ADDRESS: &'static str = stringify!($decl.address);
        pub const ENABLED: bool = $decl.enabled;
        pub const MEMBER_COUNT: usize = 0 $for channel in $decl.members { + 1 };
    }

    $for channel in $decl.members {
        $channel.attrs
        #[doc = $channel.doc]
        $channel.vis struct $channel.name;

        impl $channel.name {
            pub const INDEX: usize = $channel.index;
            pub const INPUT: &'static str = stringify!($channel.input);
            pub const LIMITS: Option<(i32, i32)> = $if let Some(limits) = $channel.limits {
                Some(($limits.min, $limits.max))
            } else {
                None
            };
        }
    }
}
```

### Why these four constructs

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

The `$decl` namespace was added for the same reason the language is small: one
rule ("the declaration's values are on `$decl`, a variable's values are on the
variable") replaces a list of special top-level names, and schema fields can
no longer collide with built-ins outside their own namespace.

## Documentation

Docs come from the schema, so they cannot drift from the macro.

### Macro docs

The `define!`'s hand-written docs (prose and examples) go on the public alias.
The generated syntax block and field tables go on the hidden wrapper. Rustdoc
shows a re-export's own docs followed by the original item's, so the macro's
page shows both. A crate that re-exports the macro under its own docs keeps the
generated tables. (Rustdoc drops the docs of intermediate re-exports across
crates, which is why the tables live on the wrapper rather than the alias.)

For the `channels` schema above, the generated part is:

````text
**Syntax:**

```text
channels! {
    [<attributes>] [<visibility>] <Name> {
        address: <ident>,
        enabled: <expr>, // optional, default: true
        [<attributes>] <MemberName> { // 1 to 2 members; visibility comes from the group
            input: <ident>,
            limits: { // optional
                min: <expr>,
                max: <expr>,
            },
        },
    }
}
```

**Fields:**

| Field | Kind | Default | Description |
| ----- | ---- | ------- | ----------- |
| `address` | ident | required | Required device address. |
| `enabled` | expr | `true` | Whether polling is enabled by default. |

**Member fields** (1 to 2 members; One or more named channel members.):

| Field | Kind | Default | Description |
| ----- | ---- | ------- | ----------- |
| `input` | ident | required | The channel input. |
| `limits` | block | optional | Optional nested range limits. |
| `limits.min` | expr | required | Inclusive lower bound. |
| `limits.max` | expr | required | Inclusive upper bound. |
````

Defaults are shown pretty-printed, or with their `default_display` spelling.
A `by_index` default is shown as `by member index: ...`.

### Instance docs

Every declaration and member gets a generated description of the configuration
the caller used, available to the template as `$decl.doc` and `$m.doc`. For the
invocation above, `SensorChannels` gets:

```text
Generated by `channels!`.

| Field | Value |
| ----- | ----- |
| `address` | `ADDRESS_0` |
| `enabled` | `true` (default) |

Members: `Temperature`, `Humidity`.
```

and `Humidity` gets:

```text
Member `Humidity` of `SensorChannels`, generated by `channels!`.

| Field | Value |
| ----- | ----- |
| `input` | `INPUT_2` |
| `limits` | `(not set)` |
```

Defaulted values are marked `(default)`, and absent optional values show as
`(not set)`.

**A written doc replaces the generated one.** If the caller writes doc text on
a declaration or member, either as `///` comments or as `#[doc = "..."]`, it
reaches the template through `attrs`, and that item's `doc` is the empty
string. The usual template pattern `$decl.attrs #[doc = $decl.doc]` therefore
shows the caller's text in place of the generated description, not appended to
it. `#[doc(hidden)]` and other `doc(...)` forms are not doc text and do not
suppress the generated doc.

This is also how one macro forwards to another without double-documenting.
Device Envoy's `led_strip!` renders as a one-member `led_strips!` group and
passes `$decl.attrs #[doc = $decl.doc]` on the member. The member then
carries `led_strip!`'s description rather than a generated "Member of ..." one.

## Diagnostics

Errors are `syn::Error`s with spans on the offending tokens. During invocation
checking, field errors from the declaration and from every member are collected
and reported together.

**When `define!` runs (schema):**

| Mistake | Message |
| --- | --- |
| Unknown kind | ``unknown field kind; expected `ident`, `expr`, or `ty` `` |
| `?` together with `= default` | ``an optional (`?`) field cannot also have a default`` |
| `default_display` without a single default | `` `default_display` needs a single default value (`= ...`) `` |
| `by_index` outside member fields | `` `by_index` defaults are allowed only in member fields `` |
| Members inside a block or member | `members are allowed only at the top level of a schema` |
| Second members section | `duplicate members section` |
| Repeated field | ``duplicate schema field `x` `` |
| `members 3..=1` | `member range is empty` |
| Other attributes on a field | ``only doc comments and `#[default_display = "..."]` are allowed on schema fields`` |
| Field named like a built-in | ``field `name` collides with the template's built-in `$decl.name`; rename the field`` |
| `=> PATH` and `generate` together | ``a `generate { ... }` template replaces `=> generator`; use one or the other`` |
| Neither | ``expected `generate { ... }` after the schema`` |
| `=> ::path` | ``the generator path is relative to this crate's root; drop the leading `::` `` |

**When `define!` runs (template):**

| Mistake | Message |
| --- | --- |
| `$strip.pnael` | ``no value `pnael` here; expected one of `name`, `vis`, `doc`, `attrs`, `index`, `pin`, `dma`, `panel` `` |
| `$bus` (a declaration field) | ``unknown template value `$bus`; write `$decl.bus` `` |
| `$strp.pin` (no such variable) | ``unknown template value `$strp`; expected `$decl`, `$strip` `` |
| `$strip.panel` (optional) | `` `$strip.panel` is optional; read it with `$if let Some(x) = ... { ... }` `` |
| `$if let ... = $decl.bus` (required) | `` `$decl.bus` is not optional; `$if let` needs an optional field `` |
| `$for s in $decl.bus` | `` `$decl.bus` is not the member list; loop over `$decl.members` `` |
| `$decl.members` as a value | `` `$decl.members` is the member list; loop over it with `$for member in $decl.members { ... }` `` |
| `$decl` alone | `` `$decl` is the declaration, not a value; name one of ... `` |
| A block as a value | `` `$x.limits` is a block, not a value; name one of its fields `` |
| Non-identifier in `$ident(...)` | `` `$decl.doc` is not an identifier; identifier parts must be `ident` fields, `$decl.name`, `$member.name`, or `$member.index` `` |
| Stray `$` | `` `$` must start a template construct (`$value`, `$for`, `$if`, `$ident(...)`) or `$crate` `` |
| `$for decl in ...` | `` `decl` is reserved in templates; choose another variable name `` |
| Shadowing a variable | `` `$strip` is already in scope; choose another variable name `` |

**When a caller invokes the macro:**

| Mistake | Message |
| --- | --- |
| Unknown field | ``unknown field `x`; expected one of `a`, `b` (or a member `Name { ... }`)`` |
| Repeated field | ``duplicate field `x` `` |
| Missing required field | ``missing required field `x` `` |
| Wrong kind | ``field `x` expects an identifier`` (or `an expression`, `a type`) |
| Value given for a block | ``field `x` expects `{ ... }` `` |
| Block given for a value | `expected a value, not a block` |
| Member count | `` `channels!` takes 1 to 2 members; found 3 `` |
| Repeated member | ``duplicate member `X` `` |
| Visibility on a member | `members take their group's visibility; remove this visibility` |
| `Name: { ... }` | ``remove the `:` after `Name`; declarations are written `Name { ... }` `` |
| Member where the schema has none | `` `Name` takes fields only; expected one of ... `` |
| Member past the `by_index` list | ``field `x` has no default for member index 4; give it explicitly`` |
| Tokens after the declaration | `unexpected tokens after the declaration body` |

Caller values keep their spans through rendering, so a type error in generated
code points at the caller's value. The `alias-regression` UI case
`template_caller_span` checks this: `size: "wide"` is reported as `expected
u32, found &str` at `"wide"`.

## The generator escape hatch

`pub NAME => GENERATOR_PATH { BODY }` sends the validated declaration to a
library `macro_rules!` generator instead of a template. `GENERATOR_PATH` is
relative to the defining crate's root (the wrapper calls `$crate::PATH`). The
generator receives every field in schema order:

```text
attrs: [ATTR*], vis: [VIS], name: NAME, doc: "...",
FIELD: VALUE, ...
member_count: N,                                   // only when the schema has members
members: [ { index: I, attrs: [...], vis: [...], name: NAME, doc: "...", FIELD: VALUE, ... }, ... ],
```

- An inherited visibility is passed as `pub(self)`, because `$vis:vis` cannot
  match an empty visibility at the end of `[...]`.
- An optional field arrives as `[]` when absent or `[VALUE]` when present.
- A block arrives as `{ FIELD: VALUE, ... }`, and an optional block as
  `[{ ... }]` or `[]`.

The escape hatch is kept for output a template cannot express. It is covered by
the core tests and the `fixtures/demo` crate. Device Envoy no longer uses it.

## Why schemas live beside their APIs

A schema, its docs, its template, and the Rust items the template calls change
together. Keeping them in one place means one review sees all of them. The
generated code also names chip-specific HAL types and crate-private helpers, so
the declaration belongs in the crate that owns those types. A separate schema
crate cannot see them.

The proc-macro-per-schema design made colocation impossible, because proc
macros must live in a proc-macro crate. The wrapper-plus-alias design puts
`define!` in an ordinary library module, so Device Envoy's
`crates/device-envoy-rp/src/led_strip.rs` holds the `led_strips!` schema, its
template, and the `LedStrip` types it instantiates.

## Testing

- `const-structures-core` unit tests cover parsing, defaults, member handling,
  generated docs, template rendering, template diagnostics, embedding, snake
  case, and the written-doc rule.
- `fixtures/demo` is a client crate with a downstream test and trybuild UI cases
  under `tests/ui/`: unknown field, duplicate plus missing fields, wrong kind,
  too many members, member visibility, a template typo, and an unknown clause.
- `fixtures/alias-regression` and `fixtures/renamed-user` set
  `#![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]` and
  `#![deny(warnings)]`. They cover internal, module, and root calls, calls from
  another macro, downstream imports and full paths, further re-exports, and a
  renamed dependency. The alias fixture also builds rustdoc and checks the
  alias page and the generated field docs. Its UI control
  `alias_via_crate_path` must fail specifically when an alias refers to
  `crate::__const_structures_wrapper_widget`.
- `cargo run --example normalized` runs the example above.

Device Envoy's own suites (`cargo check-all`, RP compile-only tests and demos,
ESP embedded compile tests, and `just docs`) exercise every client macro on
hardware targets.
