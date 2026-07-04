# AGENTS.md — operating contract for `enforcer-selfhost-plan`

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `AGENTS operating contract`
> Kind: contract / index. Binding on every agent that touches this plan.
> Read when: Immediately after README, before touching any workpack.
> Stop rule: You execute ONE assigned workpack. Do not open siblings. Do not move product status without proof.
> Proves: nothing itself. It defines how proof is produced and what DONE requires.
> Does not prove: any workpack. Only that workpack's named tests + proof rows do.
> Proof rule: No DONE without the workpack's named tests green and its proof rows updated.
<!-- /agent-capsule -->

This is the binding contract. If anything below conflicts with a single workpack's prose, **this contract and the DOCTRINE win**.

The engine is **pure Rust** — a Cargo workspace of 28 crates (see [RUST_ARCHITECTURE.md](./RUST_ARCHITECTURE.md)). The only TypeScript is the `enforcer-ui` Tauri/web frontend. Proof is `cargo test -p <crate>` + fail/pass fixtures + `clippy`/`fmt --check`/`deny`/`audit` — **never** `tsc`/`jest`/typecheck. There is no `.mjs`/TS-engine/Effect-Schema/eslint-as-our-linter residue in what you build. Execution follows [EXECUTION_MODEL.md](./EXECUTION_MODEL.md): a bootstrap-safe worktree, an orchestrator, and per-workpack workers.

## Read order (do this, in this order, then stop)

1. **`EXECUTION_MODEL.md`** — how the build runs (bootstrap-safe worktree, orchestrator + worker swarm, coordination hub). Sets the operating frame.
2. **`PLAN_STATE.md`** — scope, resume route, what is present, open gaps. Orients you.
3. **`NEXT_ACTIONS.md`** — the ordered ready-now frontier. Confirms what is claimable.
4. **`WORKPACK_INDEX.md`** — the status table (107 workpacks). Locate your single assigned workpack, confirm its Track, `owns` disjointness, and tier.
5. **The ONE assigned workpack** under `workpacks/`, plus its rows in **`TEST_PROOF_EXPECTATIONS.md`**.

Do not read sibling workpacks. Do not read the full narrative to "get context." Your capsule + these files are sufficient.

## Orchestration model (per EXECUTION_MODEL.md)

- **Bootstrap-safe worktree.** All crate work happens in a SEPARATE git worktree + branch (`git worktree add ../enforcer-rust -b rust-engine`). `main` and the installed `.mjs` MCP stay pinned and live throughout the rebuild; the MCP is swapped to the Rust binary only after the `z01` dogfood gate is green. Workers operate on ABSOLUTE paths inside the one shared `../enforcer-rust` worktree — no per-agent ephemeral worktrees.
- **Orchestrator.** A high-capability model (Fable 5) drives. It reads `WORKPACK_INDEX.md`, takes the dependency-free frontier of disjoint-`owns:` workpacks, and spawns workers. It does not hand-write crate code beyond trivial glue.
- **Workers.** One worker per disjoint workpack — **Sonnet / Haiku / Opus**, reasoning matched to difficulty (Haiku for mechanical, Sonnet for standard crate builds, Opus for hard validator/parity/security work). Within a workpack, further disjoint sub-units may run in parallel.
- **Coordination hub.** Each worker takes a lane (`<agent>-<wp>`), `claim`s its `owns:` globs, `guard`s before write, posts `intent` on any unavoidable overlap (intent-queue serializes via mail), and `closeout`s when its proof is green. Disjoint-`owns:` + no dep edge ⇒ safe parallel by construction. If the coordination MCP refuses writes as stale (`writeCompatible:false`), restart/reload it (or use the `ocentra_enforcer_run` CLI fallback) before claiming.
- **Consumer contract.** BOTH MCP and CLI are first-class surfaces of the `enforcer` binary; neither is secondary.

## DOCTRINE you must uphold (T1 / T2 / T3)

Rules are conditions; **enforcement MUST be mechanical**. Prose without a backing check is hope, not proof.

- **T1 — hard/deterministic validator**, fail-closed, with ruleId <-> validator <-> doc <-> fixtures parity. Anything that *blocks* must be T1.
- **T2 — scored/advisory but still mechanical** (regex/AST/heuristic emitting score + confidence, non-blocking — the Rust literal-scan model).
- **T3 — justified prose**, only when mechanization is impossible, and it MUST carry the label `advisory, no mechanization possible + <reason>`. Even then, the *presence of the label* is enforced at T1.

If your workpack's requirement can be mechanized, mechanize it. Dragging an ADBP borrow "up the ladder" is the job — never copy prose and call it enforcement.

## What you MAY do

- Edit only the files inside your workpack's `owns:` glob set.
- Produce the named tests/artifacts listed in that workpack's Acceptance And Proof.
- Update that one workpack's row in `WORKPACK_INDEX.md` and its rows in `TEST_PROOF_EXPECTATIONS.md`.
- Under a parallel run: claim your lane, set a guard over your `owns:` set, close out when proof is green (see `PLAN_EXECUTION_BLUEPRINT.md`).

## What you MUST NOT do

- Touch any file outside your `owns:` set. If you need a change in another pack's scope, file it via the intent-queue; do not edit across the boundary.
- Open or "fix" sibling workpacks.
- Move product / plan status on the strength of prose, a green `cargo build`/`cargo check` alone, or a partial run.
- Weaken a gate to make it pass. Waivers (see `a08`) are the only sanctioned exception, and they must be honest and named.
- Ship a rule/borrow as prose when it could be a T1/T2 check.

## Failure conditions (a run is INVALID if any hold)

1. **No DONE without proof.** A workpack marked DONE whose named tests in `TEST_PROOF_EXPECTATIONS.md` are not green, or whose proof rows are not updated, is invalid — revert the status.
2. **No DONE without tests.** A workpack that ships behavior but no test/validator backing its requirement (where mechanization is possible) is invalid.
3. **Silent skip.** A validator that "passed" by running zero checks is a failure, not a pass (see `a09`). A hollow self-scan must fail, not stay green (see `a10`).
4. **Cross-scope edit.** Any change outside the claimed `owns:` set invalidates the run.
5. **Prose masquerading as a check.** A T3-labeled item without the mandatory `advisory, no mechanization possible + <reason>` label, or a claim of enforcement with no backing validator, is invalid.
6. **Status inflation.** Marking product/PR readiness from one workpack's local proof. A workpack proves only its local scope, never sibling completion.

## DONE definition (per workpack)

A workpack is DONE only when **all** hold:

- Every item in its Requirement Checklist is satisfied.
- Its named tests/validators in `TEST_PROOF_EXPECTATIONS.md` are green at the required proof tier (P0–P5) for its type — proven by `cargo test -p <crate>` + fail/pass fixtures + `clippy`/`fmt --check`/`deny`/`audit`, not a typecheck.
- Its proof rows are updated with the real artifact paths / exit codes.
- Its `owns:` set is the only thing changed; disjointness held.
- Its row in `WORKPACK_INDEX.md` is flipped to DONE with the proof reference.

Anything short of this stays `IN PROGRESS` or `BLOCKED`. When in doubt, do not mark DONE.
