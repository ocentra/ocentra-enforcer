# d09 Per-Stack Agents And Doc-Rule Parity

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Per-Stack Agents And Doc-Rule Parity`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `docs/agents/**, src/doc-rule-parity.ts, tests/doc-rule-parity.test.mjs`
- deps: `d01-rule-mechanization-engine`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`AGENTS.md` and per-language rule docs (`rules/rust`, `rules/typescript`, `rules/python`, `rules/common`) contain must/never guidance. ADBP ships per-stack agent personas as prose. Nothing verifies each imperative bullet is backed by a real ruleId.

## Where We Want To Be
Per-stack agent docs (the T3 persona prose) plus a T1 validator asserting every must/never bullet cites an existing registry ruleId. Prose is allowed only where it hangs off a real, mechanized rule.

## Requirement Checklist
- [ ] Author per-stack agent docs under `docs/agents/**` (T3 persona layer, clearly labeled advisory prose).
- [ ] Each must/never bullet carries an explicit `[ruleId]` citation.
- [ ] T1 validator parses bullets and asserts each cited id exists in `rules/rules.json` (via d01 registry map).
- [ ] Uncited must/never bullets fail closed; the persona wording itself is not gated.
- [ ] Reverse check optional: flag high-value rules with no agent-doc mention as a T2 advisory.

## Acceptance And Proof
Tier T3 (persona prose) + T1 (citation parity), P1 unit. Prove via `tests/doc-rule-parity.test.mjs`: a bullet citing a real id passes; an uncited or dangling-id bullet fails; the persona free-text is ignored by the gate. Mechanism: markdown bullet parser extracting `[ruleId]` tokens, checked against the registry map from d01.

## Parallel Ownership Notes
Depends on d01 for the registry map. Owns `docs/agents/**` and its parity validator, disjoint from d15 (README) and d14 (skills), so all three run concurrently.
