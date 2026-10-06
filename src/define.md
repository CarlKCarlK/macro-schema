Defines a declaration macro from a schema and a template.

This is the complete reference. For an introduction with a full program, see
the [crate documentation](crate), which starts with a quick start.

```text
macro_schema::define! {
    {ATTRIBUTE}
    VISIBILITY NAME {
        SCHEMA
    }

    generate {
        TEMPLATE
    }
}
```

`define!` creates a new macro, `NAME!`. Each call of `NAME!` is checked
against the schema. Defaults are filled in, and the template is rendered into
Rust items. `define!` itself checks the template against the schema, so a
mistake in the template is reported when your library compiles.

# Setup

A crate that uses `define!` must re-export [`expand!`](macro@crate::expand) at
its crate root, under exactly this name:

```text
#[doc(hidden)]
pub use macro_schema::expand as __macro_schema_expand;
```

Every generated macro calls `$crate::__macro_schema_expand!`, so the name
and location are fixed.

# The defined macro

The new macro lives where `define!` is written, like an item. Its visibility is
the `define!`'s visibility. Call it by path, or re-export it, as with any
other item:

```rust
# #[doc(hidden)]
# pub use macro_schema::expand as __macro_schema_expand;
pub mod flags {
    macro_schema::define! {
        /// Declares a flag type.
        pub flag {
            /// Whether the flag starts on.
            on: expr = false,
        }

        generate {
            $decl.attrs
            #[doc = $decl.doc]
            $decl.vis struct $decl.name;

            impl $decl.name {
                pub const ON: bool = $decl.on;
            }
        }
    }
}

// The macro is an item of `flags`; re-export it like one.
pub use flags::flag;

flags::flag! { pub Logging { on: true } }
flag! { pub Tracing {} }

fn main() {
    assert!(Logging::ON);
    assert!(!Tracing::ON);
}
```

The doc comment on `define!` becomes the macro's documentation, followed by a
syntax summary and field table generated from the schema. Other attributes,
such as `#[cfg(...)]`, apply to the defined macro.

# Schema

A schema is a comma-separated list of fields and, optionally, one members
section.

## Fields and kinds

```text
/// Doc comment for the field.
name: KIND
```

The kind says how the caller's value is parsed:

| Kind | Accepts | Typical use |
| --- | --- | --- |
| `ident` | One identifier | Names pasted into paths or new identifiers: `PIN_3`, `Small` |
| `expr` | Any Rust expression | Values: `8080`, `"server.port"`, `Current::Milliamps(250)`, `{ 2 + 1 }` |
| `ty` | Any Rust type | Types: `u16`, `Vec<String>`, `&'static str` |

Because each value is parsed with the parser for its kind, commas inside a
generic type or an expression stay part of that value.

## Required, defaulted, and optional fields

| Schema | Meaning |
| --- | --- |
| `port: expr` | Required |
| `port: expr = 8080` | Defaulted: the default is used when the caller omits it |
| `port?: expr` | Optional: may be absent; read it with [`$if let`](#if-let) |

A field cannot be both optional and defaulted. A default can use `$crate` to
name items in your crate: `gamma: expr = $crate::Gamma::Srgb`.

## Showing a default: `#[default_display]`

Defaults often have long paths. `#[default_display = "..."]` changes how a
single default value is shown in the generated docs. The template still gets
the real default:

```text
/// Color curve.
#[default_display = "Gamma::Srgb"]
gamma: expr = $crate::led::Gamma::Srgb,
```

## Nested blocks

A field can hold a block of fields, which the caller writes in braces. A block
can be optional:

```text
/// Accepted range.
range?: {
    /// Lower bound.
    min: expr,
    /// Upper bound.
    max: expr = 100,
},
```

The caller writes `range: { min: 1 }`. Blocks contain fields, never members.

## Members

A members section lets the caller declare repeated, named items inside the
declaration:

```text
/// Each member is one channel.
members 1..=4 {
    input: ident,
    /// Defaults by position: the first member gets `CH0`, the second `CH1`, ...
    channel: ident = by_index[CH0, CH1, CH2, CH3],
}
```

- `members MIN..=MAX` bounds the count. `members MIN..` has no upper limit.
- Members are allowed only at the top level, at most one section per schema.
  A field named `members` is written `members: KIND`.
- `by_index[A, B, ...]` is a default for member fields, including fields in a
  member's nested blocks. The member at index `i` gets the `i`th value. A
  member past the end of the list must give the field itself.
- Members take the declaration's visibility. Writing a visibility on a member
  is an error.

## Other rules

- Field attributes may be only doc comments and `#[default_display]`.
- Field names must be unique within a block.
- A top-level field may not be named `name`, `vis`, `doc`, `attrs`, or
  `members`, and a member field may not be named `name`, `vis`, `doc`, `attrs`,
  or `index`, because templates use those names for
  [built-in values](#values).

# Invocation syntax

A caller writes one declaration:

```text
NAME! {
    {ATTRIBUTE} [VISIBILITY] Name {
        field: value,
        block: { field: value, ... },
        {ATTRIBUTE} Member { field: value, ... },
        ...
    }
}
```

- Fields and members may come in any order, mixed. A trailing comma is
  optional.
- A declaration or member is always `Name { ... }`; writing `Name: { ... }` is
  an error that says to remove the `:`.
- Doc comments and other attributes are allowed on the declaration and on each
  member.

# Template

A template is Rust code plus four constructs, each starting with `$`:

| Construct | Meaning |
| --- | --- |
| `$decl.VALUE`, `$var.VALUE` | Insert a value |
| `$for var in $decl.members { ... }` | Repeat once per member |
| `$if let Some(var) = OPTIONAL { ... } else { ... }` | Branch on an optional value; `else` may be omitted |
| `$ident(...)`, `$snake(...)`, `$upper(...)` | Build an identifier |

`$crate` passes through. Any other `$` is an error.

## Values

Everything that belongs to the declaration is reached through `$decl`.
Everything that belongs to a loop or `$if let` variable is reached through that
variable.

| Path | Value |
| --- | --- |
| `$decl.name` | The declared identifier |
| `$decl.vis` | Its visibility; `pub(self)` when none was written |
| `$decl.attrs` | Its attributes, as written, including doc comments |
| `$decl.doc` | Its generated documentation, as a string literal; see [Documentation](#documentation) |
| `$decl.members` | The members; usable only after `in` in `$for` |
| `$decl.FIELD`, `$decl.BLOCK.FIELD` | Schema fields |
| `$m.name`, `$m.attrs`, `$m.doc` | The same for a member `$m` |
| `$m.vis` | The declaration's visibility |
| `$m.index` | The member's position, from 0, as an integer literal |
| `$m.FIELD`, `$m.BLOCK.FIELD` | The member's fields |
| `$x`, `$x.FIELD` | An `$if let` variable: the optional value, or the optional block's fields |

A path continues through `.field` only while the current value has fields.
After a plain value, `.` is ordinary Rust: if `layout` is an `expr` field,
`$decl.layout.width()` inserts `layout` and keeps `.width()`.

## Expressions keep their precedence

An `expr` value that is a compound expression is inserted in parentheses, so
it means the same thing wherever the template puts it:

| Value of `x` | `$decl.x * 2` becomes | `$decl.x.pow(2)` becomes |
| --- | --- | --- |
| `1 + 2` | `(1 + 2) * 2` | `(1 + 2).pow(2)` |
| `-4` | `(-4) * 2` | `(-4).pow(2)` |
| `7` | `7 * 2` | `7.pow(2)` |
| `LIMIT` | `LIMIT * 2` | `LIMIT.pow(2)` |

Expressions that are already a single operand are inserted as written:
literals, paths, calls, method calls, field accesses, indexing, `?`, `.await`,
blocks, macro calls, and parenthesized, tuple, array, and struct expressions.
So `concat!($decl.label)` and `include_bytes!($decl.path)` keep working, and a
literal can be a bare const generic argument (`Holder::<$decl.len>`). Two
consequences: `stringify!` shows the parentheses of a compound value, and a
compound value used as a const generic argument needs braces in the template
(`Holder::<{ $decl.n }>`), as it would with `macro_rules!`. `ident` and `ty`
values are never wrapped.

## `$for`

`$for var in $decl.members { ... }` repeats its body once per member, in the
order the caller wrote them. It can appear anywhere tokens can, including in a
parameter list, a match, or an expression:

```rust
# #[doc(hidden)]
# pub use macro_schema::expand as __macro_schema_expand;
macro_schema::define! {
    /// Declares a group of channels.
    pub channels {
        members 1..=4 {
            /// Hardware channel; defaults by position.
            channel: ident = by_index[CH0, CH1, CH2, CH3],
        }
    }

    generate {
        $decl.vis struct $decl.name;

        impl $decl.name {
            pub const COUNT: usize = 0 $for member in $decl.members { + 1 };
            pub const NAMES: [&'static str; Self::COUNT] =
                [$for member in $decl.members { stringify!($member.name), }];
        }

        $for member in $decl.members {
            $member.vis struct $member.name;

            impl $member.name {
                pub const INDEX: usize = $member.index;
                pub const CHANNEL: &'static str = stringify!($member.channel);
            }
        }
    }
}

channels! {
    pub Inputs {
        Left {},
        Right { channel: CH3 },
    }
}

fn main() {
    assert_eq!(Inputs::NAMES, ["Left", "Right"]);
    assert_eq!((Left::INDEX, Left::CHANNEL), (0, "CH0"));
    assert_eq!((Right::INDEX, Right::CHANNEL), (1, "CH3"));
}
```

Members exist only at the top level, so `$decl.members` is the only thing a
`$for` can loop over.

## `$if let`

`$if let Some(var) = PATH { THEN } else { OTHERWISE }` requires `PATH` to be an
optional field or block. If the caller gave it, `THEN` is rendered with `$var`
bound to it. If not, `OTHERWISE` is rendered, or nothing when there is no
`else`. `$var` exists only in `THEN`. Reading an optional value any other way
is an error.

```rust
# #[doc(hidden)]
# pub use macro_schema::expand as __macro_schema_expand;
macro_schema::define! {
    /// Declares a bounded value.
    pub bounded {
        /// Accepted range; unbounded when omitted.
        range?: {
            min: expr,
            max: expr = i32::MAX,
        },
    }

    generate {
        $decl.vis struct $decl.name;

        impl $decl.name {
            pub fn contains(value: i32) -> bool {
                $if let Some(range) = $decl.range {
                    ($range.min..=$range.max).contains(&value)
                } else {
                    let _ = value;
                    true
                }
            }
        }
    }
}

bounded! { pub Percent { range: { min: 0, max: 100 } } }
bounded! { pub NonNegative { range: { min: 0 } } }
bounded! { pub Anything {} }

fn main() {
    assert!(Percent::contains(50) && !Percent::contains(101));
    assert!(NonNegative::contains(1_000) && !NonNegative::contains(-1));
    assert!(Anything::contains(-1));
}
```

## Identifiers

`$ident(PART, ...)` joins its parts into one identifier. `$snake(...)` and
`$upper(...)` join the parts, then convert the result to `snake_case` or
`SCREAMING_SNAKE_CASE`. Each part is one of:

- a plain identifier or an integer, such as `_COUNT` or `2`;
- an identifier value: `$decl.name`, `$m.name`, `$m.index`, an `ident` field,
  or an `$if let` variable bound to an optional `ident` field.

The new identifier takes the span of its first inserted value, so compiler
errors about it point at the caller's name.

Case conversion inserts `_` before a capital letter that follows a lowercase
letter, before a capital that follows digits that follow a lowercase letter,
and before the last capital of an acronym when a lowercase letter follows. It
never doubles an underscore. Then it changes case. These conversions are
checked by the crate's tests:

| Joined text | `$snake` | `$upper` |
| --- | --- | --- |
| `Gpio0LedStrip_pin` | `gpio0_led_strip_pin` | `GPIO0_LED_STRIP_PIN` |
| `HTTPServer` | `http_server` | `HTTP_SERVER` |
| `Ir15Receiver` | `ir15_receiver` | `IR15_RECEIVER` |
| `Font4x6Trim` | `font4x6_trim` | `FONT4X6_TRIM` |
| `PIO0_BUS` | `pio0_bus` | `PIO0_BUS` |
| `LED2D_OR_LED_STRIPS` | `led2d_or_led_strips` | `LED2D_OR_LED_STRIPS` |
| `__Inner` | `__inner` | `__INNER` |

```rust
# #[doc(hidden)]
# pub use macro_schema::expand as __macro_schema_expand;
pub mod counting {
    pub use std::sync::atomic::{AtomicU32, Ordering};

    macro_schema::define! {
        /// Declares a named event counter.
        pub counter {
            /// How much each event adds.
            step: expr = 1,
        }

        generate {
            static $upper($decl.name, _COUNT): $crate::counting::AtomicU32 =
                $crate::counting::AtomicU32::new(0);

            $decl.vis fn $snake(record_, $decl.name)() -> u32 {
                $upper($decl.name, _COUNT).fetch_add($decl.step, $crate::counting::Ordering::Relaxed)
                    + $decl.step
            }
        }
    }
}

counting::counter! { pub PageView { step: 1 + 1 } }

fn main() {
    // `$upper(PageView, _COUNT)` is `PAGE_VIEW_COUNT`, and
    // `$snake(record_, PageView)` is `record_page_view`.
    assert_eq!(record_page_view(), 2);
    assert_eq!(record_page_view(), 4);
    assert_eq!(PAGE_VIEW_COUNT.load(counting::Ordering::Relaxed), 4);
}
```

## `$crate`

`$crate` in a template, or in a schema default, means the crate that contains
the `define!`, exactly as in `macro_rules!`. The counter example names
`$crate::counting::AtomicU32` so its output compiles in any crate, including
one that renamed your crate in its `Cargo.toml`.

## When a template is checked

`define!` checks the template against the schema when your crate compiles.
These are all errors at the template: an unknown value, a misspelled field,
reading an optional value without `$if let`, looping over anything other than
`$decl.members`, a non-identifier as an identifier part, and malformed
constructs. See [Errors](#errors).

# Documentation

**Macro docs.** The defined macro's page shows the `define!`'s doc comment,
then a generated `**Syntax:**` block, a `**Fields:**` table, and, when there
are members, a `**Member fields**` table. The tables show each field's kind,
whether it is required or its default, and its doc comment.

**Item docs.** `$decl.doc` and `$m.doc` hold generated documentation for the
declaration or member: which macro generated it, and a table of the values it
used, with defaults marked `(default)` and absent optional values shown as
`(not set)`. A declaration with members also lists them. The usual pattern is:

```text
$decl.attrs
#[doc = $decl.doc]
$decl.vis struct $decl.name;
```

**Written docs replace generated docs.** If the caller writes doc text on a
declaration or member, as `///` comments or `#[doc = "..."]`, it arrives
through `attrs`, and that item's `doc` is empty. So the caller's text replaces
the generated description rather than being added to it. `#[doc(hidden)]` and
other `doc(...)` attributes are not doc text and don't suppress it.

A macro can use this rule to forward to another macro: passing
`$decl.attrs #[doc = $decl.doc]` on a member gives that member the outer
macro's description instead of a generated one.

# Errors

Every error points at the tokens that caused it. When a call has several
problems with its fields or members, they are reported together.

## Schema errors, when `define!` compiles

| Mistake | Message |
| --- | --- |
| Unknown kind | ``unknown field kind; expected `ident`, `expr`, or `ty` `` |
| `?` together with `= default` | ``an optional (`?`) field cannot also have a default`` |
| `default_display` without a single default | `` `default_display` needs a single default value (`= ...`) `` |
| `by_index` outside member fields | `` `by_index` defaults are allowed only in member fields `` |
| Members inside a block or member | `members are allowed only at the top level of a schema` |
| A second members section | `duplicate members section` |
| A repeated field | ``duplicate schema field `x` `` |
| `members 3..=1` | `member range is empty` |
| Another attribute on a field | ``only doc comments and `#[default_display = "..."]` are allowed on schema fields`` |
| A field named like a built-in | ``field `name` collides with the template's built-in `$decl.name`; rename the field`` |
| No `generate` block | ``expected `generate { ... }` after the schema`` |
| Both `generate` and `=> generator` | ``a `generate { ... }` template replaces `=> generator`; use one or the other`` |
| `=> ::path` | ``the generator path is relative to this crate's root; drop the leading `::` `` |

## Template errors, when `define!` compiles

| Mistake | Message |
| --- | --- |
| `$strip.pnael` | ``no value `pnael` here; expected one of `name`, `vis`, `doc`, `attrs`, `index`, `pin`, `dma`, `panel` `` |
| `$bus`, for a declaration field | ``unknown template value `$bus`; write `$decl.bus` `` |
| `$strp.pin`, with no such variable | ``unknown template value `$strp`; expected `$decl`, `$strip` `` |
| `$strip.panel`, which is optional | `` `$strip.panel` is optional; read it with `$if let Some(x) = ... { ... }` `` |
| `$if let ... = $decl.bus`, which is required | `` `$decl.bus` is not optional; `$if let` needs an optional field `` |
| `$for s in $decl.bus` | `` `$decl.bus` is not the member list; loop over `$decl.members` `` |
| `$decl.members` as a value | `` `$decl.members` is the member list; loop over it with `$for member in $decl.members { ... }` `` |
| `$decl` alone | `` `$decl` is the declaration, not a value; name one of ... `` |
| A block as a value | `` `$x.limits` is a block, not a value; name one of its fields `` |
| Not an identifier in `$ident(...)` | `` `$decl.doc` is not an identifier; identifier parts must be `ident` fields, `$decl.name`, `$member.name`, or `$member.index` `` |
| A stray `$` | `` `$` must start a template construct (`$value`, `$for`, `$if`, `$ident(...)`) or `$crate` `` |
| `$for decl in ...` | `` `decl` is reserved in templates; choose another variable name `` |
| Reusing a variable name | `` `$strip` is already in scope; choose another variable name `` |

## Invocation errors, when the macro is called

| Mistake | Message |
| --- | --- |
| Unknown field | ``unknown field `x`; expected one of `a`, `b` (or a member `Name { ... }`)`` |
| Repeated field | ``duplicate field `x` `` |
| Missing required field | ``missing required field `x` `` |
| Wrong kind | ``field `x` expects an identifier`` (or `an expression`, `a type`) |
| A value where a block belongs | ``field `x` expects `{ ... }` `` |
| A block where a value belongs | `expected a value, not a block` |
| Too many or too few members | `` `channels!` takes 1 to 4 members; found 5 `` |
| Repeated member | ``duplicate member `X` `` |
| Visibility on a member | `members take their group's visibility; remove this visibility` |
| `Name: { ... }` | ``remove the `:` after `Name`; declarations are written `Name { ... }` `` |
| A member where the schema has none | `` `Name` takes fields only; expected one of ... `` |
| A member past its `by_index` list | ``field `x` has no default for member index 4; give it explicitly`` |
| Extra tokens after the declaration | `unexpected tokens after the declaration body` |

Values keep the caller's spans when they're inserted, so a type error in the
generated code points at the caller's value.

# Escape hatch: a `macro_rules!` generator

When a template can't express the output, write `=> GENERATOR` instead of a
`generate` block. `define!` then sends each validated call to your own
`macro_rules!` macro, at a path relative to your crate root:

```rust
# #[doc(hidden)]
# pub use macro_schema::expand as __macro_schema_expand;
#[doc(hidden)]
#[macro_export]
macro_rules! __flag_generate {
    (
        attrs: [$(#[$attr:meta])*],
        vis: [$vis:vis],
        name: $name:ident,
        doc: $doc:literal,
        on: $on:expr,
        label: [$($label:expr)?],
    ) => {
        $(#[$attr])*
        #[doc = $doc]
        $vis struct $name;

        impl $name {
            pub const ON: bool = $on;
            pub const LABEL: Option<&'static str> = {
                let label: Option<&'static str> = None;
                $(let label = Some($label);)?
                label
            };
        }
    };
}

macro_schema::define! {
    /// Declares a flag type.
    pub flag => __flag_generate {
        /// Whether the flag starts on.
        on: expr = false,
        /// Optional display label.
        label?: expr,
    }
}

flag! { pub Logging { on: true, label: "logging" } }

fn main() {
    assert!(Logging::ON);
    assert_eq!(Logging::LABEL, Some("logging"));
}
```

The generator receives every field in schema order:

```text
attrs: [ATTRIBUTES], vis: [VISIBILITY], name: NAME, doc: "...",
FIELD: VALUE, ...
member_count: N,                                    // only with members
members: [{ index: I, attrs: [...], vis: [...], name: NAME, doc: "...", FIELD: VALUE, ... }, ...],
```

- An unwritten visibility arrives as `pub(self)`, so `$vis:vis` can match it.
- An optional field arrives as `[]` when absent and `[VALUE]` when present.
- A block arrives as `{ FIELD: VALUE, ... }`, and an optional block as
  `[{ ... }]` or `[]`.

A generator matches `$x:expr`, so values keep their precedence there too.
