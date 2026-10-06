# const-structures

`const-structures` lets a library declare a named configuration syntax once and
turn each invocation into normalized tokens for its own generator. The crate is
`no_std` and does not allocate at runtime. It does not prescribe a domain model
or a backend.

## A schema and output template

The library author writes a schema and a `generate { ... }` template. `define!`
type-checks the template against the schema and generates the hidden export and
bare-name alias:

```text
const_structures::define! {
    /// A configured indicator.
    pub indicators {
        /// Whether the indicator starts enabled.
        enabled: expr = true,
    }

    generate {
        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name;

        impl $decl.name {
            pub const ENABLED: bool = $decl.enabled;
        }
    }
}
```

Re-export `const_structures::expand` as `__const_structures_expand` once at the
library crate root. The crate documentation has a compiling version of this
example.

A template is ordinary Rust tokens plus four constructs:

| Construct | Meaning |
| --- | --- |
| `$decl.name`, `$decl.vis`, `$decl.doc`, `$decl.attrs` | Declaration built-ins |
| `$decl.enabled`, `$decl.range.min` | Declaration fields, nested by `.` |
| `$for m in $decl.members { ... }` | Repeat per member |
| `$m.name`, `$m.vis`, `$m.doc`, `$m.attrs`, `$m.index`, `$m.input` | Member built-ins and fields |
| `$if let Some(x) = $m.limits { ... } else { ... }` | Read an optional field or block |
| `$ident(...)`, `$snake(...)`, `$upper(...)` | Build an identifier from parts |

Everything belonging to the declaration is reached through `$decl`; everything
belonging to a loop or `$if let` variable, through that variable. `$crate`
refers to the defining library. A written `///` or `#[doc = "..."]` on a
declaration or member replaces its generated `$decl.doc` / `$m.doc` rather than
being appended to.

A library that needs output a template cannot express can instead write
`pub NAME => GENERATOR_PATH { BODY }` and supply its own `macro_rules!`
generator, which receives every resolved field in schema order. The generator
path is relative to the crate root.

## A library-defined declaration

The standalone example re-exports the expansion entry point once at the crate
root and declares a schema with a template.
Its schema requires an address, defaults the enabled flag, and allows nested
optional range limits on each member. See the complete
[`normalized` example](examples/normalized.rs).

A downstream invocation supplies required values, may override defaults, adds
attributes and visibility, and can configure members:

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

The schema validates field names and kinds, reports missing or duplicate
values, inserts defaults, and forwards attributes, visibility, docs, members,
and every resolved field to the library's template. The template decides the
resulting Rust types and representation. The example normalizes values to
associated constants and checks its output when run with
`cargo run --example normalized`.

## Workspace crates

- `const-structures` re-exports the schema declaration and expansion macros.
- `const-structures-core` contains the ordinary Rust parser, validator, and
  token normalization implementation.
- `const-structures-derive` provides proc-macro entry points that forward to
  the core implementation.

See the crate documentation and examples for invocation details.
