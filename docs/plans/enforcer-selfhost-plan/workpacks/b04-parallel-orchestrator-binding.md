# b04 Parallel Orchestrator Binding

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Parallel Orchestrator Binding`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-plan/src/orchestrator.rs, crates/enforcer-plan/tests/fixtures/orchestrator/**`
- deps: `arc-20-enforcer-plan, arc-16-enforcer-coordination, b02-plan-structure-validator`
- tier: `P1 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
A validated plan tells you which workpacks may run in parallel (disjoint owns, no dep edge) but nothing turns that into an execution frontier. The `enforcer-coordination` crate (arc-16: hub/lane/claim/guard/closeout/ledger/presence) exists but is not bound to plan structure, so lane assignment is manual. arc-20 stands up the `enforcer-plan` crate skeleton but ships no orchestrator. This pack owns the `src/orchestrator.rs` module in `enforcer-plan` plus its `cargo test` fixtures — it does NOT own either whole crate.

## Where We Want To Be
A deterministic binding in `enforcer-plan` (arc-20): `src/orchestrator.rs` computes the parallel frontier from the validated plan graph and drives it through the `enforcer-coordination` (arc-16) crate API: frontier -> hub lanes -> claim/guard/closeout, with an intent-queue for overlap resolution. It reuses b02's PLAN-PARALLEL-SAFETY predicate (imported, not reimplemented) to keep concurrent lanes disjoint.

## Requirement Checklist
- [ ] Build the dep DAG from workpack `deps:` fields and compute the ready frontier (deps satisfied), over `enforcer-domain` newtypes.
- [ ] Assign frontier workpacks to hub lanes so no two concurrent lanes share owns globs (reuse b02's PLAN-PARALLEL-SAFETY predicate, do not reimplement).
- [ ] **Default each assigned lane to its OWN worktree via arc-16's lane-worktree spawn primitive** (total isolation per EXECUTION_MODEL.md §2b — no shared `Cargo.lock`/`target`/`node_modules` across lanes); do not reuse a single shared tree unless the caller explicitly opts a group of lanes into one (e.g. tightly sequential sub-splits of one workpack).
- [ ] Bind lane lifecycle to the `enforcer-coordination` (arc-16) claim -> guard -> closeout API.
- [ ] Intent-queue serializes any residual owns overlap that slips past static checks (fail-closed: refuse concurrent claim on overlapping owns).
- [ ] Reuse the existing `enforcer-coordination` crate; add no parallel coordination store.
- [ ] **Self-driving loop (owner-set 2026-07-04, lessons L14/L16 — proven live building THIS plan):** the binding exposes a `tick()`-based standing loop that runs UNTIL PLAN-DONE, never a one-shot dispatch. Each tick: (1) drain + ack lane mail, act on blocker flags; (2) liveness-check every in-flight lane (done-mail? branch pushed? worktree activity?) against a staleness threshold; (3) verify-and-integrate DONE lanes (zero-trust: scope diff + independent proof re-run — never trust the done-claim); (4) respawn dead/hung lanes fresh from the integration branch; (5) recompute the frontier and dispatch EVERY newly-ready pack (worker-reuse capped per L11-FILL: ≤2 chained packs per agent); (6) checkpoint state; (7) re-arm the next tick — a tick that would end with in-flight lanes and no scheduled next tick is a TYPED ERROR (idle-without-watchdog is a failure mode, L14). Wake signals compose: event-driven (lane completion) preempts the timer; the timer is the fallback that survives a dead lane. Terminal condition: frontier empty + all packs DONE → hand off to the GATEKEEPER verification (EXECUTION_MODEL §2d three-role gate), not to silence. The b05 `/plan` skill EMITS this loop protocol as part of every generated plan (harness-neutral text + the c02 capability mapping: cron/scheduled-task where the harness has one, self-rescheduling wakeup otherwise, honestly-labeled manual cadence as last resort).
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier P1, T1, Rust-native (`cargo test`). Prove via `cargo test -p enforcer-plan` over `crates/enforcer-plan/tests/fixtures/orchestrator/**`: a frontier test asserts the ready frontier for a fixture plan graph; a lanes test asserts disjoint-owns lane assignment via the reused PLAN-PARALLEL-SAFETY predicate; a claim/guard test uses an `enforcer-coordination` fake/in-memory harness to assert claim/guard/closeout are invoked in order and that overlapping-owns claims are rejected fail-closed. A loop test drives `tick()` over a fixture plan with a simulated dead lane and asserts: the dead lane is detected + respawned, a DONE lane is verified before integration (a tampered done-claim is rejected), the frontier re-dispatches after integration, ending a tick with in-flight lanes and no next tick scheduled returns the typed idle-without-watchdog error, and the empty-frontier/all-DONE terminal hands off to gatekeeper verification. Name these detection tests in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Deps `arc-20-enforcer-plan` (crate skeleton), `arc-16-enforcer-coordination` (the hub/lane/claim/guard/closeout API this binds to), and `b02-plan-structure-validator` (imports its PLAN-PARALLEL-SAFETY predicate and consumes a validated plan graph), so it starts after those land. Owns only `crates/enforcer-plan/src/orchestrator.rs` + `crates/enforcer-plan/tests/fixtures/orchestrator/**`, disjoint by file from b01 (scaffolder module), b02 (validator module), b03 (`templates/` assets), b05 (skill/command), and d25 (verify-gates module) — d25 hosts its own `src/verify_gates.rs` in the same arc-20 crate, a different file, so the two run concurrently. It blocks nothing else in Track B.
