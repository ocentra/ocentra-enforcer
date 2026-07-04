# a08 Waiver Honesty Overrides To Waivers

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Waiver Honesty Overrides To Waivers`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `ocentra-enforcer.config.json`
- deps: `none`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`ocentra-enforcer.config.json` carries `sourceShapeOverrides` (51 entries as of writing, e.g. `check-source-core.mjs` with `maxBranches: 122`, `maxFunctionLines: 540`). These silently raise limits with no reason, no owner, no expiry — the enforcer excuses itself without saying so. That is dishonest self-enforcement.

## Where We Want To Be
Every override is either deleted (the file is fixed by a migration pack) or converted to an explicit, justified waiver: a structured record with `path`, the specific rule waived, a written `reason`, and an owner, so each excuse is visible and auditable.

## Requirement Checklist
- [ ] Enumerate all `sourceShapeOverrides` (51) and classify each: fix-later vs must-waive.
- [ ] Introduce a `waivers` shape (`path`, `ruleId`, `reason` non-empty, `owner`, optional `expires`).
- [ ] Convert every retained override into a waiver with a real reason; drop no-longer-needed ones.
- [ ] No silent numeric limit bumps remain in `sourceShapeOverrides`.
- [ ] Waiver schema is decodable (ties to a03 `RuleId`) and reason is mechanically required non-empty.

## Acceptance And Proof
Tier P1. A test asserts the config has zero bare `sourceShapeOverrides` limit-bumps and that every `waivers[]` entry has a non-empty `reason` and a valid `ruleId`. Count parity: waived + fixed == original 51. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
No deps (config-only), but its waiver `ruleId` field aligns with a03's brand and its enforcement lands via a09/a10. Owns `ocentra-enforcer.config.json` exclusively — the single most contended file, so it is deliberately one workpack.
