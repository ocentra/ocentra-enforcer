# arc-11 Crate enforcer-lang-iac

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-lang-iac`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-iac/**`
- deps: `arc-01`, `arc-02`, `arc-05`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
IaC-family rule detection (Terraform/CloudFormation/config-file shapes, CFLint-adjacent checks) is only partially present in the `.mjs` scanners and not organized as a language family. No crate implements IaC against the `Validator` trait.

## Where We Want To Be
`enforcer-lang-iac` is the per-family validator crate for infrastructure-as-code: `Validator` impls (built on `enforcer-validator`) covering the IaC rule family, each with fail/pass fixtures and a `cargo test` detection test.

## Requirement Checklist
- [ ] Implement the IaC-family `Validator` impls per RUST_ARCHITECTURE.md, keyed to their `RuleId`s in `enforcer-rules`.
- [ ] Port the corresponding `.mjs` IaC/config-file detection logic (generic-scanner IaC shapes) to Rust validators; where an external engine is irreplaceable (e.g. CFLint) leave a graceful-skip adapter seam for `enforcer-harness`.
- [ ] Provide fail/pass fixtures per rule; wire them through the `enforcer-validator` parity harness.
- [ ] `cargo test -p enforcer-lang-iac` passes: every validator fires on its fail fixture and is silent on its pass fixture.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-lang-iac` exits 0 with fail/pass fixture coverage per rule. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-lang-iac/**`. Deps arc-01/02/05. Parallel-safe with all sibling lang crates (arc-06..10, arc-12) and arc-13 — disjoint crate trees.
