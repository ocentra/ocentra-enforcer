# arc-07 Crate enforcer-lang-ts

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-lang-ts`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-ts/**`
- deps: `arc-01`, `arc-02`, `arc-05`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
TypeScript/JS-family rule detection lives in `src/source-policy-typescript-*.mjs` (source-domain, package-manifest, tsconfig, tests, boundaries) and the eslint-rule logic, as ad hoc JS. No crate implements the TS family against the `Validator` trait. Note: the enforcer VALIDATES TS from Rust — it does not run in TS.

## Where We Want To Be
`enforcer-lang-ts` is the per-family validator crate for TypeScript: `Validator` impls (built on `enforcer-validator`) covering the TS rule family (source domain, package manifest, tsconfig, boundaries, tests), each with fail/pass fixtures and a `cargo test` detection test.

## Requirement Checklist
- [ ] Implement the TS-family `Validator` impls per RUST_ARCHITECTURE.md, keyed to their `RuleId`s in `enforcer-rules`.
- [ ] Port the corresponding `.mjs` detection logic (`src/source-policy-typescript-*.mjs`, package-manifest/tsconfig/boundaries/tests, and the eslint-rule detection) to Rust validators.
- [ ] Provide fail/pass fixtures per rule; wire them through the `enforcer-validator` parity harness.
- [ ] `cargo test -p enforcer-lang-ts` passes: every validator fires on its fail fixture and is silent on its pass fixture.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-lang-ts` exits 0 with fail/pass fixture coverage per rule. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-lang-ts/**`. Deps arc-01/02/05. Parallel-safe with all sibling lang crates (arc-06, arc-08..12) and arc-13/arc-19 — disjoint crate trees.
