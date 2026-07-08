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

## Where We Are (updated 2026-07-05 — see reconciliation note below)
`enforcer-core` exists, builds, and is depended on by every other crate. Originally this section described the crate as not-yet-built with the OcentraParent borrows having "no home" — that was true through 2026-07-04 but is stale now; keeping the original text below for the historical record of what a01/arc-01 solved, followed by what actually shipped.

Original framing (2026-07-04, now resolved): "The Cargo workspace + toolchain are stood up by a01; nothing yet provides the shared foundation crate... The OcentraParent logging/redaction/hash-chain borrows also have no home..."

**RECONCILIATION NOTE (2026-07-05, commit `3122786`):** the "VENDORED as-is... do NOT re-implement" instruction below (the original `Where We Want To Be`) was written when OcentraParent's `logging-core` source was physically unreachable (lesson L12), so `redaction`/`ndjson_writer`/`hash_chain`/`platform` were spec-implemented against the workpack's behavioral description instead. That source is now reachable and has been diff-reconciled directly against it (not via an arc-25 pass, contra L12's original framing — arc-25 itself never got vendored; see `crates/enforcer-events`, still pending). Finding: **none of the four modules needed a literal port** — `redaction` is a deliberate two-layer/nested-JSON extension beyond upstream's weaker single-layer/flat design; `hash_chain` has no upstream counterpart at all (upstream `logging-core` contains no hash-chain logic whatsoever); `platform`/`ndjson_writer`'s upstream equivalents are superseded by Enforcer's own `RepoRoot`/`RelPath` branded newtypes and explicit-path-per-caller design. Full detail in `crates/enforcer-core/src/lib.rs`'s module doc and `redaction.rs`/`hash_chain.rs`'s own doc comments.

## Where We Want To Be
`arc-01` is the `enforcer-core` foundation CRATE (a01 owns the workspace root); it exists and every other crate in RUST_ARCHITECTURE.md depends on it. It exports the shared `Result`/`Error` type, `tracing` setup, and the shared primitives (exit codes, cross-cutting utilities) that every other crate reuses, plus the reusable telemetry infrastructure per the OcentraParent "Logging = structured data (NO new crate)" borrow: TWO-layer redaction, a generic append-only `NdjsonWriter<T>` sink, a pure SHA-256 hash-chain primitive, a Windows-first path/time/env module (`platform.rs`), and a structured decode/validation error type (`DecodeError`). Per the reconciliation above, these are Enforcer-native implementations that satisfy (and in redaction's case, exceed) the borrowed behavioral contract, not literal vendored copies — see `crates/enforcer-core/src/lib.rs` for the full attribution. (Domain record SHAPES live in `enforcer-domain` (arc-02); core owns the MECHANISM. The proof journal in `enforcer-proof` (arc-17) reuses the hash-chain util; d04 run-telemetry records ride the NDJSON sink.)

## Requirement Checklist
- [x] Add `enforcer-core` as the first workspace member (a01 owns the root `Cargo.toml`/`rust-toolchain.toml`; the `members = ["crates/*"]` glob picks it up); opt in via `[lints] workspace = true` and honor the deny wall (no `unwrap/expect/panic/print_*`; no `pub use` barrels).
- [x] Implement `enforcer-core` per RUST_ARCHITECTURE.md: the shared `Result<T>`/`Error` type (typed, `thiserror`-style), `tracing` initialization (structured fields keyed by `correlation_id`), and shared primitives (exit codes, small cross-cutting helpers).
- [x] Two-layer redaction (OcentraParent borrow, extended — see reconciliation note): a redactor that ALWAYS runs BOTH a key-name layer (redact by field/key name) AND a value-pattern layer (secret-detecting value regexes) over structured records before they are written; neither layer alone is sufficient.
- [x] Generic append-only `NdjsonWriter<T>` sink (OcentraParent borrow, Enforcer-native design — see reconciliation note): a reusable append-only newline-delimited-JSON writer generic over any serde `T`, used by d04 run-telemetry records and any pack emitting structured records; append-only (no rewrite/truncate).
- [x] Pure SHA-256 hash-chain primitive (Enforcer-native, no upstream counterpart exists — see reconciliation note): a side-effect-free hash-chain util (each entry hashes its payload + the prior digest) that `enforcer-proof` (arc-17) reuses for its tamper-evident journal; core owns the primitive, proof owns the journal envelope.
- [x] Path/time/env module: cross-cutting path, time, and environment helpers with Windows backslash normalization (Windows-first per the consumer-contract doctrine — normalize `\` paths, argv-safe), so downstream crates never hand-roll path handling.
- [x] Structured decode/validation error type: a typed error (feeding the shared `Result`/`Error`) for decode/validation failures that boundary parsers (`enforcer-domain`, `enforcer-events`, `enforcer-config`) return instead of stringly-typed errors.
- [x] `cargo test -p enforcer-core` passes; include unit tests with fail/pass fixtures — error-conversion + exit-code mapping asserted both ways; redaction proven for BOTH layers (key-name hit and value-regex hit each redacted, and a clean record passes through); `NdjsonWriter<T>` append round-trip; hash-chain detects a tampered link vs. verifies an intact chain; Windows path normalization asserted. (Verified 2026-07-05: 45/45 tests green — 40 unit + 5 integration.)
- [x] `cargo clippy -p enforcer-core` and `cargo fmt --check` are clean. (Verified 2026-07-05 after fixing a `doc_lazy_continuation` violation introduced by the reconciliation commit's own doc comment — `crates/enforcer-core/src/lib.rs:34`.)

## Acceptance And Proof
Tier P1. `cargo test -p enforcer-core` exits 0 with fail/pass fixture coverage — including both redaction layers (key-name + value-regex), `NdjsonWriter<T>` append round-trip, hash-chain tamper-detect vs. verify, and Windows path normalization — plus clean `cargo clippy`/`cargo fmt --check` on the crate. Verified 2026-07-05; artifact at `proof/cargo/arc-01.txt`. **Note:** `TEST_PROOF_EXPECTATIONS.md`'s combined `arc-01..24 (each)` row still reads PENDING — that row covers 24 crates collectively and is a known, separate, workspace-wide staleness issue (WORKPACK_INDEX.md shows the same generic `TODO` status on every one of arc-01 through arc-08 and likely beyond, not specific to arc-01); it is intentionally NOT flipped here since this workpack has fresh evidence for arc-01 only, not the other 23.

## Parallel Ownership Notes
Foundation crate — owns ONLY `crates/enforcer-core/**` (including the redaction, `NdjsonWriter<T>`, hash-chain, path/time/env, and structured-error mechanisms folded in here per the borrows). It deps a01 (which owns the workspace root: `Cargo.toml`/`rust-toolchain.toml`) and blocks every other arc crate (they dep arc-01). Sibling crates own only their own `crates/<name>/**` (auto-included by a01's `members = ["crates/*"]` glob — no coordinated root edit needed). arc-17 (proof) reuses the hash-chain primitive; arc-02 (domain) owns the record SHAPES that ride the sink — disjoint by file.
