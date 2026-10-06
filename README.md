# const-structures

`const-structures` lets a library declare a named configuration syntax once and
turn each invocation into normalized tokens for its own generator. The crate is
`no_std` and does not allocate at runtime. It does not prescribe a domain model
or a backend.

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
