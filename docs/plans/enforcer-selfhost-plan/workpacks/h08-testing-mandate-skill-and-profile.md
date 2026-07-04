# h08 Testing Mandate Skill And Profile

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Testing Mandate Skill And Profile`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `skills/security-testing/SKILL.md`, `src/policy-ingest/*.ts`, `profiles/money-critical-security.json`, `tests/policy-ingest/**`
- deps: `d01`, `b01`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
The money-critical testing mandate has no agent-facing skill and no loadable profile. There is no mechanism to ingest an arbitrary project's security/testing spec and turn it into an enforced profile; the reference spec's §8 doctrine stays prose.

## Where We Want To Be
Three deliverables. (1) A generic SKILL that walks an agent through the mandate: route -> classify (h01) -> required categories (h02) -> threat-map (h03) -> invariants (h05) -> mechanics (h06) -> tooling/CI (h07). (2) A neutral loadable profile `profiles/money-critical-security.json` (NO product/company/game branding) bundling the Track H rule IDs + severities + required-test-categories (§3.1–3.20). (3) POLICY-SPEC-INGESTION: `src/policy-ingest` ingests any project's `.mdc`/spec doc and maps it to a mechanized profile, generalizing "target repo owns policy" — backed rules become enabled rules; un-backed asserted rules are flagged for mechanization and fed to d01/d08.

## Requirement Checklist
- [ ] SKILL sequences the seven Track H stages with routing preconditions.
- [ ] Profile is neutral-named, lists rule IDs + severities (T1 block / T2 score / T3 label) + required-test-categories.
- [ ] T1: ingesting the reference spec yields a profile whose categories + invariants match it.
- [ ] T2: an ingested spec asserting an un-backed rule is flagged for mechanization (no silent accept).
- [ ] No branding anywhere in skill/profile/ingest output.

## Acceptance And Proof
Tier P1. Fixtures: `pass/ingest-reference-spec` -> profile whose required-test-categories + invariants equal the spec's §3 + §2.3 set; `fail/ingest-unbacked-rule` -> a spec asserting a rule with no mechanized backing is flagged (feeds d01/d08), not accepted. Detection test `policy-ingest-mapping.test` + `profile-shape.test` assert mapping equality and the flag path. 5-way parity oracle. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on d01 (mechanization engine) and b01 (plan/profile scaffolder). References h01–h07 rule IDs by string only (does not open them). Owns the skill dir, policy-ingest sources, the profile JSON and its tests exclusively.
