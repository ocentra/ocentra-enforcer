# g05 Settings Config UI

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Settings Config UI`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/ui/settings/*`
- deps: `g01, c01`
- tier: `P1/P5`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The install core + CLI contract (c01) can write harness config via adapters, and g01 provides the Node-served self-contained HTML shell. But a human still hand-edits JSON to enable CI gates, pre-commit hooks, active profile, and rule severities/waivers — the exact things an AI agent must not silently write.

## Where We Want To Be
A per-project SETTINGS UI, inside the g01 shell, for human-only config: enable/configure CI gates + pre-commit hooks, pick the active profile, per-project tabs, toggle rule severities, and author gated waivers. It writes the real config files through the c-track adapters so nobody hunts JSON and flips switches by hand.

## Requirement Checklist
- [ ] Render settings from live config state (profile, gates, hooks, severities) read through the c01 core; no hardcoded defaults in the view.
- [ ] Writes route ONLY through c-track adapters; the UI never touches config files directly.
- [ ] Waiver authoring writes an EXPLICIT gated waiver (owner+reason+ruleId) to `.enforce/` — never a silent suppression.
- [ ] Toggle operations are idempotent: re-enabling an already-enabled gate produces byte-identical config (no dup hook lines).
- [ ] Loopback+token gated; human-invoked only, no popups.

## Acceptance And Proof
T1 (`settings-config-writes`): fail-fixture — a waiver save missing owner/reason/ruleId is rejected and writes nothing. pass-fixture — toggling a CI gate writes the correct hook/CI config once (temp-dir fixture, config diff matches golden). detection test — re-toggling ON twice yields identical bytes (idempotency assert). Record artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `src/ui/settings/*`. Consumes g01 shell and c01 adapters read-only; disjoint from g04/g06. Config writers here delegate to c-track owners rather than duplicating their logic.
