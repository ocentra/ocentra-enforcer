# RM08 — Gap Adjudication and Retirement Contract

<!-- agent-capsule -->
```yaml
id: RM08
owns: "shared capability matrix, disposition ledger, cutover criteria"
deps: "RM02-RM07"
tier: P0
owner: "boss/Sol only"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

Read-only oracle rows may show equality, strictness, legacy-only paths, unavailable paths, or non-comparable evidence.

## Where We Want To Be

Every row is honestly classified `equal`, `stricter`, `not-yet-native`, `legacy-only`, or `intentionally-retired`, with an assigned owner for repair.

## Acceptance And Proof

Serial boss review rejects a `better` verdict without comparable evidence and issues disjoint RM09/RM10 repair bundles only.

## Stop Rules

No repair starts from ambiguous authority, unresolved T1 gap, or private-overlay-derived public result.
