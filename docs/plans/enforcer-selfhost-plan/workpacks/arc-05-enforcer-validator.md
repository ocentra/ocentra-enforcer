# arc-05 Crate enforcer-validator

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-validator`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-validator/**`
- deps: `arc-01`, `arc-02`, `arc-04`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Detection logic and the "does this rule fire correctly?" parity check are ad hoc across `.mjs`. There is no reusable `Validator` abstraction and no shared fixture/parity harness, so every family reinvents fail/pass verification.

## Where We Want To Be
`enforcer-validator` defines the `Validator` trait + the reusable fixture/parity harness per doctrine: a validator impl plus fail/pass fixtures plus a `cargo test` detection test constitute the Rust-native 5-way parity. Every lang/security/literal-scan crate builds on this base.

## Requirement Checklist
- [ ] Define the `Validator` trait per RUST_ARCHITECTURE.md (consuming `enforcer-domain` `ScanScope`/`Finding`, keyed to `RuleId` from `enforcer-rules`).
- [ ] Implement the reusable fixture/parity harness: given a validator + its fail/pass fixtures, assert it fires on fail fixtures and stays silent on pass fixtures (the Rust-native parity oracle).
- [ ] Port the ad-hoc `.mjs` detection-check/parity plumbing into this shared harness so lang crates reuse it rather than reimplementing.
- [ ] `cargo test -p enforcer-validator` passes: the harness itself has a sample validator with fail/pass fixtures proving both directions (fires on fail, silent on pass), plus a negative test where a broken validator is caught.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-validator` exits 0 — the parity harness catches a fail fixture and passes a pass fixture, and flags a deliberately broken validator. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-validator/**`. Deps arc-01/02/04. This is the base every validator family (arc-06..13, arc-19) depends on, so it precedes them. Not parallel-safe upstream of the lang crates; runs right after arc-04.
