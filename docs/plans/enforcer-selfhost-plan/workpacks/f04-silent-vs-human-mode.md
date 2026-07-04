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

- owns: `src/run-context/mode.*`, mode signal threading in hooks + mcp entrypoints
- deps: `c04-claude-pretooluse-deny-hook, f01-scan-modes-and-mcp`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
There is no formal distinction between the enforcer running silently under an AI agent versus a human asking for a review. Nothing prevents an agent-inline run from opening the UI or a server. The silent-vs-human doctrine exists only as prose, not as a threaded, testable signal.

## Where We Want To Be
Two formalized run contexts: AGENT-INLINE (silent, STRUCTURED output only, NO UI, no server — used by the c04 deny-hook and by the agent running checks while coding) vs HUMAN-REVIEW (may open the Node-served self-contained UI). A single `mode` flag/env is threaded through the MCP entrypoints and hooks so every code path knows which context it is in, and agent-inline is provably UI-free.

## Requirement Checklist
- [ ] A `RunContext` mode value (`agent-inline` | `human-review`) with one resolution point (flag > env > default `agent-inline`).
- [ ] Mode is threaded through MCP tool invocation and all hooks; deny-hook always runs as `agent-inline`.
- [ ] In `agent-inline`, output is structured only; no UI render, no server start, no popup — enforced, not advisory.
- [ ] Only `human-review` may start the Node/self-contained-HTML UI (loopback+token).
- [ ] Default when unspecified is `agent-inline` (silent).

## Acceptance And Proof
Tier P1. Proof row `run-context-agent-inline-silent` in TEST_PROOF_EXPECTATIONS.md:
- fail-fixture: force a UI/server open under `agent-inline` -> asserts it is refused/never happens (test fails if a listener binds).
- pass-fixture: `human-review` -> UI/server start path is reachable (loopback+token) and returns structured HTML.
- detection test: run the deny-hook and an MCP scan with no mode set -> asserts resolved mode is `agent-inline` AND no server socket/UI artifact is produced.

## Parallel Ownership Notes
Owns `src/run-context/mode.*` and the mode-signal threading only. Depends on c04 (hook consumes the mode) and f01 (MCP threads it). Disjoint from f03 (native-tie mode) — that is a separate axis. Does not own the UI itself, only the gate that permits/forbids it.
