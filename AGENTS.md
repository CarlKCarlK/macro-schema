# Coding Notes for Agents

- This crate is a general-purpose proc macro. Device Envoy
  (`~/programs/mcu/device-envoy`, branch `proc-macro-const-structures`) is its
  first serious customer and test corpus, not a compatibility target. Do not
  add Device Envoy concepts to this crate.
- Follow the shared Rust rules in Device Envoy's `AGENTS.md` (error handling,
  no `mod.rs`, no lint suppression, no `unsafe` without justification, no
  backwards-compatibility shims, `rust,no_run` doctests).
- Put planning documents in `specs/`. Each spec carries a `TODO0` marker near
  the top reminding readers to delete it once the work is done.
- Prefer good compile errors over permissive parsing: every rejected input
  gets a `trybuild` UI test in `tests/ui/`.
