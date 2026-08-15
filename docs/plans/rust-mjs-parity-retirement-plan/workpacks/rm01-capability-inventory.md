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

Boss-owned working artifacts:

- [`../inventory/RM01_CAPABILITIES.schema.json`](../inventory/RM01_CAPABILITIES.schema.json) defines the singleton row contract.
- [`../inventory/RM01_CAPABILITIES.json`](../inventory/RM01_CAPABILITIES.json) records current proposals and explicit coverage omissions.

RM01 is accepted only when the matrix represents every public CLI/check, all 50 canonical MCP tools and their 50 compatibility aliases, all 570 registered public rule IDs, and the coordination/install/hook/CI/dogfood/release surfaces. `inventoryState: incomplete`, grouped rows, or any unexpanded surface keeps RM02-RM07 blocked. Acceptance means the source inventory partition is complete; its `source-inventory-only` and `unmeasured` rows remain explicitly unproved and are the inputs that RM02-RM07 must measure. RM01 acceptance never promotes behavioral parity.

## Stop Rules

Do not classify from prose alone. Only the boss edits the matrix/schema singleton.
