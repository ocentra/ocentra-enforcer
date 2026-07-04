# arc-01 Crate enforcer-core

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-core`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-core/**`
- deps: `a01`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The Cargo workspace + toolchain are stood up by a01; nothing yet provides the shared foundation crate (`enforcer-core` — error shapes, logging, exit codes) that every other crate depends on (today those concerns are re-implemented ad hoc across the legacy `.mjs` scripts). The OcentraParent logging/redaction/hash-chain borrows also have no home: there is no two-layer redaction, no generic append-only NDJSON sink, no pure hash-chain primitive, and no Windows-first path/time/env module for the whole workspace to reuse.

## Where We Want To Be
`arc-01` is the `enforcer-core` foundation CRATE (a01 owns the workspace root); it exists and every other crate in RUST_ARCHITECTURE.md can depend on it. It exports the shared `Result`/`Error` type, `tracing` setup, and the shared primitives (exit codes, cross-cutting utilities) that every other crate reuses. Per the OcentraParent "Logging = structured data (NO new crate)" borrow, it ALSO owns the reusable telemetry infrastructure that folds into core rather than a separate `enforcer-log` crate: TWO-layer redaction, a generic append-only `NdjsonWriter<T>` sink, a pure SHA-256 hash-chain primitive, a Windows-first path/time/env module, and a structured decode/validation error type. These are **VENDORED as-is** from OcentraParent's `logging-core` (copy the `redaction`/`ndjson_writer`/`hash_chain`/`path` module source per EXECUTION_MODEL §2 — do NOT re-implement), then adapted minimally: re-type bare ids to `enforcer-domain` newtypes and route errors through the shared `Result`/`Error`. (Domain record SHAPES live in `enforcer-domain` (arc-02); core owns the MECHANISM. The proof journal in `enforcer-proof` (arc-17) reuses the hash-chain util; d04 run-telemetry records ride the NDJSON sink.)

## Requirement Checklist
- [ ] Add `enforcer-core` as the first workspace member (a01 owns the root `Cargo.toml`/`rust-toolchain.toml`; the `members = ["crates/*"]` glob picks it up); opt in via `[lints] workspace = true` and honor the deny wall (no `unwrap/expect/panic/print_*`; no `pub use` barrels).
- [ ] Implement `enforcer-core` per RUST_ARCHITECTURE.md: the shared `Result<T>`/`Error` type (typed, `thiserror`-style), `tracing` initialization (structured fields keyed by `correlation_id`), and shared primitives (exit codes, small cross-cutting helpers).
- [ ] Two-layer redaction (OcentraParent borrow): a redactor that ALWAYS runs BOTH a key-name layer (redact by field/key name) AND a value-pattern layer (secret-detecting value regexes) over structured records before they are written; neither layer alone is sufficient.
- [ ] Generic append-only `NdjsonWriter<T>` sink (OcentraParent borrow): a reusable append-only newline-delimited-JSON writer generic over any serde `T`, used by d04 run-telemetry records and any pack emitting structured records; append-only (no rewrite/truncate).
- [ ] Pure SHA-256 hash-chain primitive (OcentraParent borrow): a side-effect-free hash-chain util (each entry hashes its payload + the prior digest) that `enforcer-proof` (arc-17) reuses for its tamper-evident journal; core owns the primitive, proof owns the journal envelope.
- [ ] Path/time/env module: cross-cutting path, time, and environment helpers with Windows backslash normalization (Windows-first per the consumer-contract doctrine — normalize `\` paths, argv-safe), so downstream crates never hand-roll path handling.
- [ ] Structured decode/validation error type: a typed error (feeding the shared `Result`/`Error`) for decode/validation failures that boundary parsers (`enforcer-domain`, `enforcer-events`, `enforcer-config`) return instead of stringly-typed errors.
- [ ] `cargo test -p enforcer-core` passes; include unit tests with fail/pass fixtures — error-conversion + exit-code mapping asserted both ways; redaction proven for BOTH layers (key-name hit and value-regex hit each redacted, and a clean record passes through); `NdjsonWriter<T>` append round-trip; hash-chain detects a tampered link vs. verifies an intact chain; Windows path normalization asserted.
- [ ] `cargo clippy -p enforcer-core` and `cargo fmt --check` are clean.

## Acceptance And Proof
Tier P1. A proof row in TEST_PROOF_EXPECTATIONS.md asserts `cargo test -p enforcer-core` exits 0 with fail/pass fixture coverage — including both redaction layers (key-name + value-regex), `NdjsonWriter<T>` append round-trip, hash-chain tamper-detect vs. verify, and Windows path normalization — plus clean `cargo clippy`/`cargo fmt --check` on the crate. Record the test artifact path.

## Parallel Ownership Notes
Foundation crate — owns ONLY `crates/enforcer-core/**` (including the redaction, `NdjsonWriter<T>`, hash-chain, path/time/env, and structured-error mechanisms folded in here per the borrows). It deps a01 (which owns the workspace root: `Cargo.toml`/`rust-toolchain.toml`) and blocks every other arc crate (they dep arc-01). Sibling crates own only their own `crates/<name>/**` (auto-included by a01's `members = ["crates/*"]` glob — no coordinated root edit needed). arc-17 (proof) reuses the hash-chain primitive; arc-02 (domain) owns the record SHAPES that ride the sink — disjoint by file.
