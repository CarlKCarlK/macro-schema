# const-structures Spec

<!-- TODO0 consider deleting this spec once the work below is implemented and released. -->

## Goal

A general proc-macro system for ergonomic, named, compile-time declarations of
constant structures: named fields, defaults, required and optional fields,
nesting, repetition/collections, visibility, validation, and generation of
strongly typed `const`/`static` Rust.

Existence proofs: Python's keyword-construction ergonomics, and Device
Envoy's current `macro_rules!` macros. Neither is a spec. The name and API say
nothing about Python.

## Success criterion

After converting all of Device Envoy, the proc-macro implementation is easier
to understand than the current declarative macros as a whole, not merely
prettier in isolated examples. If covering everything needs a forest of
exceptions, either this framework is wrong or Device Envoy needs more
regularization.

## Hard rules

1. One regular syntax for every declaration.
2. Keyword-like fields everywhere; any order.
3. Optional fields and defaults expressed the same way everywhere.
4. Rust visibility (`pub`, `pub(crate)`, ...) handled uniformly, in one place.
5. Repeated sub-items use one standard form.
6. Unknown, duplicate, missing, or incompatible fields produce good, spanned
   compile errors.
7. Per-structure behavior is mostly data: allowed fields, defaults,
   constraints, and code-generation hooks.
8. If a Device Envoy macro cannot fit cleanly, change Device Envoy's syntax
   rather than add a special case.

## Documentation comes from the schema

Each field is described once (name, kind, required/default, doc string). From
that one description come parsing, default insertion, error messages,
generated code, and rustdoc. Success test: no field is described in more than
one place.

- Generated items get `#[doc]` listing the configuration actually used,
  marking defaulted fields, with values formatted via `prettyplease` (raw
  `TokenStream::to_string()` spacing is unreadable).
- Generated docs use ```` ```text ```` fences, never ```` ```rust ````: they
  land in the user's crate and would run as its doctests.
- The macro's own doc page is static, so keep its field table in sync with a
  test (or a `build.rs` that renders it), not by hand.
- Device Envoy's `*_generated` doc modules become real macro invocations.

## Decisions (2026-10-04)

From the [Device Envoy survey](DE_MACRO_SURVEY.md):

1. Shared group fields go inside the group's braces, next to its members.
2. Members are written `Name { ... }`, exactly like a top-level declaration.
3. Every declaration and member accepts any field order, an optional
   trailing comma, `#[attrs]`, and visibility.
4. No field aliases. One spelling per field.
5. `servo!` becomes a named declaration on both rp and esp.
6. The framework is indifferent to whether different macros agree with each
   other. It enforces syntax; each schema owns its field meanings and
   defaults. Making Device Envoy's macros agree semantically (e.g.
   `max_frames` vs `max_steps`) is Device Envoy's own, separate decision.
7. Out of scope: positional expression macros (`tone!`, `combine!`, `tga!`,
   `pio_split!`) and `init_and_start!`. They stay `macro_rules!`.
8. Field syntax is `name: value`.

## Target grammar

```ebnf
Invocation  = Declaration ;
Declaration = { ATTR } VIS NAME Body ;
Body        = "{" [ Item { "," Item } [ "," ] ] "}" ;
Item        = Field | Declaration ;
Field       = IDENT ":" Value ;
Value       = Body | KindValue ;
```

- `Field` vs member `Declaration` is decided with two tokens of lookahead:
  `IDENT ":"` is a field; `#`, a visibility keyword, or `IDENT "{"` starts a
  member.
- A value starting with `{` is a nested field list (e.g. `led2d: { ... }`).
  A Rust block expression as a value must be parenthesized.
- `KindValue` is parsed according to the field's kind in the schema
  (`Expr`, `Type`, `Ident`, or one of a fixed set of idents). Values are not
  split at top-level commas first, because commas also appear inside
  `Foo<A, B>` and `f::<A, B>()`, where `<>` are not token groups.
- Whether a declaration may have members, which kinds, and how many is
  schema data.

## Examples (Device Envoy, regularized)

```rust,ignore
led_strips! {
    pub LedStrips0 {
        pio: PIO0,
        Gpio0LedStrip { pin: PIN_0, len: 8, max_current: Current::Milliamps(25) },
        pub Gpio4Led2d {
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
        pub Top { width: 16, height: 2, address: 0x27 },
    }
}

servo! { pub Pan { pin: PIN_0, slice: PWM_SLICE0, channel: A } }
```

## Pipeline

```text
TokenStream
  -> generic parser: attrs, visibility, name, items (fields + members),
     with each value parsed by the kind its schema declares
  -> schema validation: unknown, duplicate, missing, member count,
     cross-field constraints; defaults inserted
  -> normalized spec: typed per macro, no token-order or spelling quirks
  -> code generation hook (per macro): ordinary Rust items + rustdoc
```

Most Device Envoy macros generate structs, statics, Embassy tasks, and
methods, not only constant data, so code generation is a per-macro hook that
receives a validated spec. The framework owns everything before that hook.

## Open questions

- Where the schema lives: inferred from an ordinary Rust struct with
  attributes, a separate schema DSL, or plain data in the proc-macro crate.
  Device Envoy's generated code depends on chip HAL types the schema crate
  cannot see, which favors schemas as data in each proc-macro crate.
- How a schema defined in one crate is visible to a macro invocation in
  another (proc macros cannot see other items' definitions directly).
- How per-member defaults that depend on member index (rp `led_strips!`
  DMA auto-numbering) are expressed as schema data rather than code.

## Project layout and practice

Follows "Nine Rules for Creating Procedural Macros in Rust" (Kadie), with
dependencies updated:

- `const-structures`: facade crate; re-exports the macro; integration tests and
  `trybuild` UI tests in `tests/`.
- `const-structures-derive`: thin `proc-macro = true` shim.
- `const-structures-core`: all logic on `proc_macro2`; unit tests compare
  `prettyplease` output with `pretty_assertions` diffs, debuggable normally.
- `syn` 3. Errors are `syn::Error` returned through `?` and merged with
  `Error::combine`, emitted once via `into_compile_error()` in the shim. This
  replaces the article's `proc-macro-error` (unmaintained; its successor
  `proc-macro-error2` still pins `syn` 2) and avoids panic-based `abort!`.

## Rollback plan

- This repo is standalone; deleting it removes the experiment entirely.
- Device Envoy work happens only on branch `proc-macro-const-structures`
  (from `main` at `4c1ee16f`), using a path dependency on this repo. Rolling
  back means deleting that branch; `main` is never touched.
- Convert Device Envoy one macro per commit so partial rollback is a revert.

## Device Envoy corpus

Files containing `macro_rules!` at the start of the experiment:

- core: `audio_player`, `cyd/display/tga`, `wifi_auto/fields`
- esp: `lib`, `init_and_start`, `audio_player`, `button/button_watch`, `ir`,
  `ir/kepler`, `ir/mapping`, `lcd_text`, `led`, `led2d`, `led_strip`,
  `led_strip/spi`, `servo`, `servo_player`
- rp: `lib`, `audio_player`, `button/button_watch`, `ir`, `ir/kepler`,
  `ir/mapping`, `lcd_text`, `led`, `led2d`, `led_strip`, `pio_irqs`, `servo`,
  `servo_player`, `wifi_auto/stack`

Inventory, grammars, and inconsistencies: [DE_MACRO_SURVEY.md](DE_MACRO_SURVEY.md).
