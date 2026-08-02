# RM01 — Machine-Readable Capability Inventory

<!-- agent-capsule -->
```yaml
id: RM01
owns: "shared parity capability matrix/schema"
deps: "RM00"
tier: P0
owner: "boss integrator; Luna read-only row audits"
```
<!-- /agent-capsule -->

> Plan: rust-mjs-parity-retirement-plan

## Where We Are

CLI, scanner, MCP, configuration, routing, rules, proof, coordination, install, hook, CI, dogfood, and release capabilities are distributed across MJS and Rust.

## Where We Want To Be

Each public capability has one ID and names its MJS/native entrypoints, fixture, scope, exit/diagnostic/evidence contract, CI caller, authority, and status.

## Acceptance And Proof

Audit no more than 40 rows per child; integrator rejects missing entrypoints, duplicate IDs, or unproved `supported` status.

## Stop Rules

Do not classify from prose alone. Only the boss edits the matrix/schema singleton.
