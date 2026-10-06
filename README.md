# macro-schema

`macro-schema` helps Rust library authors write *declaration macros*. A
declaration macro lets the library's users write a named declaration with
keyword fields, like this:

```text
setting! {
    pub Port { key: "server.port", value: u16, default: 8080 }
}
```

and turns it into new Rust items: here, a type `Port` that implements the
library's `Setting` trait.

The author defines such a macro in two parts, which give the crate its name:

- A **schema** defines the macro's syntax: its fields, the kind of each
  (`ident`, `expr`, or `ty`), which are required, and their defaults.
- A **template** defines the macro's output: the Rust code that each
  declaration becomes.

From those two parts, `macro-schema` builds the macro: parsing, validation,
defaults, error messages that point at the caller's tokens, and generated
documentation. The library exports the new macro like any other, and its users
call it without depending on `macro-schema` themselves.

## Why generate items?

Most library APIs take values. Users fill in a struct, chain a builder, or pass
arguments, and the library's already-compiled code handles every user the same
way. Some libraries need more: each thing a user declares must become items of
its own, such as:

- a **type**, so that each declaration can have its own trait implementations,
  associated constants, and an identity the compiler checks;
- a **static**, for state with a fixed address that lives for the whole program;
- **methods and functions** named after what the user declared;
- a **task** or handler that a framework requires to be a concrete, non-generic
  function.

Values can't create items. A `const` struct can describe a configuration, and a
builder can assemble one, but neither can declare a type or a static. Generics
help only where the surrounding framework accepts generic items. When a library
has to add items to its user's crate, it needs a macro.

## The problem

`macro_rules!` can generate items, but a declaration with named fields asks a
lot of it. Writing `setting!` by hand with `macro_rules!` means handling:

- named fields given in any order;
- required fields, optional fields, and defaults;
- unknown, duplicate, and missing fields, each with a useful error;
- nested blocks, such as `limits: { min: 0, max: 10 }`;
- repeated named members, such as several commands in one declaration;
- attributes and visibility on the declaration;
- documentation for the macro and for each item it generates.

Done with `macro_rules!`, this machinery is usually built from recursive
"tt-muncher" rules. It is often far larger, and much harder to check, than the
code the macro finally emits. In [Device Envoy](#origin-device-envoy), hand-written
declaration macros came to over 14,000 lines. A hand-written procedural macro
avoids the tt-munching, but needs its own crate, its own parser, and its own
error reporting and documentation.

With `macro-schema`, you write the schema and the template, and that
machinery comes from the crate. Both live in your library, next to the API they
generate; you don't need a separate procedural-macro crate.

## When to use it

- For ordinary data, use Rust structs and constants:
  `const CFG: Config = Config { port: 8080, ..Config::DEFAULT }`.
- For named function arguments, consider a builder.
- For simple token substitution, use `macro_rules!`.
- For arbitrary transformations of Rust syntax, write a procedural macro.
- For declarations that need named fields, defaults, nesting, or repeated
  members, and that must *generate items* (types, impls, statics, tasks),
  consider `macro-schema`.

What it deliberately does not do:

- It doesn't guess what a declaration means. Every macro has a template, because
  a schema describes syntax (an identifier, an expression, a type), not Rust
  types or behavior.
- Its template language has no arithmetic, string operations, or branching on a
  value's contents. Computation belongs in ordinary Rust that the template
  calls.
- It doesn't type-check values. A value of kind `expr` is any expression; if it
  has the wrong type, the compiler reports that in the generated code, at the
  user's token.

## Quick start

There are two roles:

- The **library author** defines a declaration macro with
  `macro_schema::define!`.
- The **library user** invokes that macro and gets ordinary Rust items.

Only the library depends on `macro-schema`:

```toml
[dependencies]
macro-schema = "0.1"
```

The library's users depend on the library alone; the macros it defines don't
need `macro-schema` in the user's `Cargo.toml`.

Here both roles are in one file. The author defines `setting!` in the module
`settings`; the user declares two settings and reads them from configuration
text. In the template, `$decl.name`, `$decl.key`, and the other `$decl` values
are filled in from each declaration; [the template language](#the-template-language)
explains them. This is [`examples/quick_start.rs`](https://github.com/CarlKCarlK/macro-schema/blob/main/examples/quick_start.rs) in the repository:

```rust
// examples/quick_start.rs: run with `cargo run --example quick_start`.

// ----- Library author: defines the `setting!` declaration macro -----

// Every crate that defines macros with `macro-schema` re-exports this once,
// at its crate root.
#[doc(hidden)]
pub use macro_schema::expand as __macro_schema_expand;

pub mod settings {
    /// A named configuration setting with a typed value.
    pub trait Setting {
        /// The setting's value type.
        type Value: core::str::FromStr;
        /// The key that identifies the setting in configuration text.
        const KEY: &'static str;
        /// The value used when the configuration doesn't mention the setting.
        fn default_value() -> Self::Value;

        /// Reads the setting from `key=value` lines, falling back to its default.
        fn read(config: &str) -> Self::Value {
            config
                .lines()
                .filter_map(|line| line.split_once('='))
                .find(|(key, _)| key.trim() == Self::KEY)
                .and_then(|(_, value)| value.trim().parse().ok())
                .unwrap_or_else(Self::default_value)
        }
    }

    macro_schema::define! {
        /// Declares a configuration setting: a type that implements [`Setting`].
        pub setting {
            /// Key that identifies the setting, such as `"server.port"`.
            key: expr,
            /// Type of the setting's value.
            value: ty,
            /// Value used when the setting is absent.
            #[default_display = "Default::default()"]
            default: expr = ::core::default::Default::default(),
        }

        generate {
            $decl.attrs
            #[doc = $decl.doc]
            $decl.vis struct $decl.name;

            impl $crate::settings::Setting for $decl.name {
                type Value = $decl.value;
                const KEY: &'static str = $decl.key;
                fn default_value() -> Self::Value {
                    $decl.default
                }
            }
        }
    }
}

// ----- Library user: declares settings and uses the generated types -----

use settings::{Setting, setting};

setting! {
    /// TCP port the server listens on.
    pub Port { key: "server.port", value: u16, default: 8080 }
}

setting! {
    pub Verbose { key: "log.verbose", value: bool }
}

fn main() {
    let config = "server.port = 3000\n";

    let port = Port::read(config);
    let verbose = Verbose::read(config);
    println!("{} = {port}", Port::KEY);
    println!("{} = {verbose}", Verbose::KEY);

    assert_eq!(port, 3000);
    assert!(!verbose); // not in `config`, so `bool::default()`
}
```

Running it prints:

```text
server.port = 3000
log.verbose = false
```

The pieces fit together like this:

```text
schema + template   (library author, inside define!)
        ↓
setting!            (a new macro, exported like any other)
        ↓
setting! { pub Port { ... } }   (library user)
        ↓
pub struct Port;  impl Setting for Port { ... }   (ordinary Rust items)
```

For `Port`, the generated code is equivalent to:

```text
/// TCP port the server listens on.
pub struct Port;

impl crate::settings::Setting for Port {
    type Value = u16;
    const KEY: &'static str = "server.port";
    fn default_value() -> Self::Value {
        8080
    }
}
```

`Verbose` didn't give `default`, so it gets the schema's default,
`Default::default()`.

## Schema and template

A declaration macro has two halves, and `define!` takes both.

**The schema describes what input is legal.**

```text
key: expr,
value: ty,
default: expr = ::core::default::Default::default(),
```

Each field has a name and a *kind*: `ident`, `expr`, or `ty`. The kind says
how to parse the caller's value. A field without `= ...` is required; one with
it has a default. A field written `name?: kind` is optional and has no default.
The `///` comment on each field becomes part of the macro's documentation.

**The template describes what the input means.**

```text
$decl.vis struct $decl.name;

impl $crate::settings::Setting for $decl.name {
    type Value = $decl.value;
    ...
}
```

A template is ordinary Rust plus a few `$` constructs, described next.

Both halves are necessary. A schema says that `value` is a type, but not
whether it becomes an associated type, a field, or a generic argument. So
`generate` is always required. This was checked against all 33 schemas in
Device Envoy: none of their output could be derived from the schema alone.

## The template language

A template has exactly four constructs. Everything else in it is copied
through as Rust.

### Values

`$decl` is the declaration being expanded. Its built-in values are:

| Value | Meaning |
| --- | --- |
| `$decl.name` | The declared name, such as `Port` |
| `$decl.vis` | Its visibility, such as `pub`; `pub(self)` when none was written |
| `$decl.attrs` | The attributes the caller wrote on it, including doc comments |
| `$decl.doc` | Generated documentation for it; see [Generated documentation](#generated-documentation) |

Schema fields are reached the same way: `$decl.key`, `$decl.value`. A field
inside a nested block is `$decl.limits.min`.

Everything that belongs to the declaration is reached through `$decl`. Values
of loop and `$if let` variables, introduced below, are reached through the
variable.

### Repetition: `$for`

A schema can allow repeated, named *members* with `members MIN..=MAX { ... }`
(or `members MIN.. { ... }` for no upper limit). The caller writes each member
as `Name { fields }` inside the declaration. `$for` repeats template text once
per member. This snippet, like the next two, comes from the
[larger example](#a-larger-example):

```text
$decl.vis enum $decl.name {
    $for command in $decl.members {
        $command.attrs
        #[doc = $command.doc]
        $command.name,
    }
}
```

Inside the loop, `$command.name`, `$command.attrs`, and `$command.doc` work as
they do on `$decl`. `$command.index` is the member's position, starting at 0,
and `$command.vis` is the declaration's visibility: members always share it.
The member's schema fields are `$command.summary`, `$command.args.min`, and
so on.

### Optional values: `$if let`

An optional field or block (`name?: ...`) may be absent, so a template must say
what to do in both cases:

```text
$if let Some(args) = $command.args {
    ($args.min..=$args.max).contains(&count)
} else {
    count == 0
}
```

Reading an optional value any other way is an error when the macro is defined.
This catches a missing case before any caller can hit it.

### Identifiers: `$ident`, `$snake`, `$upper`

Generated items often need new names built from the caller's names.
`$ident(...)` joins its parts into one identifier. `$snake(...)` and
`$upper(...)` join them, then convert the result to `snake_case` or
`SCREAMING_SNAKE_CASE`. Parts are plain identifiers, integers, the
`name` or `index` of a declaration or member, and `ident` fields.

| Template | Caller's names | Result |
| --- | --- | --- |
| `$ident($decl.name, ParseError)` | `FileTool` | `FileToolParseError` |
| `$snake($command.name)` | `DebugDump` | `debug_dump` |
| `$upper($decl.name, _, $command.name, _USES)` | `FileTool`, `Copy` | `FILE_TOOL_COPY_USES` |
| `$ident(sm, $member.index)` | member index `2` | `sm2` |

Case conversion starts a new word at a capital letter that follows a lowercase
letter, or that follows digits after a lowercase letter
(`Ir15Receiver` → `ir15_receiver`), and before the last capital of an acronym
(`HTTPServer` → `http_server`). Text that is already
all capitals keeps its words (`LED2D` → `led2d`).

### `$crate` and expressions

`$crate` in a template means the crate that defined the macro, exactly as in
`macro_rules!`. Generated code that names `$crate::settings::Setting` keeps
working when a user renames your crate in their `Cargo.toml`.

An `expr` value that is a compound expression is inserted in parentheses, so it
keeps its meaning: with `x: 1 + 2`, `$decl.x * 2` is `(1 + 2) * 2`.

The full rules for every construct are in the [`define!`] reference.

## A larger example

[`examples/commands.rs`](https://github.com/CarlKCarlK/macro-schema/blob/main/examples/commands.rs) defines `commands!`, which turns
a list of commands into an enum with a parser, argument checking, help text,
and a usage counter per command. A user writes:

```text
cli::commands! {
    #[derive(Hash)]
    pub FileTool {
        about: "A small file tool.",

        List { summary: "List files" },
        Copy { summary: "Copy a file", args: { min: 2, max: 2 } },
        /// Removes files. This written doc replaces the generated one.
        Remove { summary: "Remove files", args: { min: 1, max: usize::MAX } },
        DebugDump { summary: "Dump internal state", listed: false },
    }
}
```

Its schema has a required field (`about`), members with a required field
(`summary`), a defaulted field (`listed: expr = true`), and an optional nested
block (`args?: { min: expr, max: expr }`). From the declaration above, the
template generates:

- `pub enum FileTool { List, Copy, Remove, DebugDump }`, with the caller's
  `#[derive(Hash)]` added to the template's own derives;
- `FileTool::parse("copy")`, which matches each variant's `snake_case` name and
  otherwise returns a generated `FileToolParseError`;
- `FileTool::accepts(count)`, built with `$if let` on each command's `args`;
- `FileTool::help()`, which lists the commands whose `listed` is `true`;
- one `static FILE_TOOL_COPY_USES: AtomicUsize` (and so on) per command, used
  by `FileTool::record_use()`.

Run it with `cargo run --example commands`.

## When something goes wrong

Errors are reported in one of two places.

**Invocation errors** are found when a library user calls the macro. They point
at the user's tokens:

```text
setting! {
    pub Port { key: "server.port", value: u16, defualt: 8080 }
}
```

```text
error: unknown field `defualt`; expected one of `key`, `value`, `default`
   |
18 |     pub Port { key: "server.port", value: u16, defualt: 8080 }
   |                                                ^^^^^^^
```

Missing, duplicate, and wrongly-kinded fields, and too many or too few members,
are reported the same way, all at once rather than one per compile.

**Schema and template errors** are found when the library author compiles
`define!`, before any user exists. Here a template reads an optional field
(`env_var?: expr`) without `$if let`:

```text
error: `$decl.env_var` is optional; read it with `$if let Some(x) = ... { ... }`
   |
16 |             pub const ENV_VAR: &'static str = $decl.env_var;
   |                                                     ^^^^^^^
```

The [`define!`] reference lists every error message.

## Generated documentation

The schema is the single source of the macro's documentation. For `setting!`,
the macro's rustdoc page shows the `define!`'s own doc comment, followed by
this generated Markdown:

````text
**Syntax:**

```text
setting! {
    [<attributes>] [<visibility>] <Name> {
        key: <expr>,
        value: <ty>,
        default: <expr>, // optional, default: Default::default()
    }
}
```

**Fields:**

| Field | Kind | Default | Description |
| ----- | ---- | ------- | ----------- |
| `key` | expr | required | Key that identifies the setting, such as `"server.port"`. |
| `value` | ty | required | Type of the setting's value. |
| `default` | expr | `Default::default()` | Value used when the setting is absent. |
````

`#[default_display = "Default::default()"]` on the `default` field sets how
its default is shown, without changing the tokens the template receives.

Each generated item can be documented too. `$decl.doc` holds a description of
the declaration's configuration, so `#[doc = $decl.doc]` gives `Verbose` this
documentation:

```text
Generated by `setting!`.

| Field | Value |
| ----- | ----- |
| `key` | `"log.verbose"` |
| `value` | `bool` |
| `default` | `Default::default()` (default) |
```

The rule for combining this with the user's own documentation:

- By default, an item gets the generated description.
- If the user writes doc text on the declaration or member (`///` or
  `#[doc = "..."]`), that text replaces the generated description. `Port`'s
  page shows only "TCP port the server listens on."
- `#[doc(hidden)]` alone is not doc text and doesn't replace anything.

## Origin: Device Envoy

`macro-schema` came out of [Device Envoy](https://github.com/CarlKCarlK/device-envoy),
a Rust library for embedded devices built on [Embassy](https://embassy.dev).
Embassy tasks can't be generic, so a library can't provide one task definition
that serves any number of LED strips or buttons. Device Envoy's macros instead
generate a concrete task for each device the application declares, together
with the statics and types that device needs. Application developers choose
their devices, and how many of each, with no task-pool capacity fixed by the
library.

Those declaration macros were first written with `macro_rules!`, and they
became hard to write and maintain. `macro-schema` was built to replace them.
Device Envoy now defines 33 declaration macros with it (on its
`proc-macro-const-structures` branch, not yet released), including its LED
strip, LED panel, infrared, LCD, audio, button, and servo APIs. Moving to it:

- replaced 14,188 lines of hand-written `macro_rules!` with 3,246 lines of
  `define!` blocks (schemas, templates, and their documentation) plus 1,912
  lines of remaining helper macros;
- gave every macro the same syntax rules and the same checks for unknown,
  duplicate, missing, and wrongly-kinded fields;
- replaced hand-written syntax docs, which had drifted from what the macros
  accepted, with docs generated from the schemas;
- uncovered test runners that had silently stopped running tests;
- put each schema beside the API it generates, instead of in a separate
  proc-macro crate.

Embassy's task model is what made generated items necessary there; plenty of
embedded programs are well served by ordinary structs and generics. And
`macro-schema` itself has nothing to do with embedded Rust. It works for any
library whose users declare things that must become items, and the examples in
this README are ordinary `std` programs.

## Learn more

- [`define!`] is the complete reference: schema syntax, every template
  construct, documentation rules, the error catalog, and the escape hatch for
  output a template can't express.
- [`expand!`] is the procedural macro that every generated macro calls. You
  re-export it once; you never call it yourself.
- The repository's [`examples/`](https://github.com/CarlKCarlK/macro-schema/tree/main/examples) directory has the two programs above.
- [`specs/MACRO_SCHEMA_SPEC.md`](https://github.com/CarlKCarlK/macro-schema/blob/main/specs/MACRO_SCHEMA_SPEC.md), in the
  repository, explains how the crate works inside: how `define!` builds a macro
  that other crates can call, how `expand!` validates and renders, how spans
  are kept, and why the design is what it is.

## Crates

- `macro-schema`: the crate to depend on. It is `#![no_std]`, so `no_std`
  libraries can use it.
- `macro-schema-derive`: the procedural macro entry points.
- `macro-schema-core`: the implementation, as ordinary Rust over
  `proc_macro2`, so it can be unit tested.

## License

Licensed under either of the Apache License, Version 2.0 ([`LICENSE-APACHE`](https://github.com/CarlKCarlK/macro-schema/blob/main/LICENSE-APACHE))
or the MIT license ([`LICENSE-MIT`](https://github.com/CarlKCarlK/macro-schema/blob/main/LICENSE-MIT)), at your option.

[`define!`]: https://docs.rs/macro-schema/latest/macro_schema/macro.define.html
[`expand!`]: https://docs.rs/macro-schema/latest/macro_schema/macro.expand.html
