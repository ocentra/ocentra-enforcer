# d05 Context Budget Brake

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Context Budget Brake`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/context-budget.ts, scripts/context-budget-scan.mjs, proof/context-budget-baseline.json, tests/context-budget.test.mjs`
- deps: `d01-rule-mechanization-engine, d04-run-telemetry-ndjson`
- tier: `P2 CI cross-platform`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The enforcer exposes ~60 MCP tools (see the deferred tool list under `mcp__ocentra-enforcer__*` / `rust_rules_*`) whose descriptions consume agent context. Nothing measures or caps this surface. ADBP's "context is a budget" is an idea, not a check.

## Where We Want To Be
A validator that measures the enforcer's own MCP tool-description surface (tool count + total description bytes/tokens) and ratchets it against a committed baseline in CI.

## Requirement Checklist
- [ ] Enumerate registered MCP tools from `mcp/ocentra-enforcer-mcp.mjs` / `mcp/rust-rules-mcp*.mjs` registries and sum description size.
- [ ] Record measured surface into d04 telemetry per run.
- [ ] Compare against `proof/context-budget-baseline.json`; growth beyond a set tolerance is a CI failure (T1 ratchet).
- [ ] Emit a T2 score (surface-per-tool efficiency, with confidence) that is advisory, non-blocking.
- [ ] Baseline is updatable only by an explicit, reviewed commit, not silently.

## Acceptance And Proof
Tier T1 (hard ratchet) + T2 (advisory score), proven at P2 CI cross-platform. Prove via `tests/context-budget.test.mjs` (measurement determinism; simulated growth fails the ratchet; T2 score in [0,1] with confidence) and a CI job invoking `scripts/context-budget-scan.mjs`. Mechanism: static enumeration of the tool registry + byte/token count diffed against committed baseline.

## Parallel Ownership Notes
Depends on d01 (engine) and d04 (telemetry sink). Owns its own scan/baseline/test files, disjoint from d11 (CI parity) which it shares a CI stage with but not files.
