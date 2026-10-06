# const-structures

`const-structures` lets a library declare a named configuration syntax once and
turn each invocation into normalized tokens for its own generator. The crate is
`no_std` and does not allocate at runtime. It does not prescribe a domain model
or a backend.

## A schema and output template

The library author writes a schema and a Rust macro output template. `define!`
generates the backend's input matcher, hidden export, and bare-name alias:

```text
const_structures::define! {
    pub indicators => __indicators_generate {
        enabled: expr = true,
    }

    generate {
        $(#[$attrs])*
        #[doc = $doc]
        $vis struct $name;

        impl $name {
            pub const ENABLED: bool = $field_enabled;
        }
    }
}
```

Re-export `const_structures::expand` as `__const_structures_expand` once at the
library crate root. The crate documentation has a compiling version of this
example. The generator path is relative to that root: a schema in `indicators`
would use `indicators::__indicators_generate`. Its last segment names the alias
created beside the schema. A template backend can also be called by other
client generators with normalized input.
If the schema module is private, re-export the backend through a public path
and use that path in the declaration so downstream expansions can reach it.

Template bindings distinguish metadata from schema fields:

| Input | Binding |
| --- | --- |
| Declaration metadata | `$attrs`, `$vis`, `$name`, `$doc` |
| Field `enabled` | `$field_enabled` |
| Nested field `range.min` | `$field_range_min` |
| Group member count | `$member_count` |
| Member metadata | `$member_attrs`, `$member_vis`, `$member_name`, `$member_doc`, `$member_index` |
| Member field `input` | `$member_field_input` |
| Nested member field `limits.min` | `$member_field_limits_min` |

Use `$( ... )*` for members, `$( ... )?` for optional fields/blocks, and
`$(#[$attrs])*` for attributes. Repetitions follow schema nesting. Blocks expose
their leaf fields; they do not create an opaque block binding. Names whose
flattened paths collide are rejected with a field diagnostic.
An optional block without leaf fields has no binding to repeat over; use an
explicit backend when its presence matters to the output.

A library can omit `generate` and supply its own backend, for example when it
needs multiple matcher arms. Templates handle one normalized schema shape;
the library still owns its domain-specific output code.

## A library-defined declaration

The standalone example re-exports the expansion entry point once at the crate
root, declares a schema, and defines a generator for its normalized output.
Its schema requires an address, defaults the enabled flag, and allows nested
optional range limits on each member. See the complete
[`normalized` example](examples/normalized.rs), including the matching
generator macro.

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
and every resolved field to the library's generator. The generator decides the
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
