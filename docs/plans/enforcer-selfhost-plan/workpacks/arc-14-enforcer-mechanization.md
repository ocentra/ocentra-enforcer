# arc-14 Crate enforcer-mechanization

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-mechanization`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-mechanization/**`
- deps: `arc-01`, `arc-02`, `arc-04`, `arc-05`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Track D (d01) rule mechanization — scaffolding a new rule and the fail-closed parity oracle that refuses to accept a rule without proving its validator/fixtures — is spread across `.mjs` check/contract scripts (`scripts/check-source-core-contract-*.mjs`). It is not a crate.

## Where We Want To Be
`enforcer-mechanization` is the d01 crate: a rule scaffolder that emits a new structured rule record (into `enforcer-rules`) with its `Validator` stub + fail/pass fixture slots, plus a fail-closed parity oracle that rejects any rule lacking a firing validator and both fixtures.

## Requirement Checklist
- [ ] Implement the rule scaffolder per RUST_ARCHITECTURE.md: generate a well-formed `enforcer-rules` record + `Validator` scaffold + fixture slots for a new rule.
- [ ] Implement the fail-closed parity oracle: a rule is only accepted if its validator fires on the fail fixture and is silent on the pass fixture (reuse the `enforcer-validator` harness); otherwise it hard-fails.
- [ ] Port the `.mjs` contract-coverage / contract-load logic (`scripts/check-source-core-contract-*.mjs`) that enforces rule/fixture completeness to Rust.
- [ ] `cargo test -p enforcer-mechanization` passes: scaffolding a rule produces a loadable record; the oracle rejects a rule with a missing fixture or non-firing validator (fail fixture) and accepts a complete one (pass fixture).
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-mechanization` exits 0 — scaffolder output loads and the fail-closed oracle rejects incomplete rules / accepts complete ones. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-mechanization/**`. Deps arc-01/02/04/05. Parallel-safe with the lang crates and arc-13 once the validator base + rule registry exist.
