# d13 Rule Version And Drift

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rule Version And Drift`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/rule-version-drift.ts, rules/rule-version-manifest.json, tests/rule-version-drift.test.mjs`
- deps: `d01-rule-mechanization-engine`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
When the enforcer is vendored into a target repo, its config (`ocentra-enforcer.config.json`, `rust-rules.config.json`, `rules/rules.json`) can silently drift from the upstream release. There is a `rule-id-lock.json` for ids but no version+content-hash drift check across the whole rule config.

## Where We Want To Be
A T1 validator that records a version and content hash for the vendored rule config and fails when the deployed config drifts from the pinned manifest without a version bump.

## Requirement Checklist
- [ ] Compute a stable content hash over the rule config set (`rules/rules.json`, `rule-id-lock.json`, config files).
- [ ] Record version + hash in `rules/rule-version-manifest.json`.
- [ ] On run, recompute and compare: hash mismatch without a version bump fails closed.
- [ ] A legitimate version bump requires both a new version and a new hash together (neither alone passes).
- [ ] Drift failure names which config file changed.

## Acceptance And Proof
Tier T1, P1 unit. Prove via `tests/rule-version-drift.test.mjs`: unchanged config passes; content change without version bump fails; matched version+hash bump passes; version bump without content change fails. Mechanism: deterministic hash-over-config compared to the pinned manifest, fail-closed on unexplained drift.

## Parallel Ownership Notes
Depends on d01 (registry shape). Owns the manifest + drift files, disjoint from d11 (CI parity) and all siblings; concurrent.
