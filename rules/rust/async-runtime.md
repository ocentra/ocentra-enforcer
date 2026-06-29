# Rust Async And Runtime Shape Rules

Use this doc for `*.rs` files that include async code, runtime code, iterator
logic, or hot-path loops.

## Covered Rules

- `RR-8.1`: blocking primitives are forbidden inside async modules.
- `RR-8.2`: C-style index loops are forbidden.

## Agent Rule

Use async-compatible primitives and iterator APIs. If a blocking boundary is
intentional, isolate it behind an explicit boundary module and profile rule.
