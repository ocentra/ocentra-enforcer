# b02 Plan Structure Validator

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Plan Structure Validator`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/plan/validator/**, src/rules/plan/**, test/plan/validator.*.test.ts, test/fixtures/plan-validator/**`
- deps: `none`
- tier: `P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The skeleton and capsule rules exist only as prose in this plan's authoring instructions. Nothing mechanically checks that a workpack has the capsule, owns/deps/tier, or that no-dep-edge workpacks have disjoint owns globs. Prose without a backing check is hope, not proof.

## Where We Want To Be
A hard, fail-closed validator exposing PLAN-* rules that can be pointed at any plan dir — including THIS one — and deterministically reports every structural violation.

## Requirement Checklist
- [ ] PLAN-CAPSULE: every workpack contains the exact agent-capsule marker block, unmodified fields.
- [ ] PLAN-SKELETON: required headings present in order (Where We Are, Where We Want To Be, Requirement Checklist, Acceptance And Proof, Parallel Ownership Notes).
- [ ] PLAN-FRONTMATTER: owns/deps/tier lines present and well-formed; tier in the P0-P5 set.
- [ ] PLAN-PARALLEL-SAFETY: for any two workpacks with no dep edge between them, their owns globs MUST be disjoint (overlap => finding).
- [ ] Each rule has ruleId<->validator<->doc<->fixture parity (pass + fail fixtures) and fails closed on missing input.

## Acceptance And Proof
Tier T1 / P4. Proof: `test/plan/validator.rules.test.ts` drives each PLAN-* rule against pass/fail fixtures in `test/fixtures/plan-validator/**`; `validator.selfhost.test.ts` runs the validator against `docs/plans/enforcer-selfhost-plan/` and asserts zero findings (self-enforce green). A parity test asserts every PLAN-* ruleId maps to a validator, a doc row, and both fixtures. Name these in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Consumes b01's golden fixture read-only for a positive case; otherwise independent. owns: (validator src + plan rules + own fixtures) is disjoint from b01 (scaffolder src), b03 (template text), b04 (orchestrator binding). b05 invokes this validator against the live plan dir but adds no source here.
