# arc-17 Crate enforcer-proof

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-proof`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-proof/Cargo.toml`, `crates/enforcer-proof/src/lib.rs`, `crates/enforcer-proof/src/harness.rs`, `crates/enforcer-proof/src/journal.rs`, `crates/enforcer-proof/src/envelope.rs`, `crates/enforcer-proof/src/claim.rs`, `crates/enforcer-proof/src/legacy_import.rs`, `crates/enforcer-proof/tests/**`
- deps: `arc-01`, `arc-02`, `arc-15`, `arc-25`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The proof harness — routed proofs, artifact capture, freshness, PR-ready claims, legacy import — lives across `src/proof-*.mjs` / `scripts/profile-proof-runner.mjs` as ad hoc JS. It is not a workspace crate.

## Where We Want To Be
`enforcer-proof` is the Rust proof harness per RUST_ARCHITECTURE.md: it runs routed proofs, captures artifacts, tracks freshness, and emits PR-ready claims, built on `enforcer-domain` (`Sha256` artifact hashes, `Report`) and driven by the scan engine. Per the OcentraParent audit/proof tamper-evidence borrow it adds an **append-only SHA-256 hash-chained NDJSON journal** (`src/journal.rs`): each record's hash folds in the previous record's hash, so the log is tamper-evident. The journal is **verified-on-open AND on-replay** (any break in the chain fails closed). It CONSUMES `enforcer-core`'s generic append-only `NdjsonWriter<T>` sink + the pure hash-chain primitive (no duplicated crypto here), and KEEPS the existing rich proof envelope (`src/envelope.rs`: git-state / in-toto / retention). Journal records are versioned serde structs (`schema_version` + `eventType`) reusing `enforcer-domain` newtypes, with two-layer redaction applied by `enforcer-core`.

## Requirement Checklist
- [ ] Implement the proof harness per RUST_ARCHITECTURE.md: route a proof request, run it (via `enforcer-scan` / registered proof routes), capture the artifact, hash it (`Sha256`), and record freshness.
- [ ] Implement the **append-only SHA-256 hash-chained NDJSON journal** (`src/journal.rs`) by CONSUMING `enforcer-core`'s `NdjsonWriter<T>` sink + hash-chain primitive (do not reimplement crypto): each appended record folds in the prior record's hash.
- [ ] **Verify-on-open and verify-on-replay**: opening or replaying the journal recomputes the chain and fails closed on any break/tamper/reorder.
- [ ] Keep the existing rich proof **envelope** (`src/envelope.rs`: git-state / in-toto / retention); journal records are versioned serde structs (`schema_version` + `eventType`) reusing `enforcer-domain` newtypes, redacted via `enforcer-core`'s two-layer redaction.
- [ ] Emit PR-ready claims tied to artifacts + freshness so a stale or missing artifact fails closed.
- [ ] Port the `.mjs` proof logic (`src/proof-*.mjs`, proof-cli, legacy storage/import, `scripts/profile-proof-runner.mjs`) to Rust, including a legacy-artifact import path.
- [ ] `cargo test -p enforcer-proof` passes with fail/pass fixtures (fresh artifact -> claim GREEN; stale/missing artifact -> claim fails; legacy import round-trip; an intact journal verifies on open, and a tampered/reordered record makes verify fail closed).
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-proof` exits 0 — fresh vs. stale/missing artifact behavior proven with fail/pass fixtures, plus the hash-chained journal: an intact chain verifies on open and on replay, and a seeded tamper/reorder makes verification fail closed. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
arc-17 owns the crate SKELETON + baseline of `enforcer-proof`: `Cargo.toml` (`[lints] workspace = true`), `src/lib.rs`, the proof `harness`, the hash-chained NDJSON `journal` (built ON `enforcer-core`'s sink + hash-chain util, not reimplemented), the rich `envelope`, the PR-ready `claim` logic, the `legacy_import` path, and `tests/**`. Deps arc-01/02/15 — arc-01 for the `NdjsonWriter<T>` sink + hash-chain primitive + two-layer redaction it consumes, and the scan engine (arc-15) to run routed proofs. Parallel-safe with arc-16/arc-18/arc-19/arc-20 — disjoint crate trees. Consumed by arc-21 (mcp) proof tool surface.

Parallel-ownership boundary (disjoint-owns model): proof-route or proof-rule feature packs own SPECIFIC files under this crate — e.g. `crates/enforcer-proof/src/routes/<name>.rs` (+ `tests/fixtures/<name>/**`) — and `deps: arc-17` so they land after this skeleton exists. They do NOT own the whole crate, and MUST NOT reimplement the sink/hash-chain (that lives in `enforcer-core`). Keep owns DISJOINT by file; sequence by `deps:`.
