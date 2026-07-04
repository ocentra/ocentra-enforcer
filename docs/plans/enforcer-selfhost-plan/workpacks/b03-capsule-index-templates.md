# b03 Capsule Index Templates

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Capsule Index Templates`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/plan/templates/capsule.tpl, src/plan/templates/workpack-index.tpl, src/plan/templates/plan-readme.tpl, test/fixtures/plan-templates/**`
- deps: `none`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The agent-capsule block, WORKPACK_INDEX, and the plan README no-read routing lists live only in authoring prose. b01 hand-inlines the capsule text and b02 hand-encodes the expected block. Two copies of the same contract are a drift bug waiting to happen.

## Where We Want To Be
One canonical set of verbatim templates (capsule, WORKPACK_INDEX, plan README with token-routing / no-read lists) that both the scaffolder (b01) and validator (b02) import as the single source of truth.

## Requirement Checklist
- [ ] `capsule.tpl` holds the exact agent-capsule block with named placeholders (`{{plan}}`, `{{doc}}`).
- [ ] `workpack-index.tpl` defines the WORKPACK_INDEX table columns and selection semantics.
- [ ] `plan-readme.tpl` encodes token-routing guidance and the explicit no-read lists (which docs an agent must NOT open unless routed).
- [ ] Templates are pure data (no logic); rendering is a deterministic string substitution.
- [ ] A frozen snapshot fixture pins each template so accidental edits surface as a diff.

## Acceptance And Proof
Tier P0 (contract/schema). Proof: `test/plan/templates.snapshot.test.ts` asserts each template equals its frozen fixture in `test/fixtures/plan-templates/**`; a contract test asserts b01 and b02 both import from these template files (no inline duplicates) via a grep/AST check that no capsule literal appears outside `src/plan/templates/`. Name these in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
This is the shared contract layer, so it is authored as its own workpack to keep b01 and b02 disjoint. Its owns: set is template files + template fixtures only; b01/b02/b04/b05 import these read-only. No dep edge is required because the templates are static data delivered before b01/b02 wire them; the parity check enforces adoption.
