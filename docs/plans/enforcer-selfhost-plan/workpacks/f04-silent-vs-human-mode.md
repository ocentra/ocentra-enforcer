# f04 Silent Vs Human Mode

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Silent Vs Human Mode`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-core/src/run_context.rs`, `crates/enforcer-core/tests/run_context.rs`, `crates/enforcer-core/tests/fixtures/run_context/**`
- deps: `arc-01-enforcer-core, c04-claude-pretooluse-deny-hook, f01-scan-modes-and-mcp`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
There is no formal distinction between the enforcer running silently under an AI agent versus a human asking for a review. Nothing prevents an agent-inline run from opening the Tauri UI or the served-HTML server. The silent-vs-human doctrine exists only as prose, not as a threaded, testable signal. The arc-01 `enforcer-core` foundation holds shared primitives but no `run_context` type.

## Where We Want To Be
Two formalized run contexts in `crates/enforcer-core/src/run_context.rs`: `AgentInline` (silent, STRUCTURED output only, NO UI, no server — used by the c04 deny-hook and by the agent running checks while coding) vs `HumanReview` (may open the Tauri desktop UI or the served self-contained HTML fallback — presentation only, per RUST_ARCHITECTURE). A single `RunContext` value (typed enum, one resolution point) is threaded through the MCP entrypoints (`enforcer-mcp`) and hooks so every code path knows which context it is in, and agent-inline is provably UI-free.

## Requirement Checklist
- [ ] A `RunContext` enum (`AgentInline` | `HumanReview`) in `enforcer-core` with one resolution point (flag > env > default `AgentInline`), parsed at boundary with a typed error on an invalid value.
- [ ] The value is threaded through MCP tool invocation and all hooks; the c04 deny-hook always runs as `AgentInline`.
- [ ] In `AgentInline`, output is structured only; no UI render, no server start, no popup — enforced at the type/gate level, not advisory.
- [ ] Only `HumanReview` may start the Tauri UI / served self-contained-HTML surface (loopback + token). No business logic in that surface; it is presentation only.
- [ ] Default when unspecified is `AgentInline` (silent). Obeys `[workspace.lints]` (no `unwrap`/`expect`/`print_*` outside the one sanctioned output sink).

## Acceptance And Proof
Tier P1. Proof row `run-context-agent-inline-silent` in TEST_PROOF_EXPECTATIONS.md asserts `cargo test -p enforcer-core --test run_context` exits 0:
- fail-fixture: force a UI/server open under `AgentInline` -> asserts it is refused/never happens (test fails if a listener binds).
- pass-fixture: `HumanReview` -> the UI/server start path is reachable (loopback + token) and returns structured HTML.
- detection test: resolve with no mode set (deny-hook path + an MCP scan) -> asserts resolved context is `AgentInline` AND no server socket/UI artifact is produced.

## Parallel Ownership Notes
Owns ONLY `crates/enforcer-core/src/run_context.rs` + its `tests/run_context.rs` and `tests/fixtures/run_context/**` — disjoint files inside the arc-01 crate, which owns the crate skeleton (`Cargo.toml`, `lib.rs`, core primitives). Deps arc-01 (skeleton), c04 (hook consumes the context), and f01 (MCP threads it). Disjoint from f03 (native-tie mode) — that is a separate axis. Does not own the UI itself (Track G / `enforcer-ui`), only the gate that permits/forbids it. `owns disjoint? = Y`.
