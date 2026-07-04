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

- owns: `src/ui/report/*`
- deps: `g01`, `f01`
- tier: `P3`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Scan results land in `.enforce/` as structured data, but there is no human-readable enforcement view. The vendored dashboard renders coordination/ledger state only — nothing shows a developer WHICH rules fired, where, or why.

## Where We Want To Be
A self-contained enforcement REPORT page mounted into the g01 shell (hub dashboard pattern, inline CSS, no framework). It reads `.enforce/` scan results and renders a rule-by-rule VIOLATION MATRIX, groupable by severity / tier / file / crate. Each row shows: the `ruleId`, WHAT it forbids, WHY it matters (the doc anchor), and the offending location (file:line). It is strictly HUMAN-invoked: when f04 silent mode is active (agent running inline checks), the report renders nothing and emits no UI.

## Requirement Checklist
- [ ] `src/ui/report/*` reads `.enforce/` scan output; never re-runs the scanner itself.
- [ ] Renders a violation matrix with grouping by severity, tier, file, and crate.
- [ ] Each row carries `ruleId`, forbidden-behavior text, WHY/doc-anchor link, and file:line location.
- [ ] Output is one self-contained HTML view mounted into g01's registry (no external assets).
- [ ] Honors f04 silent mode: no report render, no UI, during inline agent runs.

## Acceptance And Proof
Tier P3. Fail-fixture: `report-silent-mode-suppressed` (f04 silent active) -> zero UI output emitted. Pass-fixture: `report-matrix-render` (fixture `.enforce/` with mixed severities) -> matrix groups correctly and every row exposes ruleId + why-anchor + location. Detection test: `report-view-contract` asserts each violation row is complete (no missing anchor/location), grouping keys resolve, silent-mode suppression holds, and no external asset is fetched. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `src/ui/report/*` exclusively. Mounts into g01's shell registry (read-only on serve). Depends on f01 for scan-result shape/schema and g01 for the surface. Provides the row surface that g03 attaches per-violation actions to.
