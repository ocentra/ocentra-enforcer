# RM02 — CLI and Scanner Oracle

<!-- agent-capsule -->
```yaml
id: RM02
owns: "read-only CLI/scanner inventory rows"
deps: "RM01"
tier: P1
owner: "Luna read-only"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

MJS CLI/scanner paths include `scripts/ocentra-enforcer.mjs`, `src/cli-*.mjs`, and generic scanners; Rust candidates live in CLI/scan crates.

## Where We Want To Be

Comparable command rows prove scope, exit, ordered normalized findings, and unavailable-tool behavior at one target SHA.

## Acceptance And Proof

Run at most 15 commands per child against shared fixtures. Capture both raw artifacts and normalized comparison.

## Stop Rules

Schema or compile success is not runtime parity. Stop on different target SHA, nondeterminism, or unexplained finding delta.
