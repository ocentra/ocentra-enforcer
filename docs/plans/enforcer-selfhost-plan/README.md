# enforcer-selfhost-plan — README (route)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `README (route)`
> Kind: index / route. Read first to find where to go; do not treat as work.
> Read when: You just arrived at this plan and need the entry route.
> Stop rule: Do NOT read every workpack. Follow the Default agent path below and stop.
> Proves: nothing. This file gates no status and is not proof of anything.
> Does not prove: workpack completion, product status, or DONE. Only proof rows do that.
> Proof rule: Product status changes only via a workpack's named tests in TEST_PROOF_EXPECTATIONS.md.
<!-- /agent-capsule -->

This plan makes the `enforcer` **eat its own dog food**. Tracks A/B/C/D/E plus a cross-cutting rename + terminal dogfood gate:

- **A — Self-host (dogfood):** big-bang `.mjs` -> strict TypeScript for all of `src/`, `mcp/`, `tests/`, plus branded domains, parse-at-boundary, waiver honesty, anti-silent-skip, and real self-enforcement in CI.
- **B — Planning skill:** ship the OcentraParent plan methodology as a mechanical `enforcer plan new` scaffolder + `PLAN-*` structure validator + `/plan` skill that self-validates against this very plan.
- **C — Install + enforce anywhere:** harness-neutral install core and adapters (Claude, Codex, generic, stubs) with a **PreToolUse deny-hook** that mechanically blocks T1 violations before a write lands.
- **D — ADBP borrows, mechanized:** every idea borrowed from ADBP is dragged UP the enforcement ladder (grandfather ratchet, deferred-work gate, telemetry, context brake, fix loop, doc-rule parity, plus FSM validity, Rust error handling, security STOP watchlist, change discipline, size/shape caps, test companion/quality, orchestrator verify gates, dispatch prompt assembly, loop resilience, target-repo CI parity), never copied as prose.
- **E — New languages + universal scanning:** an always-on universal literal-scan T2 floor across ~65 languages plus first-class Dart, CFML/ColdFusion, and React/Next (Effect-only) language packs.
- **Cross-cutting:** `x01` renames the product to `enforcer` (early); `z01` is the terminal dogfood-proof-gate that runs the finished `enforcer` against its own now-TS multi-language self and gates plan-DONE on zero self-violations.

## DOCTRINE (governs every workpack)

Rules are conditions. **Enforcement MUST be mechanical.** Prose without a backing check is hope, not proof. Three tiers:

- **T1 — Hard / deterministic validator.** ruleId <-> validator <-> doc <-> fixtures parity, **fail-closed**. Blocks. This is the bar for anything that gates.
- **T2 — Scored / advisory but still mechanical.** regex / AST / heuristic emitting `score` + `confidence`, non-blocking (the Rust literal-scan model). Mechanized, just not fatal.
- **T3 — Justified prose.** Only when mechanization is genuinely impossible, and it MUST be labeled `advisory, no mechanization possible + <reason>`. The *label* is enforced at T1 even when the content is judgment.

Every ADBP borrow is dragged UP this ladder, never left as prose to trust.

## Default agent path

1. Read [`AGENTS.md`](./AGENTS.md) — the operating contract (what you may and may not do).
2. Read [`PLAN_STATE.md`](./PLAN_STATE.md) — scope, resume route, what's present, open gaps.
3. Read [`NEXT_ACTIONS.md`](./NEXT_ACTIONS.md) — the ordered ready-now frontier.
4. Read [`WORKPACK_INDEX.md`](./WORKPACK_INDEX.md) — pick / confirm your one assigned workpack.
5. Read **only that one** workpack under [`workpacks/`](./workpacks/), plus [`TEST_PROOF_EXPECTATIONS.md`](./TEST_PROOF_EXPECTATIONS.md) for its proof rows.

Then do the work, produce the named proof, update that workpack's row. Stop.

## Do not default-read

- Any workpack other than the one assigned to you (there are 114; reading siblings wastes context and risks cross-scope edits).
- [`README_FULL_ORIGINAL.md`](./README_FULL_ORIGINAL.md) — long-form narrative; open only for background, never as a task list.
- [`PLAN_HEALTH.md`](./PLAN_HEALTH.md) — for the hub / auditor, not for a workpack executor.
- [`PLAN_EXECUTION_BLUEPRINT.md`](./PLAN_EXECUTION_BLUEPRINT.md) — for whoever is sequencing/orchestrating, not for a single-pack executor (your capsule already tells you your deps).

## Map of index files

| File | For whom | Purpose |
|---|---|---|
| [`AGENTS.md`](./AGENTS.md) | every agent | operating contract; read order; failure conditions |
| [`PLAN_STATE.md`](./PLAN_STATE.md) | every agent | scope, resume route, present/open gaps, workpack summary |
| [`NEXT_ACTIONS.md`](./NEXT_ACTIONS.md) | executor / hub | the ordered ready-now frontier |
| [`WORKPACK_INDEX.md`](./WORKPACK_INDEX.md) | executor / hub | status table over all workpacks |
| [`PLAN_EXECUTION_BLUEPRINT.md`](./PLAN_EXECUTION_BLUEPRINT.md) | orchestrator | tracks, sequence, parallel model |
| [`TEST_PROOF_EXPECTATIONS.md`](./TEST_PROOF_EXPECTATIONS.md) | every agent | proof tiers P0–P5 + decision tree |
| [`PLAN_HEALTH.md`](./PLAN_HEALTH.md) | hub / auditor | invariants and health checks |
| [`README_FULL_ORIGINAL.md`](./README_FULL_ORIGINAL.md) | background | long-form narrative |
