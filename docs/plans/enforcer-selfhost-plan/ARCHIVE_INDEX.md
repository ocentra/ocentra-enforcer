# ARCHIVE_INDEX

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Archive Index`
> Kind: archive register. Records superseded / historical plan docs and the rule for archiving. Keeps the live plan surface small so agents don't read stale material.
> Read when: You are chasing why a decision changed, OR you are about to supersede/retire a plan doc and need the archiving procedure. Almost never needed to execute current work.
> Stop rule: Nothing here is live. Do NOT read archived material to plan or execute current work — use the live indexes. Archived content proves nothing about the current plan.
> Proves: nothing. It is a history register.
> Does not prove: any current status, proof, or completion.
<!-- /agent-capsule -->

Sources: [DOC_INDEX](./DOC_INDEX.md), [ROUTE_INDEX](./ROUTE_INDEX.md).

---

## Current archive contents

**Empty.** This plan is newly authored; no doc has been superseded yet. The live surface is the nine root docs listed in [DOC_INDEX.md](./DOC_INDEX.md) (including the governing [RUST_ARCHITECTURE.md](./RUST_ARCHITECTURE.md)) plus the 118 workpacks under `workpacks/`.

> Note (Track A Rust re-cast): the 50-pack `.mjs -> TypeScript` conversion swarm was **removed, not archived** — it was specification-only (no code, no proof rows GREEN, no dependents DONE) and is superseded by the `arc-01`..`arc-25` Rust crate-build swarm per [RUST_ARCHITECTURE.md](./RUST_ARCHITECTURE.md). No `archive/` entry is created because nothing was ever built against it; the successor packs (`arc-*`) carry the same scope in Rust. Track G's former coordination deps have already re-homed to the `arc-16` `enforcer-coordination` crate: `g04`/`g06` now dep `arc-16` and own `crates/enforcer-ui/…` Rust paths (the Rust re-frame is DONE, not deferred).

| Archived doc | Superseded by | Date archived | Reason |
|--------------|---------------|---------------|--------|
| _(none yet)_ | | | |

---

## What belongs here (and what does not)

Archive:
- A **root plan doc** that a newer doc fully replaces (e.g. a re-cut PLAN_EXECUTION_BLUEPRINT).
- A **workpack** that is retired, merged into another, or split into replacements — record the successor id(s).
- A superseded **proof approach** where the tier/oracle changed and the old row would mislead a reviewer.

Do NOT archive:
- A workpack that is merely `DONE` — DONE is a live status in WORKPACK_INDEX, not an archive event.
- Source, tests, or fixtures — those live in the repo tree, governed by their owning workpack, never here.
- Anything still routed by ROUTE_INDEX / referenced by a live workpack's deps.

---

## Archiving procedure

1. Create `archive/` under this plan dir if it does not exist; move the retired file there **unchanged** (preserve its capsule).
2. Add a row to the table above: doc, superseding doc(s), date (ISO), one-line reason.
3. Remove the doc's route from [ROUTE_INDEX.md](./ROUTE_INDEX.md) and its row from [DOC_INDEX.md](./DOC_INDEX.md) (or the WORKPACK_INDEX row for a workpack), so the live surface no longer points at it.
4. If a workpack is retired, update every dependent's `deps:` to the successor id(s) and re-check PLAN-PARALLEL-SAFETY (owns must stay disjoint). Once b02 exists, re-run the PLAN-* validator to confirm zero findings.
5. If the retired doc carried a proof row, mark that row in [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md) as `ARCHIVED -> <successor>` rather than deleting it, so proof history stays traceable.

---

## Integrity rule

Archiving must never leave a dangling link on the live surface, and must never silently drop a proof obligation. A retired obligation is **re-homed to a successor**, not deleted — same doctrine as the code: no silent skips. Once the plan-structure validator (b02) is live, a link-integrity + orphan-deps check over this plan dir is the mechanical backing for this rule.
