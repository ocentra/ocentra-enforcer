# arc-20 Crate enforcer-plan

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-plan`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-plan/**`
- deps: `arc-01`, `arc-02`, `arc-04`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Track B — the plan scaffolder and the PLAN-* structure validator (the rules that enforce workpack/plan document shape, parallel-safety, proof rows) — exists as plan docs + ad hoc `.mjs` checks, not a crate.

## Where We Want To Be
`enforcer-plan` is the Track B crate per RUST_ARCHITECTURE.md: a plan scaffolder that emits the workpack skeleton (agent-capsule + owns/deps/tier + the standard sections) and a PLAN-* structure validator (`Validator` impls) that enforce the plan document contract (e.g. PLAN-PARALLEL-SAFETY disjoint-owns).

## Requirement Checklist
- [ ] Implement the plan scaffolder per RUST_ARCHITECTURE.md: generate a valid workpack file (agent-capsule + owns/deps/tier + Where-We-Are/Want/Requirement-Checklist/Acceptance-And-Proof/Parallel-Ownership-Notes).
- [ ] Implement PLAN-* structure `Validator`s (keyed to their `RuleId`s in `enforcer-rules`): missing capsule, missing sections, non-disjoint owns between no-dep-edge workpacks (PLAN-PARALLEL-SAFETY), stale proof rows.
- [ ] Port any `.mjs` plan/workpack-shape checks to Rust.
- [ ] `cargo test -p enforcer-plan` passes with fail/pass fixtures (malformed workpack -> violations; well-formed workpack -> clean; overlapping owns between independent packs -> PLAN-PARALLEL-SAFETY fires).
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-plan` exits 0 — plan structure validators fire on malformed fixtures and pass on well-formed ones. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-plan/**`. Deps arc-01/02/04. Parallel-safe with arc-14..19 — disjoint crate trees on the rules/validator base.
