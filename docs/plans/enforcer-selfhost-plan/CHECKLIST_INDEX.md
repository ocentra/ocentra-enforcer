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
- [ ] For crate-build (arc-*) packs: the crate compiles standalone and opts into the deny wall via `[lints] workspace = true`; keep `unsafe_code = forbid`; add **no re-export barrels** (`pub use`/`pub(crate) use`) — import concrete module paths (enforced as an `enforcer-lang-rust` Validator); an arc pack owns only its crate SKELETON, never a sibling feature's `src/rules/**` / `src/adapters/**` / `src/hooks/**`.
- [ ] For brand/schema packs (`enforcer-domain` newtypes): parse-at-boundary via serde in the owning module, and add the negative test that proves a bare primitive (e.g. `String`) cannot substitute for the branded newtype.
- [ ] For validators (T1): make it **fail-closed** on missing/invalid input; add pass + fail fixtures; ensure ruleId<->validator<->doc<->fixture parity (via d01 once it exists).
- [ ] For scored checks (T2): emit `score in [0,1]` + a `confidence`, and never change exit code.
- [ ] For T3 content: add the exact `advisory, no mechanization possible: <reason>` label and the mechanical label gate; never claim a check you did not build.
- [ ] For `enforcer-domain` DTOs consumed by the UI: derive `#[derive(ts_rs::TS)]` — never hand-write the `.ts`; the fail-closed drift test is what proves the committed types match.
- [ ] Work stays inside `C:/Projects/ocentra-enforcer` (the `../enforcer-rust` worktree during the build); do not touch anything outside the workpack's owns.

---

## C. Closeout checklist (before DONE)

Run before flipping a WORKPACK_INDEX status to `DONE`. This is the doctrine gate.

- [ ] Confirm the proof tier via the decision tree in [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md) section 3. If the workpack's declared tier disagrees, resolve first.
- [ ] The **named test/oracle** for this workpack's proof row passes on the migrated tree.
- [ ] For **T1 / P4 / P5**: the **seeded-violation case** is demonstrated to FAIL (a gate that never trips proves nothing).
- [ ] For **T2**: assert `score in [0,1]`, confidence present, exit code unchanged.
- [ ] For **T3**: the label gate passes (the prose itself is not trusted).
- [ ] For **crate-build (arc-*) packs**: `cargo test -p <crate>` is green, and `cargo clippy` / `cargo fmt --check` / `cargo deny` / `cargo audit` are clean over the crate — the deny wall (`unsafe_code=forbid` + clippy denies) holds and the no-reexports Validator finds zero barrels in the owned crate.
- [ ] For **`enforcer-domain` schema changes**: the `ts_rs` drift test passes (committed generated `.ts` == freshly-emitted); do not commit hand-edited types.
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
- [ ] Respect the global sequence: a01 (Rust toolchain) -> arc-01/arc-02 (workspace core + domain) -> rest of the arc crate swarm (incl. arc-25 `enforcer-events`) -> A domain packs (a02..a09) -> D01 -> feature packs (C/D/E/F/G/H) in parallel -> B last -> a10.
- [ ] Do not read workpack bodies; you route, lanes execute.

---

## Where the checklists are mechanically backed

These lists are procedure; the doctrine is that procedure must be *checkable*:

- Claim/parallel-safety -> **b02** PLAN-PARALLEL-SAFETY rule + coordination MCP guard.
- Proof-before-DONE -> **TEST_PROOF_EXPECTATIONS** GREEN rows (the only DONE authority).
- Crate-build invariants -> per-crate `cargo test -p enforcer-<crate>` + clippy/fmt/deny/audit clean (arc-01..arc-25 proof rows); the `[workspace.lints]` deny wall (unsafe_code=forbid + clippy denies) lands in **a01**.
- No re-export barrels -> **enforcer-lang-rust** `no-reexports` Validator (syn-AST, structured Findings); also shipped as a T1 rule for consumer repos.
- Two-layer redaction (key-name + value-pattern) -> folded into **enforcer-core** (no `enforcer-log` crate).
- Hash-chain proof journal -> append-only SHA-256 hash-chained NDJSON in **enforcer-proof** (verify-on-open + on-replay).
- Rust->TS drift -> **enforcer-domain** `ts_rs` derive + fail-closed drift test (arc-02 / g05 `ts_drift` proof rows).
- Disjoint-owns -> **b02** PLAN-PARALLEL-SAFETY rule + coordination MCP guard.
- Doc/capsule structure -> **b02** PLAN-CAPSULE / PLAN-SKELETON / PLAN-FRONTMATTER rules.
- T3 labeling -> **d14** label gate; doc-rule citations -> **d09** parity.
