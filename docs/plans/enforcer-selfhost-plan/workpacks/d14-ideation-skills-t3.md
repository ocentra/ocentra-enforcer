# d14 Ideation Skills T3

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Ideation Skills T3`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `skills/ideation/devil.md, skills/ideation/think-with-me.md, skills/ideation/README.md, tests/ideation-skills-labeling.test.mjs`
- deps: `none`
- tier: `P0 contract/schema`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The repo ships enforcement skills under `skills/ocentra-enforcer` and `skills/rust-rules-hard-gate`. ADBP includes ideation aids (a devil's-advocate pass, a think-with-me pass) that are inherently non-mechanizable judgment tools.

## Where We Want To Be
Ship the ideation skills as-is under `skills/ideation/`, explicitly LABELED T3 (advisory, no mechanization possible + reason), so they never masquerade as enforcement.

## Requirement Checklist
- [ ] Add `devil` and `think-with-me` skills under `skills/ideation/`.
- [ ] Each carries a mandatory header: `Tier: T3 advisory — no mechanization possible: <reason>`.
- [ ] `skills/ideation/README.md` states these produce no findings and gate nothing.
- [ ] A lint test asserts every file under `skills/ideation/` contains the exact T3 label (fail-closed on an unlabeled ideation skill).
- [ ] These skills are excluded from any enforcement/gating registry.

## Acceptance And Proof
Tier T3 content, but the LABELING is enforced at T1, P0 contract/schema. Prove via `tests/ideation-skills-labeling.test.mjs`: every ideation skill file contains the required T3 label; an unlabeled file fails; the skills appear in no rule registry. Mechanism: label-presence validator over `skills/ideation/**` (the mechanization is on the labeling, not the judgment).

## Parallel Ownership Notes
`deps: none` — pure content plus a labeling gate. Owns `skills/ideation/**` and its labeling test, disjoint from d09 (agent docs) and d15 (README); fully concurrent.
