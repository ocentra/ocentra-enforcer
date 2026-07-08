# g02 Scan Report Ui

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Scan Report Ui`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-ui/src/report/`
- deps: `g01`, `f01`
- tier: `P3`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
`enforcer-scan` (arc-15) produces a typed `enforcer-domain` `Report` (findings/violations/tiers) persisted under `.enforce/` as versioned serde records, but there is no human-readable enforcement view. The g01 serve surface exposes a view-mount registry and the arc-24 backend derives frontend types from `enforcer-domain` via `ts_rs` — nothing yet shows a developer WHICH rules fired, where, or why.

## Where We Want To Be
An enforcement REPORT module — `crates/enforcer-ui/src/report/` — that renders the `enforcer-domain` `Report` at the Rust boundary and mounts into the g01 view registry (Tauri command + served HTML fallback). It reads the `.enforce/` scan output (typed `Report`, arc-15/f01 shape) and produces a rule-by-rule VIOLATION MATRIX, groupable by severity / tier / file / crate. Each row carries: the `RuleId`, WHAT it forbids, WHY it matters (the doc-anchor from the `enforcer-rules` record), and the offending location (`RelPath`:line). The frontend (TS under `crates/enforcer-ui/frontend/`) only presents; the payload is built in Rust from derived types. Strictly HUMAN-invoked: when f04 silent mode is active (`enforcer-core` run-context), the report renders nothing and emits no UI.

## Requirement Checklist
- [ ] `crates/enforcer-ui/src/report/` reads the `.enforce/` typed `enforcer-domain` `Report` (arc-15/f01 shape); never re-runs the scanner itself.
- [ ] Builds a violation-matrix payload with grouping by severity, tier, file, and crate at the Rust boundary; the frontend only presents.
- [ ] Each row carries `RuleId`, forbidden-behavior text, WHY/doc-anchor (resolved from the `enforcer-rules` record), and `RelPath`:line location.
- [ ] Output mounts into g01's view registry (Tauri command + self-contained served HTML fallback, no external assets); frontend types derived from `enforcer-domain` via `ts_rs`.
- [ ] Honors f04 silent mode (`enforcer-core` run-context): no report render, no UI, during inline agent runs.

## Acceptance And Proof
Tier P3. Fail-fixture: `report-silent-mode-suppressed` (f04 silent active) -> zero UI output emitted. Pass-fixture: `report-matrix-render` (fixture `.enforce/` `Report` with mixed severities) -> matrix groups correctly and every row exposes `RuleId` + why-anchor + location. Detection test: `report-view-contract` (`cargo test -p enforcer-ui`) asserts each violation row is complete (no missing anchor/location), grouping keys resolve, silent-mode suppression holds, and no external asset is fetched. Clean `cargo clippy` / `cargo fmt --check` (obey `[workspace.lints]`). Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `crates/enforcer-ui/src/report/` exclusively. Mounts into g01's view registry (read-only on serve). Depends on f01/arc-15 for the `Report` shape/schema and g01 for the surface. Provides the row surface that g03 attaches per-violation actions to. Deps arc-24 skeleton (via g01); owns stay DISJOINT BY FILE from sibling g0x modules.
