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

- owns: `src/baseline-ratchet.ts, src/baseline-store.ts, tests/baseline-ratchet.test.mjs, tests/fixtures/baseline/**`
- deps: `d01-rule-mechanization-engine`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The enforcer runs all-or-nothing: a scan against a legacy codebase either passes or floods with pre-existing findings, so teams disable rules. ADBP mentions "grandfathering" only as guidance. There is no persisted baseline and no ratchet in `src/cli-check-routing.mjs`.

## Where We Want To Be
A `--baseline` mode where findings present in a recorded baseline count as warnings, but any new finding, or growth in count/severity of a grandfathered one, fails closed.

## Requirement Checklist
- [ ] `ocentra check --baseline write` records current findings to a stable, hashed baseline file (ruleId + normalized location + count).
- [ ] `--baseline` run classifies each finding: in-baseline -> warn; not-in-baseline -> error; grown-past-baseline -> error.
- [ ] Location normalization is deterministic so line drift alone does not create false "new" findings.
- [ ] Ratchet is one-directional: fixing a finding shrinks the allowance; it can never silently expand.
- [ ] Baseline entries reference real registry ruleIds (parity via d01 engine).

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `tests/baseline-ratchet.test.mjs` with `tests/fixtures/baseline/**`: (a) clean baseline write, (b) unchanged run passes with warnings, (c) one added finding fails, (d) one grown count fails, (e) one removed finding shrinks allowance. Mechanism: set-diff of normalized finding keys against the persisted baseline, fail-closed on delta.

## Parallel Ownership Notes
Depends on d01 for ruleId parity of baseline entries. Disjoint `owns:` (baseline files + its own fixtures) means it runs concurrently with d03/d04 once d01 lands.
