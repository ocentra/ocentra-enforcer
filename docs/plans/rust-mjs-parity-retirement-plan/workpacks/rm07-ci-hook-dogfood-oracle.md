# RM07 — CI, Hook, Dogfood, and Release Oracle

<!-- agent-capsule -->
```yaml
id: RM07
owns: "read-only CI/hook/dogfood rows"
deps: "RM01"
tier: P1
owner: "Luna read-only"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

Current workflows and scripts still use MJS for planning, local parity, frozen scanning, and release paths.

## Where We Want To Be

Each local command maps to a required CI step and a seeded violation demonstrably fails that step.

## Acceptance And Proof

One workflow/job path per child; record event/ref conditions, command, required aggregation, and failure injection.

## Stop Rules

Stop on frozen-pin drift, docs-only masking of source failure, or native build-only jobs that do not enforce semantics.
