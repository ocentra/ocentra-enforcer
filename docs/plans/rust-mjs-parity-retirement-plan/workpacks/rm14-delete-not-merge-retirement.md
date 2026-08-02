# RM14 — Delete-Not-Merge Legacy Retirement

<!-- agent-capsule -->
```yaml
id: RM14
owns: "legacy runtime MJS entrypoints, wrappers, dependencies, CI references, retirement manifest"
deps: "RM13"
tier: P0
owner: "boss/Sol only"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

MJS remains only as the frozen comparison oracle until production observation closes.

## Where We Want To Be

No executable MJS enforcement path remains in local, MCP, hook, CI, release, or install selection. Historical evidence is immutable and non-executable.

## Acceptance And Proof

Delete legacy runtime code and references; run a deny-list scan for executable MJS invocation, clean install without Node, strict verification, mutation-risk, workspace CI, dogfood, and exact-SHA independent reproduction.

## Stop Rules

Stop if any live caller remains or deletion removes an unclosed capability. Do not merge the private overlay, preserve a runtime MJS fallback, or lower a rule to manufacture closure.
