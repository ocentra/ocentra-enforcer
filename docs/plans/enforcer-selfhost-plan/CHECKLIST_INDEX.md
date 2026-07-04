# CHECKLIST_INDEX

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Checklist Index`
> Kind: repeatable gates. The claim / execute / close / author checklists every agent runs at the corresponding moment. Copy the relevant list; do not skip steps.
> Read when: You are about to CLAIM a workpack, EXECUTE one, CLOSE one out, or AUTHOR a new workpack/doc.
> Stop rule: These are procedure gates, not proof. Running a checklist does not make a workpack DONE — only a GREEN proof row does. Do not use these lists to justify skipping TEST_PROOF_EXPECTATIONS.
> Proves: nothing on its own. It sequences the actions that lead to a GREEN proof row.
> Does not prove: completion or status.
<!-- /agent-capsule -->

Sources: [WORKPACK_INDEX](./WORKPACK_INDEX.md), [TEST_PROOF_EXPECTATIONS](./TEST_PROOF_EXPECTATIONS.md), [PLAN_EXECUTION_BLUEPRINT](./PLAN_EXECUTION_BLUEPRINT.md), [PROOF_INDEX](./PROOF_INDEX.md).

---

## A. Claim checklist (before you start a workpack)

Run before flipping a WORKPACK_INDEX status to `CLAIMED`.

- [ ] The workpack's **deps** (from WORKPACK_INDEX / the workpack frontmatter) are all `DONE`. If not, it is not on the frontier — pick another.
- [ ] Its `owns:` globs are **disjoint** from every currently-`CLAIMED`/`IN-PROGRESS` workpack (PLAN-PARALLEL-SAFETY). Check the "owns disjoint?" column; if `Y*`, honor the stated sequencing (e.g. a05 before a02 on the fingerprint file) or take the intent-queue.
- [ ] Claim the lane via the coordination MCP: `coordination_claim` for this workpack's owns set (the parallel model in PLAN_EXECUTION_BLUEPRINT binds frontier -> hub lane -> claim). A rejected claim means overlap — resolve, do not force.
- [ ] You have read this workpack's capsule + body and NOTHING else under `workpacks/` (no sibling reads).
- [ ] You know the workpack's declared tier and have found its row in [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md).
- [ ] Set the WORKPACK_INDEX status to `CLAIMED` (then `IN-PROGRESS` when you begin editing).

---

## B. Execution checklist (while doing the work)

- [ ] Edit **only** files inside this workpack's `owns:` globs. Touching a sibling's file is a parallel-safety violation — stop and coordinate.
- [ ] Hold the coordination guard (`coordination_guard`) for the duration; it is the mechanical backstop against a concurrent overlapping write.
- [ ] For `.mjs -> TS` conversions: drop every `import * as` wildcard (use named imports); keep the module's public surface; if the file is a SPLIT target, split by responsibility with no barrel wildcards and preserve every existing test case.
- [ ] For brand/schema packs: mint the brand **only** at the boundary (the owning module), and add the `tsc --noEmit` negative fixture that proves a bare `string` cannot substitute.
- [ ] For validators (T1): make it **fail-closed** on missing/invalid input; add pass + fail fixtures; ensure ruleId<->validator<->doc<->fixture parity (via d01 once it exists).
- [ ] For scored checks (T2): emit `score in [0,1]` + a `confidence`, and never change exit code.
- [ ] For T3 content: add the exact `advisory, no mechanization possible: <reason>` label and the mechanical label gate; never claim a check you did not build.
- [ ] Work stays inside `C:/Projects/ocentra-enforcer`; do not touch anything outside the workpack's owns.

---

## C. Closeout checklist (before DONE)

Run before flipping a WORKPACK_INDEX status to `DONE`. This is the doctrine gate.

- [ ] Confirm the proof tier via the decision tree in [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md) section 3. If the workpack's declared tier disagrees, resolve first.
- [ ] The **named test/oracle** for this workpack's proof row passes on the migrated tree.
- [ ] For **T1 / P4 / P5**: the **seeded-violation case** is demonstrated to FAIL (a gate that never trips proves nothing).
- [ ] For **T2**: assert `score in [0,1]`, confidence present, exit code unchanged.
- [ ] For **T3**: the label gate passes (the prose itself is not trusted).
- [ ] For **conversions**: scoped `tsc --noEmit` over owned files exits 0 under strict; `grep 'import *'` over owned files is empty; SPLIT exports == original surface.
- [ ] Record the artifact path in the proof row and flip that row's Status to `GREEN` in TEST_PROOF_EXPECTATIONS.md.
- [ ] Release the lane and close out via `coordination_closeout`.
- [ ] Only now set the WORKPACK_INDEX status to `DONE`. Do not move product/plan status beyond this one workpack's scope (per the workpack stop rule).

---

## D. Plan-author checklist (adding a workpack or plan doc)

- [ ] The new file opens with the exact `<!-- agent-capsule -->` block (Plan/Doc/Kind/Read-when/Stop-rule/Proves/Does-not-prove [+ Proof-rule if proof-bearing]).
- [ ] A workpack carries `owns:` / `deps:` / `tier:` frontmatter; tier is drawn from the P0-P5 set (with T-tier noted if scored/advisory).
- [ ] The workpack body has the required headings in order: Where We Are, Where We Want To Be, Requirement Checklist, Acceptance And Proof, Parallel Ownership Notes.
- [ ] `owns:` globs are disjoint from every workpack that has no dep edge to this one (PLAN-PARALLEL-SAFETY). If a shared file is unavoidable, mark it `Y*` and state the sequencing.
- [ ] Register the workpack in [WORKPACK_INDEX.md](./WORKPACK_INDEX.md) (row with owns/tier/parallel-safe-with) and add its proof row to [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md).
- [ ] Every provable claim names a validator/test, or is labeled T3 with a reason. No prose-only proof.
- [ ] Once b02 exists, run the PLAN-* structure validator against the plan dir and get zero findings (self-enforce green).

---

## E. Hub / orchestrator checklist (per wave)

- [ ] Compute the ready **frontier**: workpacks whose deps are all DONE (see PLAN_EXECUTION_BLUEPRINT frontier model; once b04 exists, it computes this from the validated plan graph).
- [ ] Assign frontier workpacks to **hub lanes** such that no two concurrent lanes share `owns:` globs (reuse the PLAN-PARALLEL-SAFETY predicate; do not reimplement).
- [ ] Drive each lane through `coordination_claim` -> `coordination_guard` -> `coordination_closeout`.
- [ ] Route any residual owns-overlap through the **intent-queue** (serialize; fail-closed refuse concurrent claim on overlapping owns).
- [ ] Respect the global sequence: a01 -> conversion swarm -> A domain packs -> D01 -> rest of D + C in parallel -> B last.
- [ ] Do not read workpack bodies; you route, lanes execute.

---

## Where the checklists are mechanically backed

These lists are procedure; the doctrine is that procedure must be *checkable*:

- Claim/parallel-safety -> **b02** PLAN-PARALLEL-SAFETY rule + coordination MCP guard.
- Proof-before-DONE -> **TEST_PROOF_EXPECTATIONS** GREEN rows (the only DONE authority).
- Conversion invariants -> per-pack scoped typecheck + `import *` grep (a-conv proof rows).
- Doc/capsule structure -> **b02** PLAN-CAPSULE / PLAN-SKELETON / PLAN-FRONTMATTER rules.
- T3 labeling -> **d14** label gate; doc-rule citations -> **d09** parity.
