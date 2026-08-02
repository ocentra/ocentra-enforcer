# Rust/MJS Parity Retirement — Plan State

<!-- agent-capsule -->
```yaml
planId: rust-mjs-parity-retirement-plan
state: "RM00 authority is accepted; RM01 read-only capability inventory is ready."
integrator: "boss on rust-build"
terminalRule: "no MJS fallback; delete legacy runtime paths rather than merging or retaining them"
```
<!-- /agent-capsule -->

## Where We Are

The repository contains both MJS and native Rust enforcement paths. Some native schema and fixture oracles exist, but their presence does not prove public runtime, MCP, install, hook, CI, or coordination parity. RM00 records the split authority and required aggregate contract in [`authority/RM00_AUTHORITY.json`](authority/RM00_AUTHORITY.json). No native cutover is authorized.

## Where We Want To Be

All registered public mechanical capability rows are independently reproduced at one candidate SHA; local CLI, MCP, install, hooks, CI, and release select native execution; rollback is to a previous native release; legacy MJS runtime code is deleted, not merged or kept as a fallback.

## Dependency Checklist

1. RM00 accepted: exact public, provenance, overlay, dogfood, and aggregate authority are frozen by SHA.
2. RM01 ready: create the canonical capability matrix from bounded read-only audits.
3. RM02-RM07 establish read-only public oracles.
4. RM08 adjudicates gaps; RM09-RM10 repair only approved disjoint gaps.
5. RM11 aggregates exact-SHA evidence, then RM12-RM14 control cutover and deletion.
