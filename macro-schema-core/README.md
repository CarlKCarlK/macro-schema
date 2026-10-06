# macro-schema-core

The proc-macro-independent implementation used by `macro-schema-derive`.
It parses schemas and invocations, validates values, inserts defaults, and
produces normalized tokens. This crate is an implementation detail for normal
users; depend on [`macro-schema`](https://crates.io/crates/macro-schema) to
define declaration macros.

The crate is licensed under MIT OR Apache-2.0. See the
[repository](https://github.com/CarlKCarlK/macro-schema) for the license files.
