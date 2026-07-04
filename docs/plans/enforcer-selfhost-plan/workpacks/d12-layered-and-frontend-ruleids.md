# d12 Layered And Frontend RuleIds

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Layered And Frontend RuleIds`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `eslint-rules/no-repo-in-router.js, eslint-rules/no-fetch-in-use-effect.js, eslint-rules/feature-boundaries.js, eslint-rules/str-enum-only.js, eslint-rules/symbol-level-di.js, rules/typescript/layered-frontend.md, tests/fixtures/layered-frontend/**`
- deps: `d01-rule-mechanization-engine`
- tier: `P0 contract/schema`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The repo already has AST-based eslint rules (`eslint-rules/no-app-string-literals.js`, `no-naked-domain-string-types.js`, `no-runtime-string-types.js`) registered in `eslint-rules/index.js`. ADBP's layered/frontend linters exist elsewhere as separate prose-y guidance.

## Where We Want To Be
Fold ADBP's AST linters (no-repo-in-router, no-fetch-in-useEffect, feature-boundaries, StrEnum-only, symbol-level-DI) into our engine as first-class registry ruleIds with fixtures and docs, matching the existing eslint-rule pattern.

## Requirement Checklist
- [ ] Implement each as an AST rule following the existing `eslint-rules/*.js` shape and register in `eslint-rules/index.js`.
- [ ] Mint a registry ruleId per rule via d01 (`rules/rules.json` row + `rule-id-lock.json` entry + doc anchor + pass/fail fixtures).
- [ ] Each rule passes the d01 parity validator (id<->validator<->doc<->fixtures).
- [ ] Doc `rules/typescript/layered-frontend.md` describes each rule with covered-rules anchors.
- [ ] Rules are AST-based (T1 deterministic), not text-heuristic.

## Acceptance And Proof
Tier T1, P0 contract/schema. Prove via `tests/fixtures/layered-frontend/**` (pass + fail per rule) exercised by the eslint-rule tester and the d01 parity oracle. Mechanism: five AST visitors emitting registry-backed findings, each with a fail-fixture that must trip and a pass-fixture that must not.

## Parallel Ownership Notes
Depends on d01 for registry rows/parity. Owns new eslint-rule files + one doc + fixtures, disjoint from the pre-existing string-literal rules and all sibling workpacks.
