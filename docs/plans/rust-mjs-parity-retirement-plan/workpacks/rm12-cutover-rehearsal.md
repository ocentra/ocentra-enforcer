# RM12 — Native Cutover Rehearsal and Rollback Proof

<!-- agent-capsule -->
```yaml
id: RM12
owns: "non-production release candidate and disposable configurations"
deps: "RM11"
tier: P0
owner: "boss/Sol only"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

Aggregate parity does not prove installation, selection, and recovery in a clean environment.

## Where We Want To Be

A clean multi-platform rehearsal selects native CLI/MCP/hooks/CI and proves rollback to a prior native release.

## Acceptance And Proof

Use disposable clones/profiles, force a controlled failure, capture before/after configuration, and independently verify the restored native version.

## Stop Rules

Stop if rehearsal changes production state or rollback requires reviving MJS.
