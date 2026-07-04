# b01 Plan Scaffolder

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Plan Scaffolder`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/plan/scaffolder/**, src/plan/skeleton-templates/**, test/plan/scaffolder.*.test.ts`
- deps: `none`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Plans today are hand-assembled: someone copies a prior plan dir, hand-edits the capsule, and hopes the skeleton matches. There is no `ocentra plan new` command and no single source of the OcentraParent skeleton, so drift between plans is guaranteed and unprovable.

## Where We Want To Be
A deterministic emitter: `ocentra plan new <name>` writes a complete, byte-stable plan skeleton (PLAN_STATE, PLAN_EXECUTION_BLUEPRINT, TEST_PROOF_EXPECTATIONS, WORKPACK_INDEX, capsule-stamped workpack stub) that b02 validates green.

## Requirement Checklist
- [ ] `ocentra plan new <name>` CLI subcommand emits the full directory tree under `docs/plans/<name>/`.
- [ ] Every emitted file carries the exact agent-capsule block and required frontmatter (owns/deps/tier).
- [ ] Emission is deterministic: same `<name>` yields byte-identical output (golden fixture).
- [ ] Refuses to overwrite an existing plan dir (fail-closed) unless `--force`.
- [ ] Emitted skeleton passes b02's PLAN-* validator with zero findings.

## Acceptance And Proof
Tier T1 / P1. Proof: `test/plan/scaffolder.emit.test.ts` diffs emitter output against a checked-in golden tree fixture (`test/fixtures/plan-golden/**`); `scaffolder.determinism.test.ts` runs the emitter twice and asserts identical bytes; a cross-check test feeds emitted output to b02's validator entrypoint and asserts zero findings. Name these in TEST_PROOF_EXPECTATIONS.md proof rows before DONE.

## Parallel Ownership Notes
Blocks nothing directly but b02 and b05 consume its golden fixture and validator cross-check. Its owns: set (scaffolder + skeleton-templates + its tests) is disjoint from b02 (validator src) and b03 (capsule/index template text), so all three run concurrently; the only shared artifact is the golden fixture, produced here and read-only elsewhere.
