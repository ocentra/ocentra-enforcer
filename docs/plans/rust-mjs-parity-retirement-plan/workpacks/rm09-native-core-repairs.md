# RM09 — Native Core Repair Packets

<!-- agent-capsule -->
```yaml
id: RM09
owns: "one boss-assigned non-singleton CLI/scan/rule/proof capability plus tests"
deps: "RM08"
tier: P2
owner: "Luna only after boss acceptance"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

RM08 may identify discrete core behavioral gaps.

## Where We Want To Be

One native capability family closes without semantic weakening or shared-surface drift.

## Acceptance And Proof

Limit a child to one family, at most eight methods, one crate plus dedicated tests; prove positive/negative fixtures, diagnostics, exit, scoped gate, and exact claimed diff.

## Stop Rules

Stop before touching matrix/schema, workflow, manifest, global router, or broad compatibility shim; escalate instead.
