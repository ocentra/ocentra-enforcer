# Rust Imports And Module Shape Rules

Use this doc for `*.rs` files when the change touches imports, module exports,
facades, preludes, or crate public API shape.

## Covered Rules

- `RR-7.1`: wildcard imports are forbidden.
- `RR-7.2`: wildcard public re-exports are forbidden.
- `RR-7.3`: public re-exports must match project policy.
- `RR-7.4`: dumping-ground modules such as `utils`, `helpers`, `common`, `misc`, `shared`, or `stuff` are forbidden.
- `RR-7.5`: `build.rs` is forbidden by default unless the profile explicitly allows it.

## Agent Rule

For Ocentra-style strict projects, remove `pub use` entirely. Do not replace it
with another barrel-like shim.
