# a05 Branded Sha256 Newtype (enforcer-domain)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Branded Sha256 Newtype (enforcer-domain)`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-domain/src/sha256.rs`
- deps: `a01`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The legacy fingerprint stored hash results in fields typed as raw `string` (`sha256`, `digest`) and compared them as plain strings; a truncated, uppercased, or non-hex value would compare and store without complaint. The Rust engine hashes crate artifacts (a02) and content across `enforcer-scan`/`enforcer-proof`; without a dedicated newtype every digest is just another `String`. This is a **domain-modeling** workpack that populates the `enforcer-domain` crate — it FOLDS INTO arc-02-enforcer-domain.

## Where We Want To Be
A `Sha256` branded **newtype** in `enforcer-domain` with a private inner field, minted only via `Sha256::parse` (`^[0-9a-f]{64}$`, lowercase) and via `serde` using the same validator, plus a `Sha256::of(bytes)` hashing constructor, so every digest field and comparison across the workspace is typed `Sha256` and any raw string entering the digest set must pass the decode.

## Requirement Checklist
- [ ] Define `Sha256` newtype in `crates/enforcer-domain/src/sha256.rs` with a private inner field, `parse`-at-boundary (`^[0-9a-f]{64}$`, lowercase), and a `Sha256::of(&[u8])` constructor that hashes and yields a canonical lowercase-hex value.
- [ ] Derive `Debug, Clone, PartialEq, Eq, Hash`, `Display`, and `serde` (via the validator) so equality/comparison is `Sha256`-typed, not `String`.
- [ ] Consumers (a02 fingerprint, scan/proof) receive `Sha256`, never bare `String`; the inner value is not publicly constructible except via `parse`/`of`.
- [ ] Parse fails-closed (`Err`) on wrong length / uppercase / non-hex; `serde` deserialization rejects the same.

## Acceptance And Proof
Tier P0. `cargo test` in `enforcer-domain`: valid 64-hex mints, rejection of length/case/charset violations, `of()`+`parse()` round-trip, serde reject-on-bad-input. Fail/pass fixtures per RUST_ARCHITECTURE 5-way parity. The private field makes a bare `String` populating a `Sha256` field a compile error. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01. Populates the `enforcer-domain` crate (folds into arc-02-enforcer-domain); owns `crates/enforcer-domain/src/sha256.rs` exclusively; disjoint from a03/a04/a06 newtype modules. a02 CONSUMES this newtype from its own file (`enforcer-mcp/src/fingerprint.rs`) — no longer a shared file, so sequence a05 before a02 (clean dep, not a coordinate-on-one-file). Coordinate `mod`/`pub use` in `enforcer-domain/src/lib.rs`.
