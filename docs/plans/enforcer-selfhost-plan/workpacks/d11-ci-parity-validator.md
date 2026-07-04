# d11 CI Parity Validator

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `CI Parity Validator`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-harness/src/ci_parity.rs, crates/enforcer-harness/tests/ci_parity.rs, crates/enforcer-harness/tests/fixtures/ci_parity/**`
- deps: `d01-rule-mechanization-engine, arc-18-enforcer-harness`
- tier: `P2 CI cross-platform`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Local dogfood steps (the pre-commit hook / cargo-alias emitted by the installer) and the CI job matrix can drift, and pinned toolchain versions can skew. ADBP's "local == CI" is a guideline, not a check. `enforcer-harness` (arc-18) already runs native tools (cargo/clippy/fmt/deny/audit) but has no module that proves local and CI run the same steps and pins.

## Where We Want To Be
A T1 module in `enforcer-harness` (`ci_parity`) asserting the set of local hook/steps equals the set of CI jobs, and that pinned versions (`rust-toolchain.toml`, workspace `Cargo.toml` deps, CI action/tool versions) agree between the two sources of truth. Runs both as a local `enforcer` check (through the harness run-adapters) and as a CI job.

## Requirement Checklist
- [ ] Parse the local step manifest (the installer-emitted pre-commit/cargo-alias steps) and the CI workflow job definition into comparable step sets.
- [ ] Assert step-set equality: any local-only or CI-only step fails closed (emit a `Finding`).
- [ ] Assert pinned versions agree — `rust-toolchain.toml` channel, key `[workspace.dependencies]` versions in the root `Cargo.toml`, and CI action/tool versions — parsed at boundary into typed records.
- [ ] Deterministic; runs both locally (via the `enforcer-harness` run-adapters) and as a CI job (self-referential parity). Obey `[workspace.lints]` (no `unwrap`/`panic`/`print_*`).
- [ ] Failure names the specific mismatched step or version in the `Finding`.

## Acceptance And Proof
Tier T1, P2 CI cross-platform. Prove via `cargo test -p enforcer-harness` (`crates/enforcer-harness/tests/ci_parity.rs`) over `crates/enforcer-harness/tests/fixtures/ci_parity/**`: an injected extra local step fails; an injected version skew fails; matched sets pass. The CI job runs the same check via the `enforcer` binary / `enforcer-harness` adapter. Mechanism: a normalized set/version diff between the local step manifest and the CI manifest, fail-closed on any delta.

## Parallel Ownership Notes
Depends on d01 lightly (shares the validator/parity conventions) and arc-18 for the `enforcer-harness` crate skeleton (run-adapters + compact diagnostics) it plugs into. Owns only `src/ci_parity.rs` and its `tests/ci_parity.rs` + fixtures inside `enforcer-harness` — disjoint from the arc-18 skeleton and from d28 (target-ci-parity, the other harness feature module) by file. Shares a CI stage with d05 but no files. owns disjoint? = Y (deps arc-18 sequences it after the crate skeleton exists).
