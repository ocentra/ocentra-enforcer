# Rust Source Rules

Use this doc for `*.rs` files when routing general source safety,
error-handling, allocation, panic, unsafe, lint-suppression, and runtime string
rules.

## Covered Rules

- `RR-2.1`: no `#[allow(...)]` or `#[expect(...)]` lint suppressions.
- `RR-2.2`: no validator suppression comments.
- `RR-3.1`: unsafe Rust is forbidden by default.
- `RR-3.2`: unsafe blocks require nearby `SAFETY:` documentation when unsafe is enabled.
- `RR-3.3`: unsafe functions require a `# Safety` section when unsafe is enabled.
- `RR-3.4`: raw pointers are forbidden in public/domain APIs.
- `RR-4.1`: no `.unwrap()` or `.expect()`.
- `RR-4.2`: no panic-like production macros.
- `RR-4.3`: no `dbg!`, `println!`, or `eprintln!` in Rust logic.
- `RR-4.4`: no erased application errors in non-boundary domain code.
- `RR-5.1`: clone requires `CLONE-JUSTIFICATION:`.
- `RR-5.2`: string allocation requires `ALLOC-JUSTIFICATION:`.
- `RR-5.3`: unchecked indexing and slicing are forbidden.
- `RR-5.4`: lossy casts require `CAST-JUSTIFICATION:`.
- `RR-18.16`: runtime Rust source must not contain inline string literals when enabled.

## Agent Rule

Fix code structure. Do not add lint disables, suppression comments, or config
downgrades to pass this family.
