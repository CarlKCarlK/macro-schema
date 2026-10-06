# const-structures Spec

<!-- TODO0 consider deleting this spec once the work below is implemented and released. -->

## Goal and current state

`const-structures` is a general-purpose proc-macro framework for declaring
named, schema-checked macro inputs. A client writes each schema beside its own
code generator with `const_structures::define!`; the shared
`const_structures::expand!` parses and validates invocations, inserts defaults,
builds diagnostics and instance documentation, then calls the client generator.
The framework does not own or generate a client's domain types.

Device Envoy is the first substantial client and migration corpus, not a
compatibility target. Its declaration macros use this framework; small
handwritten expression and initialization macros remain ordinary
`macro_rules!` macros. Device Envoy-specific names, defaults, hardware kinds,
and code-generation rules belong in Device Envoy schemas and generators, not
in this crate.

The design target is a small framework with one definition wrapper per public
macro and one expansion proc macro for parsing and validation. Schemas are
client-local. The framework supplies common syntax, field kinds, optional and
default values, nested blocks, member declarations, diagnostics, and generated
configuration docs. It does not require unrelated client macros to share
field names or semantics.

The current prototype also accepts an optional `generate { ... }` clause
after a schema. Its contents are ordinary `macro_rules!` output template
tokens. The framework uses the schema to generate a matcher for those tokens
and a hidden normalized backend macro; it does not introduce another template
language. A definition without `generate` continues to use an explicitly
declared generator macro, which remains useful for bespoke multi-arm
generation.

## Schema and documentation

Each client field is declared once with its name, kind, optional/default
behavior, and documentation. The framework derives invocation parsing,
default insertion, validation diagnostics, and the macro's syntax and field
tables from that schema. Generated declarations receive documentation listing
the resolved configuration, with defaulted values marked and values formatted
for readability. Client prose, examples, and generated-type stubs remain
client-owned; Device Envoy keeps `cfg(doc)` sample types for hardware-generated
APIs. Client generators also decide visibility and whether generated types
expose public constructors. Device Envoy keeps helper statics/tasks and
synthetic one-member groups private. Constructors may be `pub`; the generated
type’s own visibility controls access.

## Framework requirements

1. A client defines a macro with a schema and a generator path relative to its
   own crate root. The schema stays beside the code it describes.
2. One generated `macro_rules!` wrapper forwards to the single `expand!` proc
   macro. Expansion validates the invocation and calls the client generator
   with normalized fields.
3. Fields have declared kinds, required or optional status, and optional
   defaults. Unknown, duplicate, missing, and invalid inputs receive spanned
   diagnostics.
4. The framework supports attributes and visibility on declarations, nested
   field blocks, and bounded or unbounded member sections. Members inherit
   container visibility; an explicit member visibility is an error.
5. `by_index[...]` defaults may supply a value according to a member's index.
   Generators receive normalized fields, member indices, and member counts.
6. Field order is accepted independently of a schema's semantic meaning.
   Framework syntax rules apply uniformly; schemas own names, kinds,
   defaults, and domain-specific constraints.
7. `$crate` hygiene keeps generated wrapper references and schema defaults
   valid when a client dependency is renamed. A generated macro wrapper is
   first re-exported by bare name in its defining scope; do not refer to its
   hidden exported name through a crate path.
8. Schema field documentation and defaults drive the generated configuration
   documentation. Client-authored macro docs and examples remain with the
   client schema.
9. In the template form, the schema generates a matcher for normalized
   metadata, fields, optional values, and members. Definition-level attributes
   such as `cfg` apply to the generated backend and public macro alias.

## Device Envoy choices

These choices regularize this client and are not requirements on other users
of `const-structures`:

- Shared group fields live inside the group braces, beside members. Members
  use `Name { ... }`; fields use `name: value`.
- Declarations and members accept any field order, an optional trailing
  comma, and outer attributes. Members inherit the containing declaration's
  visibility.
- Device Envoy schemas use one spelling per field and use named declarations
  for `servo!` on both RP and ESP.
- Device Envoy chooses which field names, defaults, peripheral identifier
  kinds, cross-member constraints, and generated items each macro supports.
  For example, reconciling `max_frames` and `max_steps` is a client decision.
- Positional or statement-oriented helpers such as `tone!`, `combine!`,
  `tga!`, `pio_split!`, and `init_and_start!` remain handwritten
  `macro_rules!` macros because they are not schema-backed declarations.

## Schema and invocation grammar

A definition names the public macro, the client-local generator, and the schema.
Schema fields use the kinds `ident`, `expr`, and `ty`. A field may be required,
optional (`?`), defaulted (`= value`), or a nested block. Optional fields
cannot also declare defaults. A nested block contains fields and may itself
be optional.

```text
const_structures::define! {
    /// Client-authored macro documentation.
    pub configure => button::configure_generate {
        /// Required resource name.
        resource: ident,
        /// Omitted value is passed as an empty bracket group.
        timeout?: expr,
        /// A default may use the client crate's hygienic path.
        retries: expr = $crate::DEFAULT_RETRIES,
        /// Per-member defaults use declaration order, starting at zero.
        members 1..=2 {
            /// Channel defaults by member index.
            channel: ident = by_index[CHANNEL_0, CHANNEL_1],
            /// Nested configuration.
            panel?: {
                width: expr,
            },
        },
    }
}
```

`members MIN..=MAX { ... }` declares a bounded member section;
`members MIN.. { ... }` has no maximum. Each member is written `Name { ... }`
inside an invocation. Members inherit the container visibility, and spelling a
visibility on a member is rejected. Declarations and members accept outer
attributes. The schema's member section controls whether members are allowed
and their valid count.

Input declarations use `macro! { [attributes] [visibility] Name { ... } }`.
Fields use `field: value`; members use `Name { ... }`. Fields can appear in
any order and a trailing comma is optional. Nested field blocks use braces; leaf `expr` fields accept ordinary Rust
expressions, including block expressions. The schema distinguishes them. The parser
uses schema kinds to parse values, so generic type and expression commas stay
inside their token syntax.

The generator receives each field in schema order. Required/defaulted leaf
fields arrive as values; an omitted optional field arrives as `[]`, and a
provided optional field as `[value]`. Optional blocks use the same empty or
single-bracketed-block representation. Groups also receive `member_count` and
member entries in declaration order; each entry carries its zero-based `index`,
attributes, inherited visibility, name, generated configuration doc, and
normalized fields. A `by_index[...]` default selects the value at the member's
index; if no indexed value exists, the caller must supply that field.

Schema field doc comments describe fields in generated syntax and tables.
`#[default_display = "..."]` changes how a single-value default is shown in
documentation without changing the tokens passed to the generator. The macro's
syntax and field tables are generated from the schema. Each generated item
receives a configuration table showing resolved values and marking defaults.
These generated tables use documentation text; client-owned prose and examples
remain ordinary client documentation.

## Example (Device Envoy syntax)

```text
led_strips! {
    pub LedStrips0 {
        pio: PIO0,
        Gpio0LedStrip { pin: PIN_0, len: 8, max_current: Current::Milliamps(25) },
        Gpio4Led2d {
            pin: PIN_4,
            len: 96,
            max_current: Current::Milliamps(250),
            led2d: { led_layout: LED_LAYOUT_12X8_ROTATED, font: Led2dFont::Font4x6Trim },
        },
    }
}

ir_mappings! {
    pub Remotes {
        pio: PIO1,
        button: RemoteButton,
        capacity: 8,
        LeftRemote { pin: PIN_15 },
    }
}

i2cs! {
    pub Lcds {
        i2c: I2C0,
        sda_pin: PIN_4,
        scl_pin: PIN_5,
        Top { width: 16, height: 2, address: 0x27 },
    }
}

servo! { Servo11 { pin: PIN_11 } }
```

## Architecture

```text
const-structures (general; knows nothing about any user library)
    define!   declares one macro from a schema; generates syntax and field docs
              optionally generates a normalized template backend matcher
    expand!   generic parse / validate / default / diagnostics / dispatch

<library> (schemas live next to the code they generate)
    #[doc(hidden)]
    pub use const_structures::expand as __const_structures_expand;   // once, at crate root

    pub mod button {
        const_structures::define! {
            /// Client-authored prose and examples.
            #[cfg(feature = "buttons")]
            pub button_watch => button::button_watch_generate {   // path is relative to crate root
                /// GPIO pin for the button.
                pin: ident,
                /// Debounce interval in milliseconds.
                #[default_display = "20"]
                debounce_ms: expr = $crate::DEFAULT_DEBOUNCE_MS,
            }

            generate {
                $(#[$attrs])*
                #[doc = $doc]
                $vis struct $name;
                // Ordinary macro_rules template tokens follow.
                const _: () = { let _ = stringify!($field_pin); };
            }
        }
    }

user
    button_watch! { pub Status { pin: PIN_13 } }
```

`define!` expands to a hidden `#[macro_export] macro_rules! __const_structures_wrapper_NAME`
with one catch-all rule, followed by `pub use __const_structures_wrapper_NAME as NAME;`.
That rule forwards `macro_name`, `generator: { $crate::GENERATOR }`, the schema body
tokens, and the user's tokens to `$crate::__const_structures_expand!`, which
validates, fills defaults, and calls the generator with every field present in
schema order, plus `attrs`, `vis` (empty becomes `pub(self)`, because `$vis:vis`
cannot match empty at the end of `[...]`), `name`, and `doc`.

For the optional template form, the final path segment of the generator path
names the generated backend alias. The hidden exported backend matcher and
bare-name alias are emitted at the schema location. Internal wrapper and backend
names use disjoint `__const_structures_wrapper_` and `__const_structures_backend_`
prefixes so distinct public names cannot collide between those roles. If the
schema module is private, re-export the backend through the public generator
path specified in the declaration. Other client macros can
call that normalized backend by its ordinary path, which lets a single schema
support related public macro entry points. Templates use native `macro_rules!`
metavariables and repetitions; the schema controls which bindings exist and
their repetition depth.

Template bindings are:

- Root metadata: `$attrs` (repeated `meta` captures), `$vis`, `$name`, `$doc`.
- Root leaf fields: `$field_FIELD`, with the schema field name substituted for
  `FIELD`.
- Nested leaf fields: `$field_BLOCK_FIELD`, extending the path through each
  nested block.
- When the schema has members: `$member_count`, plus member metadata
  `$member_attrs`, `$member_vis`, `$member_name`, `$member_doc`, and
  `$member_index`.
- Member leaf fields: `$member_field_FIELD`; nested leaves extend the path,
  such as `$member_field_BLOCK_FIELD`.

Optional fields and blocks are captured in `?` repetitions, so a template uses
the corresponding `$( ... )?` repetition to emit their contents. Members and
member attributes use `*` repetitions. A nested block is not captured as an
opaque token group: its schema leaf fields are destructured into path-based
bindings, with optional nesting reflected in the repetition nesting. If two
fields produce the same flattened binding, definition fails with a diagnostic
asking the schema author to rename a field.
An optional block without leaf fields has no capture that exposes its presence
to template repetitions; use an explicit backend when that presence matters.

The template is generated as a single normalized matcher. Use an explicit
generator macro when output requires bespoke matcher arms or when the
normalized form cannot express its emission pattern cleanly. This is a
generation choice, not a compatibility shim; both forms use the same schema
validation and normalization.

Because everything goes through `$crate`, a renamed dependency still works, and
`$crate::...` is allowed in schema defaults (shown in docs through
`#[default_display]`).

### Why the alias works

rustc rejects absolute-path access to a `#[macro_export]` macro that was itself
produced by macro expansion (`macro_expanded_macro_exports_accessed_by_absolute_paths`,
deny-by-default, slated to become a hard error). `pub use crate::__const_structures_wrapper_NAME`
triggers it. A bare `pub use __const_structures_wrapper_NAME as NAME;` in the same expansion
instead re-exports the macro from textual scope, which gives it an ordinary path;
other modules and crates then reach it through that alias (`crate::button::button_watch`,
a crate-root `pub use button::button_watch;`, downstream imports and full paths).
Never refer to the hidden wrapper by its crate-root path.

An earlier design generated one `#[proc_macro]` per schema, which forced each
library to keep its schemas in a separate proc-macro crate. The alias removes that
crate. `fixtures/alias-regression` and `fixtures/renamed-user` enforce
`#![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]` and
`#![deny(warnings)]`. They cover internal/module/root calls, calls from another
macro, downstream imports and full paths, further re-exports, and renamed
dependencies. The alias fixture also builds rustdoc and checks the module alias
page and generated field docs. Its `tests/ui/alias_via_crate_path.rs` control
must fail specifically when an alias accesses `crate::__const_structures_wrapper_widget`.

## Crate layout and diagnostics

The facade crate re-exports the proc macros. `const-structures-derive` is the
thin compiler-facing shim; `const-structures-core` implements parsing,
validation, normalization, documentation, and dispatch over `proc_macro2`.
Core errors use `syn::Error`, are combined where useful, and are emitted as
compile errors by the shim. Downstream, renamed-dependency, and alias-path
integration fixtures are separate workspace members. Caller field and member
spans survive normalization; unknown/duplicate fields, missing required fields,
wrong kinds, and member-count violations retain useful source locations. Trybuild UI cases live under
`fixtures/demo/tests/ui/` and cover rejected input with source diagnostics.

## Device Envoy migration corpus

The current client inventory, finalized choices, migration scale, and
remaining handwritten macros are documented in
[DE_MACRO_SURVEY.md](DE_MACRO_SURVEY.md).
