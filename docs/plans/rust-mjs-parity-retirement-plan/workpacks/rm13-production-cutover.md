# RM13 — Observed Production Cutover

<!-- agent-capsule -->
```yaml
id: RM13
owns: "production selectors, installer defaults, release/cutover journal"
deps: "RM12"
tier: P0
owner: "boss/Sol only"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

Only a rehearsal candidate has been proven; production selection remains unchanged.

## Where We Want To Be

Native execution is selected atomically for supported public routes and observed with fresh required CI and installed-session evidence.

## Acceptance And Proof

Record exact release SHA, configuration before/after, required CI, real session status, observation journal, and independent reproduction.

## Stop Rules

Stop if any consumer lacks native proof, evidence is stale, authority changes, or a live MJS fallback is proposed.
