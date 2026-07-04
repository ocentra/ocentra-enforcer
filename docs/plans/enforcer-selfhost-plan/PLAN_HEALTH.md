# PLAN_HEALTH — `enforcer-selfhost-plan`

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `PLAN_HEALTH`
> Kind: index / audit. For the hub and the plan auditor, NOT for a single-pack executor.
> Read when: You are auditing plan integrity or gating a milestone. A lone executor does NOT need this.
> Stop rule: This checks the plan's own invariants; it never authorizes a scope change or a DONE.
> Proves: only that the plan's structure is consistent — never that any workpack is done.
> Does not prove: product status. Proof rows in TEST_PROOF_EXPECTATIONS.md do.
> Proof rule: A health check failing is a stop-the-line signal; do not advance until it clears.
<!-- /agent-capsule -->

This is the plan's own dogfood: the doctrine says enforcement must be mechanical, so the plan's integrity is stated as **invariants a `PLAN-*` validator can check** (see `b02`), not as prose to trust. Until `b02` ships, these are the manual audit checklist; after `b02` ships, they should be its ruleset run against this directory.

## Structural invariants (T1 — fail-closed)

- **PLAN-CAPSULE:** every index doc and every workpack opens with a well-formed `<!-- agent-capsule --> … <!-- /agent-capsule -->` block containing Plan / Doc / Kind / Read-when / Stop-rule / Proves / Proof-rule lines.
- **PLAN-FRONTMATTER:** every workpack declares `owns:`, `deps:`, and `tier:`.
- **PLAN-OWNS-DISJOINT:** the `owns:` glob sets across all workpacks are pairwise disjoint (the single load-bearing invariant behind the parallel model). A shared read-only fixture produced by exactly one pack is allowed and must be declared.
- **PLAN-DEPS-RESOLVE:** every id in a `deps:` list names a real workpack file; no dangling deps.
- **PLAN-DEPS-ACYCLIC:** the dep graph is a DAG; no cycles.
- **PLAN-XLINK:** every cross-link in an index file resolves to a real file (`workpacks/<id>.md`, or a sibling index).
- **PLAN-PROOF-ROW:** every workpack names at least one proof/test row and that row exists in `TEST_PROOF_EXPECTATIONS.md`.
- **PLAN-TIER-VALID:** every `tier:` maps to a defined proof tier (P0–P5) in `TEST_PROOF_EXPECTATIONS.md`.

## Doctrine invariants (T1 on labeling; content may be T1/T2/T3)

- **DOC-MECHANIZED:** every requirement that *gates* is backed by a named validator/test (T1). No blocking claim without a check.
- **DOC-SCORED-LABEL:** every scored/advisory borrow declares itself T2 (score + confidence, non-blocking).
- **DOC-T3-LABEL:** every T3 item carries the exact label `advisory, no mechanization possible + <reason>`. An unlabeled would-be-T3 item fails (this is the `d14` model — the label is enforced even though the judgment is not).
- **DOC-NO-PROSE-GATE:** no workpack claims enforcement while shipping only prose.

## Status-integrity invariants

- **STATUS-PROOF-BACKED:** no `WORKPACK_INDEX.md` row is DONE unless its proof rows are green at its required tier. (Mirrors AGENTS failure condition #1.)
- **STATUS-NO-INFLATION:** no workpack's DONE implies sibling or product DONE. Product status is a separate, explicit roll-up gate.
- **STATUS-DEPS-GREEN:** a DONE pack's deps are all DONE. A pack cannot be DONE ahead of its dependencies.
- **STATUS-NO-SILENT-SKIP:** a validator/self-scan that ran zero checks is red, not green (the `a09`/`a10` rule applied to the plan itself).

## Current health snapshot (2026-07-03)

| Invariant class | State | Note |
|---|---|---|
| Structural (capsule/frontmatter/xlink) | **Manual-pass** | Authored to satisfy; `b02` validator not yet built to prove it mechanically. |
| Owns-disjoint | **Manual-pass** | Verified from `WORKPACK_INDEX.md` owns column; the parallel model depends on this staying true. |
| Deps resolve / acyclic | **Manual-pass** | All `deps:` name real packs; roots are `a01`/`a-conv-01`/`c01`/`d01`/`b01`-`b03`/`a08`/`d14`/`d15`. |
| Doctrine labeling | **Manual-pass** | T3 items (`d14`, `d15`) scoped/labeled as advisory; no prose-gates authored. |
| Status integrity | **N/A** | No pack is DONE yet, so nothing to inflate; enforce these as soon as the first closeout happens. |

### Watch items / risks

- **Self-referential proof:** `b05` and `PLAN_HEALTH` both want the `PLAN-*` validator run against *this* plan dir. Until `b02` ships, structural invariants are audited by hand — treat "manual-pass" as provisional, re-run the validator once it exists.
- **Root contention:** four roots (`a01`, `c01`, `d01`, `b01`) gate most fan-out. If any stalls, its whole track stalls — prioritize roots.
- **`a10` ordering:** flipping on hard-fail self-enforcement before the tree is green would red-line CI. Health gate: do not DONE `a10` unless `a01`+`a09` are DONE and the migrated tree is green.
- **Silent-skip regressions:** any validator that "passes" with zero ran-checks is a health failure regardless of exit code; auditors must inspect ran-counts, not just exit codes.

## How to run the health audit

Manual (now): walk each invariant above against `WORKPACK_INDEX.md`, the workpack frontmatter, and `TEST_PROOF_EXPECTATIONS.md`; any miss is stop-the-line. Mechanical (after `b02`): run the `PLAN-*` validator entrypoint against `docs/plans/enforcer-selfhost-plan/` and require zero findings — this is exactly `b05`'s self-validate proof.
