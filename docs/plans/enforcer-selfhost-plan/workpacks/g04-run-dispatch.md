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

- owns: `crates/enforcer-ui/src/run_dispatch/`
- deps: `g02, arc-16`
- tier: `P5`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The g02 report module renders human-invoked findings but has no bridge from a finding to a fix. The coordination hub (the `enforcer-coordination` crate, arc-16) already has the ledger primitives for claim/lane/task and emits typed coordination events via `enforcer-events` (arc-25), but nothing translates a report row into agent work.

## Where We Want To Be
The report's "Run" button emits a SCHEMA-DRIVEN fix config and dispatches an agent (Claude/Codex) via the hub: it writes a typed `FixIntent` to the ledger through the arc-16 API. The agent picks it up through MCP (`enforcer-mcp` `coordination_claim`), claims the exact files, guards, fixes, then closes out. This ties the human report to the agent swarm on existing coordination primitives — no new transport, no silent side effects; the lifecycle is observable via the arc-25 event spine.

## Requirement Checklist
- [ ] Define a strict `FixIntent` serde newtype (`RuleId`, target `RelPath`s, profile, human-actor, reason) in the module, validated parse-at-boundary (typed `thiserror`); reject malformed intents fail-closed.
- [ ] "Run" click (Tauri command / served-fallback endpoint) serializes exactly the selected finding(s) into a `FixIntent` and writes it to the ledger via the arc-16 (`enforcer-coordination`) API (no direct fs writes).
- [ ] Dispatch is loopback+token gated (arc-24/g01 serve gate); no popups — the UI is human-invoked only (`enforcer-core` run-context).
- [ ] Intent is idempotent: re-clicking Run for an already-open intent dedupes on `RuleId`+files, never forks a duplicate lane.
- [ ] Closeout state from the ledger reflects back into the report row (open/claimed/fixed), driven by arc-16 state / arc-25 events.

## Acceptance And Proof
Tier P5 (`run-dispatch-intent`, T1): fail-fixture — a Run payload missing `RuleId`/files must be rejected (boundary schema error, zero ledger writes). pass-fixture — a valid Run click writes one well-formed `FixIntent` the agent can claim (assert ledger entry shape + claimable by MCP `coordination_claim`). detection test (`cargo test -p enforcer-ui`) — a duplicate Run dedupes to a single intent. Clean `cargo clippy` / `cargo fmt --check` (obey `[workspace.lints]`). Record artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-ui/src/run_dispatch/`. Consumes the g02 report module and the arc-16 coordination facade read-only; does not modify the arc-24 crate skeleton or arc-16 coordination files. Deps g02 (rows) + arc-16 (ledger/dispatch) and, via g02, the arc-24 skeleton; owns stay DISJOINT BY FILE from sibling g0x modules.
