# g04 Run Dispatch

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Run Dispatch`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/ui/run-dispatch/*`
- deps: `g02, a-conv-23, a-conv-24`
- tier: `P5`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The g02 report UI renders human-invoked findings but has no bridge from a finding to a fix. The coordination hub (`src/coordination/api.mjs`, a-conv-23; runner, a-conv-24) already has the ledger primitives for claim/lane/task, but nothing translates a report row into agent work.

## Where We Want To Be
The report's "Run" button emits a SCHEMA-DRIVEN fix config and dispatches an agent (Claude/Codex) via the hub: it writes a typed `fix-intent` to the ledger. The agent picks it up through MCP, claims the exact files, guards, fixes, then closes out. This ties the human report to the agent swarm on existing coordination primitives — no new transport, no silent side effects.

## Requirement Checklist
- [ ] Define a strict `fix-intent` schema (ruleId, target files, profile, human-actor, reason) validated at the boundary; reject malformed intents fail-closed.
- [ ] "Run" click serializes exactly the selected finding(s) into a fix-intent and writes it to the ledger via the a-conv-23 API (no direct fs writes).
- [ ] Dispatch is loopback+token gated (hub pattern); no popups — the UI is human-invoked only.
- [ ] Intent is idempotent: re-clicking Run for an already-open intent dedupes on ruleId+files, never forks a duplicate lane.
- [ ] Closeout state from the ledger reflects back into the report row (open/claimed/fixed).

## Acceptance And Proof
T1 (`run-dispatch-intent`): fail-fixture — a Run payload missing ruleId/files must be rejected (schema error, zero ledger writes). pass-fixture — a valid Run click writes one well-formed fix-intent the agent can claim (assert ledger entry shape + claimable by MCP `coordination_claim`). detection test — a duplicate Run dedupes to a single intent. Record artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `src/ui/run-dispatch/*`. Consumes the g02 report shell and the a-conv-23/a-conv-24 coordination facade read-only; does not modify vendor or coordination files.
