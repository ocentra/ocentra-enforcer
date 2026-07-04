# d02 Baseline Grandfather Ratchet

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Baseline Grandfather Ratchet`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-scan/src/rules/baseline_ratchet.rs, crates/enforcer-scan/tests/fixtures/baseline_ratchet/**`
- deps: `arc-15`, `d01-rule-mechanization-engine`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The enforcer runs all-or-nothing: a scan against a legacy codebase either passes or floods with pre-existing findings, so teams disable rules. ADBP mentions "grandfathering" only as guidance. arc-15 stands up the `enforcer-scan` crate skeleton (the parallel scan engine + router + modes) but there is no persisted baseline and no ratchet living inside it. This pack owns the `src/rules/baseline_ratchet.rs` module (a `Validator` over the aggregated `enforcer-domain` `Report`) plus its `cargo test` fixtures — it does NOT own the whole `enforcer-scan` crate.

## Where We Want To Be
A `--baseline` scan mode where findings present in a recorded baseline count as warnings, but any new `Finding`, or growth in count/severity of a grandfathered one, fails closed. The baseline is a versioned `serde` record (branded `RuleId` + normalized `RelPath` location + count + `Sha256` of the baseline file) written via the `enforcer-core` append/serialization utilities.

## Requirement Checklist
- [ ] `enforcer check --baseline write` records current findings to a stable baseline file as a versioned `serde` record (`RuleId` + normalized location + count) with a `Sha256` integrity hash.
- [ ] A `--baseline` run classifies each `Finding`: in-baseline -> warn; not-in-baseline -> error; grown-past-baseline -> error — implemented as an `enforcer-scan` `Validator`/mode returning `enforcer-domain` `Finding`s/`Violation`s.
- [ ] Location normalization is deterministic (`RelPath` + normalized anchor) so line drift alone does not create false "new" findings.
- [ ] Ratchet is one-directional: fixing a finding shrinks the allowance; it can never silently expand (fail-closed on any positive delta).
- [ ] Baseline entries reference real registry `RuleId`s — parity enforced via the d01 mechanization oracle; obey `[workspace.lints]` (no `unwrap/expect/panic/print_*`), no inline-disable.

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `cargo test -p enforcer-scan` over `crates/enforcer-scan/tests/fixtures/baseline_ratchet/**`: (a) clean baseline write, (b) unchanged run passes with warnings, (c) one added finding fails, (d) one grown count fails, (e) one removed finding shrinks the allowance. Mechanism: set-diff of normalized `Finding` keys against the persisted baseline record, fail-closed on delta. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Deps `arc-15` (owns the `enforcer-scan` crate skeleton — engine/router/modes/`Cargo.toml`/`lib.rs`) and `d01-rule-mechanization-engine` (for `RuleId`/fixture parity of baseline entries). Owns only `src/rules/baseline_ratchet.rs` + its `tests/fixtures/baseline_ratchet/**`, disjoint by file from the arc-15 skeleton and from d03/d04/d05, so it runs concurrently with them once arc-15 and d01 land.
