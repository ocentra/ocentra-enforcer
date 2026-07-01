# Rust Async And Runtime Shape Rules

Use this doc for `*.rs` files that include async code, runtime code, iterator
logic, or hot-path loops.

## Covered Rules

- `RR-8.1`: blocking primitives are forbidden inside async modules.
- `RR-8.16`: `std::sync::Mutex/RwLock` are forbidden in async modules.
- `RR-8.2`: C-style index loops are forbidden.
- `RR-8.18`: `tokio::spawn` handles must be tracked.
- `RR-8.19`: fire-and-forget spawn requires `TASK-JUSTIFICATION:`.
- `RR-8.20`: unbounded channels require `CHANNEL-JUSTIFICATION:`.
- `RR-8.21`: external async I/O futures must use a timeout policy.
- `RR-8.23`: async loops require cancellation.
- `RR-8.25`: blocking file/network I/O is forbidden in async modules.
- `RR-8.27`: libraries must not create global Tokio runtimes.
- `RR-8.28`: `block_on` is forbidden in library/domain code.
- `RR-8.29`: sleep-based tests are forbidden.
- `RR-8.30`: signatures must not expose raw `Arc<Mutex<T>>`.

## Agent Rule

Use async-compatible primitives and iterator APIs. If a blocking boundary is
intentional, isolate it behind an explicit boundary module and profile rule.

## Fails

```rust
tokio::spawn(async move { process().await });
let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
```

## Passes

```rust
let worker = tokio::spawn(async move { process().await });
let result = worker.await?;

// CHANNEL-JUSTIFICATION: backpressure is owned by the upstream bounded queue.
let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
```

## Fix Recipe

1. Store or await every spawned task handle.
2. Prefer bounded channels.
3. If an unbounded channel is intentional, document the backpressure boundary with `CHANNEL-JUSTIFICATION:`.

## Validator

- scanner: `rust/async-runtime`
- command: `ocentra-enforcer scan --root <repo> --files <file.rs>`
