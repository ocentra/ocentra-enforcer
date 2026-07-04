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

- owns: `crates/enforcer-mcp/src/tool_surface.rs, crates/enforcer-core/src/context_budget.rs, crates/enforcer-mcp/context-budget-baseline.json, crates/enforcer-mcp/tests/tool_surface.rs, crates/enforcer-mcp/tests/fixtures/tool_surface/**`
- deps: `arc-21`, `arc-01`, `d01-rule-mechanization-engine`, `d04-run-telemetry-ndjson`
- tier: `P2 CI cross-platform`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The enforcer's MCP surface is consolidated via the router (arc-21 `enforcer-mcp`), but many tools' descriptions still consume agent context. Nothing measures or caps this surface. ADBP's "context is a budget" is an idea, not a check. arc-21 stands up the `enforcer-mcp` crate skeleton (transport + tool registry + router); this pack owns the `src/tool_surface.rs` measure module inside it plus a ratchet in `enforcer-core` (`src/context_budget.rs`) and the committed baseline — it does NOT own the whole MCP or core crate.

## Where We Want To Be
A measure over the enforcer's own Rust tool registry (tool count + total description bytes/tokens) and a fail-closed ratchet in `enforcer-core` that compares the measured surface against a committed baseline in CI. The measurement is enumerated in-process from the consolidated `enforcer-mcp` tool registry (no `.mjs` — the registry is the Rust router's typed tool set), and the measured surface is recorded into the d04 `RunRecord` telemetry.

## Requirement Checklist
- [ ] Enumerate registered tools from the `enforcer-mcp` Rust tool registry (the router's consolidated tool set) and sum description size (bytes + token estimate) in `src/tool_surface.rs`.
- [ ] Record the measured surface into the d04 telemetry `RunRecord` per run (reuse the `enforcer-core` NDJSON sink; do not duplicate it).
- [ ] Compare against `crates/enforcer-mcp/context-budget-baseline.json` via the `enforcer-core::context_budget` ratchet; growth beyond a set tolerance is a CI failure (T1 hard ratchet, fail-closed).
- [ ] Emit a T2 scored signal (surface-per-tool efficiency, with confidence in [0,1]) that is advisory, non-blocking.
- [ ] Baseline is updatable only by an explicit, reviewed commit (declarative committed policy), never silently; obey `[workspace.lints]` (no `unwrap/expect/panic/print_*`).

## Acceptance And Proof
Tier T1 (hard ratchet) + T2 (advisory score), proven at P2 CI cross-platform. Prove via `cargo test -p enforcer-mcp` (`crates/enforcer-mcp/tests/tool_surface.rs` with `tests/fixtures/tool_surface/**`: measurement determinism; a simulated surface-growth fixture fails the ratchet; the T2 score is in [0,1] with confidence) plus a CI job (via `enforcer-harness` arc-18 run-adapter) invoking the measure against the committed baseline on win/mac/linux. Mechanism: static enumeration of the Rust tool registry + byte/token count diffed against the committed baseline. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Deps `arc-21` (owns the `enforcer-mcp` crate skeleton — transport/registry/router), `arc-01` (owns the `enforcer-core` skeleton), `d01-rule-mechanization-engine` (engine/parity), and `d04-run-telemetry-ndjson` (telemetry sink it records into). Owns only `enforcer-mcp/src/tool_surface.rs` + its baseline/tests and `enforcer-core/src/context_budget.rs`, disjoint by file from the arc-21/arc-01 skeletons and from d11 (CI parity) — it shares a CI stage with d11 but no files. Coordinate the `mod`/`pub` lines appended to `enforcer-mcp/src/lib.rs` and `enforcer-core/src/lib.rs` with the arc skeleton owners.
