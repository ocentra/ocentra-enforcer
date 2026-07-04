# a03 Branded RuleId And Registry

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Branded RuleId And Registry`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `schemas/effect/**`, `src/rule-registry.*`, `src/policy.*`
- deps: `a01`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`schemas/effect/enforcer-schemas-*.mjs`, `src/rule-registry.mjs`, and `src/policy.mjs` pass rule ids as raw `string`. A typo or a policy id with no registry entry is only caught at runtime (if at all). There is no compile-time distinction between "an arbitrary string" and "a validated rule id".

## Where We Want To Be
A `RuleId` branded type minted only by an Effect Schema decoder at the registry boundary, so every downstream consumer receives `RuleId` and raw strings cannot flow into policy/registry APIs.

## Requirement Checklist
- [ ] Define `RuleId` brand + Effect Schema decoder in `schemas/effect/**`.
- [ ] `src/rule-registry.*` decodes ids at load; registry keys/lookups typed `RuleId`.
- [ ] `src/policy.*` accepts `RuleId`, never bare `string`, for rule references.
- [ ] Decode rejects unknown/malformed ids fail-closed (throws/Left), never silently coerces.
- [ ] Parity holds: every policy `RuleId` resolves to a registry entry (mechanical check).

## Acceptance And Proof
Tier P0 (contract/schema). Unit tests assert the decoder mints `RuleId` for valid ids and fails on invalid; a `tsc --noEmit` negative fixture proves a bare `string` cannot be passed where `RuleId` is required. A ruleId<->registry parity test enumerates all policy ids. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01. Owns the effect schema tree plus registry/policy modules exclusively; a04 (paths), a05 (sha256), a06 (coordination ids) brand disjoint domains in disjoint files, so all P0 brand packs run concurrently after a01.
