# arc-13 Crate enforcer-literal-scan

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-literal-scan`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-literal-scan/**`, `Tools/ocentra-literal-scan/**`
- deps: `arc-01`, `arc-02`, `arc-05`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
A standalone Rust scored (T2) literal scanner already exists under `Tools/ocentra-literal-scan/`, outside the workspace. The `.mjs` literal-risk family (`src/literal-risk-*.mjs`) wraps/duplicates scoring logic. It is not a workspace crate and not wired to `enforcer-validator`.

## Where We Want To Be
`enforcer-literal-scan` folds the existing `Tools/ocentra-literal-scan` into the Cargo workspace as a first-class crate: the scored (T2) literal scanner, built on `enforcer-domain` types and exposing its findings through the `enforcer-validator`/scan interfaces so the engine consumes it like any other family.

## Requirement Checklist
- [ ] Fold `Tools/ocentra-literal-scan` into `crates/enforcer-literal-scan` per RUST_ARCHITECTURE.md ("the existing Rust scored (T2) scanner, folded in"): move/absorb its source, add it to `[workspace.members]`, retarget its types to `enforcer-domain`.
- [ ] Port the `.mjs` literal-risk family (`src/literal-risk-*.mjs`) scoring/threshold logic to Rust so there is one scorer, and expose results as `enforcer-domain` `Finding`s via the validator/scan interface.
- [ ] Preserve the existing scanner's T2 scoring behavior (fail-open vs. threshold semantics) — no regression versus the standalone tool.
- [ ] `cargo test -p enforcer-literal-scan` passes with fail/pass fixtures (scored-hit above threshold vs. clean below threshold), including the folded tool's existing tests.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-literal-scan` exits 0 with scored fail/pass fixtures, and that the standalone `Tools/ocentra-literal-scan` is subsumed (no orphaned copy). Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `crates/enforcer-literal-scan/**` plus the folded `Tools/ocentra-literal-scan/**` (sole owner during the fold — no sibling touches it). Deps arc-01/02/05. Parallel-safe with the lang crates (arc-06..12); consumed by arc-15 (scan).
