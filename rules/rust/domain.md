# Rust Domain Boundary Rules

Use this doc for `*.rs` files that define domain types, protocol shapes,
serialized envelopes, command/event structures, or public APIs.

## Covered Rules

- `RR-6.1`: no raw string/path types in non-boundary function signatures.
- `RR-6.2`: no primitive types in non-boundary function signatures.
- `RR-6.3`: no public raw fields.
- `RR-6.4`: raw private fields require `BRAND-INVARIANT:`.
- `RR-6.5`: type aliases must not disguise raw primitives.
- `RR-6.6`: tuple newtypes must not expose raw inner fields.
- `RR-6.26`: serialized domain fields must not expose raw identity primitives when enabled.

## Agent Rule

Use branded newtypes, enums, and validated constructors. Boundary exceptions
belong in profile globs, not inline bypasses.
