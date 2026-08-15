# RM06 — Installation and Harness Oracle

<!-- agent-capsule -->
```yaml
id: RM06
owns: "read-only installer and adapter rows"
deps: "RM01"
tier: P1
owner: "Luna read-only"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

Shell/MJS registration paths and Rust installer adapters coexist across harnesses and operating systems.

## Where We Want To Be

Clean-profile install, repair, idempotency, config bytes, and health check are demonstrated per supported adapter/action.

## Acceptance And Proof

One adapter or one installer action per child, with disposable profile evidence on the supported OS.

## Stop Rules

Stop if a config write is mistaken for a working MCP route or if the test writes outside its disposable root.
