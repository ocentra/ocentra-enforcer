# g06 Hub Coordination Dashboard

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Hub Coordination Dashboard`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/ui/hub/*`
- deps: `g01, a-conv-20, a-conv-23`
- tier: `P3`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The vendored hub UI (`src/coordination/vendor/dashboard.js`, 18KB self-contained HTML; `vendor/server.js`, 13.5KB Node http) renders coordination/ledger state but is buried behind a `coordination ledger:dashboard` command and lives outside the enforcer UI shell. Materialized views (a-conv-20) and the coordination API (a-conv-23) already expose the live state.

## Where We Want To Be
The hub dashboard — presence, lanes, claims, tasks, workers, mail, sync — surfaced into the SAME g01 UI shell as report+settings, so coordination + enforcement + config live under one local `enforcer ui`. Reuse `vendor/dashboard.js`'s HTML (do not rewrite from scratch); wire it to live materialized ledger state via the a-conv-23 API.

## Requirement Checklist
- [ ] Mount the vendored dashboard HTML as a tab/panel inside the g01 shell; reuse its markup + inline CSS, no framework, no binary.
- [ ] Bind panels (presence/lanes/claims/tasks/workers/mail/sync) to live materialized state via a-conv-23, not the buried standalone server.
- [ ] Single Node-served origin: one loopback+token gate shared with report/settings; retire the separate `ledger:dashboard` entry once mounted.
- [ ] Read-only surface — the dashboard reflects ledger state; it does not mutate claims/lanes (mutations remain g04's Run path).
- [ ] Empty/degraded ledger renders a stable empty state, never a crash.

## Acceptance And Proof
T1 (`hub-dashboard-mount`): fail-fixture — a corrupt/missing materialized view must render the empty state (no throw, HTTP 200 with placeholder). pass-fixture — a seeded ledger with one lane + one claim renders those exact rows in the mounted panel (assert served HTML contains lane/claim ids). detection test — the dashboard issues zero mutating calls against the coordination API (spy asserts read-only). Record artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `src/ui/hub/*`. Reuses vendored `src/coordination/vendor/dashboard.js` read-only (does not edit vendor); consumes g01 shell and a-conv-20/a-conv-23 read-only. Disjoint from g04/g05.
