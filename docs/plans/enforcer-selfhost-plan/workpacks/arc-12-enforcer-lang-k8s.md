# arc-12 Crate enforcer-lang-k8s

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-lang-k8s`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-k8s/**`
- deps: `arc-01`, `arc-02`, `arc-05`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Kubernetes-manifest rule detection (pod/security-context/RBAC/resource-limit shapes) is not organized as a language family in the current `.mjs`. No crate implements k8s against the `Validator` trait.

## Where We Want To Be
`enforcer-lang-k8s` is the per-family validator crate for Kubernetes manifests: `Validator` impls (built on `enforcer-validator`) covering the k8s rule family, each with fail/pass fixtures and a `cargo test` detection test.

## Requirement Checklist
- [ ] Implement the k8s-family `Validator` impls per RUST_ARCHITECTURE.md, keyed to their `RuleId`s in `enforcer-rules`.
- [ ] Port the corresponding `.mjs` k8s/manifest detection logic (generic-scanner YAML/manifest shapes) to Rust validators.
- [ ] Provide fail/pass fixtures per rule (well-formed vs. insecure manifests); wire them through the `enforcer-validator` parity harness.
- [ ] `cargo test -p enforcer-lang-k8s` passes: every validator fires on its fail fixture and is silent on its pass fixture.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-lang-k8s` exits 0 with fail/pass fixture coverage per rule. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-lang-k8s/**`. Deps arc-01/02/05. Parallel-safe with all sibling lang crates (arc-06..11) and arc-13 — disjoint crate trees.
