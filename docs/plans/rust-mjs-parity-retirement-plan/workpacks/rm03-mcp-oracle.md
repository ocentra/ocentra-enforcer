# RM03 — MCP Behavioral Oracle

<!-- agent-capsule -->
```yaml
id: RM03
owns: "read-only MCP tool rows"
deps: "RM01"
tier: P1
owner: "Luna read-only"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

MJS registry/runner/transport modules coexist with Rust MCP registry/router. Existing schema tests are only one oracle dimension.

## Where We Want To Be

Every public tool has valid/invalid input, response/error, framing, side-effect, and unavailable behavior evidence. Native-only UI extensions are explicit, not hidden exceptions.

## Acceptance And Proof

Audit no more than 10 tools per child using the same request corpus and normalized JSON responses.

## Stop Rules

Do not call schema equality behavioral parity; stop if production Rust delegates to MJS or a tool is unconnected.
