# a09 Anti Silent Skip Coverage (enforcer-scan)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Anti Silent Skip Coverage (enforcer-scan)`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-scan/src/coverage.rs`, `crates/enforcer-scan/src/outcome.rs`
- deps: `a01`
- tier: `P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The legacy generic scanners and CLI scan could early-return on unmatched extension, missing tool, or empty selection with no emitted record. A validator that runs on nothing looked identical to one that ran and passed — the hollow self-scan: green because it checked nothing. In the Rust engine, `enforcer-scan` fans out validators (rayon) over routed targets; without an explicit coverage model the same hollow-green failure mode returns.

## Where We Want To Be
`enforcer-scan` models every candidate's outcome explicitly as a Rust enum — `Ran { .. }` or `Skipped { reason }` (non-empty reason) — and the scan engine hard-fails when it ran zero checks. A skip is never silent; the report distinguishes "passed" from "did not run", so a scan that executed no validators FAILS rather than reporting a bare success.

## Requirement Checklist
- [ ] Define an outcome enum in `crates/enforcer-scan/src/outcome.rs`: `Outcome::Ran { .. } | Outcome::Skipped { reason: String }` with `reason` guaranteed non-empty by construction.
- [ ] Every validator dispatch in `enforcer-scan` returns an `Outcome` for each target it is handed; no code path returns from the scan without recording a result per target.
- [ ] `crates/enforcer-scan/src/coverage.rs` aggregates outcomes into ran/skipped counts and surfaces skip reasons in the `Report` (from `enforcer-domain`).
- [ ] A scan whose total ran-count is zero returns an error / non-zero exit (anti-silent-skip): zero checks executed is a hard failure, not a clean pass.
- [ ] Skip counts and reasons are part of the serialized report, so a hollow scan is visible.

## Acceptance And Proof
Tier P4 (self-enforce green). `cargo test` in `enforcer-scan`: unmatched-extension, missing-tool, and empty-selection inputs each yield an explicit `Outcome::Skipped { reason }`; a scan that ran zero checks returns `Err`/fails; the report exposes ran/skipped counts. Running the enforcer's scan on its own workspace (a10 dogfood) shows a nonzero ran-count with reasons for any skips. Fail/pass fixtures per RUST_ARCHITECTURE 5-way parity. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01. Owns the two `enforcer-scan` modules exclusively; disjoint from a10 (which owns CI workflows and the native-dogfood wiring). a09 makes the scan honest (explicit outcomes + zero-ran hard-fail); a10 runs it on the enforcer's own crates and hard-fails CI. Coordinate `mod`/`pub use` in `enforcer-scan/src/lib.rs`.
