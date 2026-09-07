# RM05 — Coordination and Lifecycle Oracle

<!-- agent-capsule -->
```yaml
id: RM05
owns: "read-only coordination operation rows"
deps: "RM01"
tier: P1
owner: "Luna read-only"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

MJS coordination vendor paths and Rust coordination/MCP lifecycle implementations coexist.

## Where We Want To Be

Claim, guard, mail, ack, release, repair, and closeout preserve fail-closed conflicts and durable wire semantics.

## Acceptance And Proof

One operation per child; compare parallel-worktree conflict, release, recovery, error code, and persisted event scenarios.

## Stop Rules

Stop when a private allowlist changes behavior or same-branch conflicting writes are not denied.
