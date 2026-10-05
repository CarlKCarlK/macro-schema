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

## Pipeline

```text
TokenStream
  -> generic parser (visibility, name, optional type, value tree)
  -> value tree: Expr | Struct(named fields) | List | Map
  -> schema validation (required, defaults, unknown/duplicate, nesting)
  -> normalized IR
  -> code generation (ordinary Rust const/static items)
```

After parsing, nothing downstream cares about token order or spelling quirks.

## Open questions

- Where the schema lives: inferred from an ordinary Rust struct with
  attributes (preferred starting point), a separate schema DSL, or both.
- Field syntax: `name: value` vs `name = value`.
- How a schema defined in one crate is visible to a macro invocation in
  another (proc macros cannot see other items' definitions directly).
- Whether some Device Envoy macros generate tasks/resources, not just data,
  and how codegen hooks express that without becoming a second language.

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

First step: inventory every macro's grammar (fields, defaults, repetition,
what it generates) into a table, and flag inconsistencies to regularize.
