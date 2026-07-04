# b05 Plan Skill And Self Validate

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Plan Skill And Self Validate`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `skills/plan/**, .claude/commands/plan.md, test/plan/skill.*.test.ts`
- deps: `b01-plan-scaffolder, b02-plan-structure-validator, b03-capsule-index-templates`
- tier: `P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Planning is tribal knowledge: the OcentraParent methodology, the capsule contract, and the scaffold/validate loop live in people's heads and in scattered prose. There is no shipped skill or `/plan` command that hands an agent the whole plan-to-make-plans workflow.

## Where We Want To Be
A `plan` skill (and later a `/plan` command) that documents the workflow and wires b01 emit -> b02 validate, and that runs the PLAN-* validator against THIS very plan directory as its own proof.

## Requirement Checklist
- [ ] `skills/plan/SKILL.md` describes the workflow: `plan new` (b01) -> author workpacks -> `plan check` (b02) -> orchestrate (b04).
- [ ] The skill states doctrine: rules are conditions, enforcement is mechanical (T1/T2/T3 ladder), no prose-without-check.
- [ ] `.claude/commands/plan.md` exposes `/plan` invoking scaffolder + validator.
- [ ] The skill's self-check runs b02's PLAN-* validator against `docs/plans/enforcer-selfhost-plan/` and requires zero findings.
- [ ] Skill doc links b01/b02/b03/b04 as the mechanical backing, not as prose to trust.

## Acceptance And Proof
Tier P4 (self-enforce green). Proof: `test/plan/skill.selfvalidate.test.ts` invokes the validator entrypoint against this plan dir and asserts zero PLAN-* findings; `skill.command.test.ts` asserts `/plan` dispatches to the real scaffolder and validator (not a stub). A doc-parity check asserts every doctrine claim in SKILL.md cites a concrete validator/ruleId. Name these in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Last workpack in Track B: depends on b01 (emit), b02 (validate), b03 (templates). owns: (skill dir + `/plan` command + skill tests) is disjoint from all siblings. It consumes their entrypoints read-only and is the integration/self-proof capstone; it blocks nothing.
