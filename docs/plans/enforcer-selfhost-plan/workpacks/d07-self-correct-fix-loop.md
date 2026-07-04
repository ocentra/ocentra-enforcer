# d07 Self-Correct Fix Loop

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Self-Correct Fix Loop`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-coordination/src/fix_loop.rs, crates/enforcer-coordination/src/fix_loop/dispatch.rs, crates/enforcer-coordination/tests/fix_loop.rs, crates/enforcer-coordination/tests/fixtures/fix_loop/**`
- deps: `d01-rule-mechanization-engine, arc-16-enforcer-coordination`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The enforcer only reports; it cannot attempt guided remediation. ADBP describes a "self-correcting" loop as aspiration. `enforcer-coordination` (arc-16) drives multi-agent editing but has no bounded fix-verify-revert mechanism, so any autofix would risk unbounded churn.

## Where We Want To Be
A bounded fix loop module in `enforcer-coordination`: dispatch a fix generator for a `Finding`, re-run the relevant `Validator`, keep the change only if the finding count strictly improves and nothing regresses, else revert; hard iteration cap. Deterministic and panic-free (obeys `[workspace.lints]` — `Result`-based, no `unwrap`/`panic`).

## Requirement Checklist
- [ ] Loop takes a `Finding` set, dispatches a fix generator (pluggable trait), and re-checks with the same `Validator` (via the `enforcer-validator` harness / `enforcer-scan`).
- [ ] Accept a change only if total findings strictly decrease and no new `RuleId` appears (measured via re-scan producing a fresh `Report`, not a model claim).
- [ ] Revert to the prior tree state on non-improvement (deterministic snapshot/restore of the working tree).
- [ ] Hard bound on iterations; the loop always terminates (bounded counter, no unbounded retry).
- [ ] Every accept/revert decision emitted as a typed coordination event (`enforcer-events`) and logged as a d04 telemetry record via the `enforcer-core` NDJSON sink.

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `cargo test -p enforcer-coordination` (`crates/enforcer-coordination/tests/fix_loop.rs`) over `crates/enforcer-coordination/tests/fixtures/fix_loop/**`: an improving fix is kept; a neutral/regressing fix is reverted; the loop halts at the iteration cap; final state never has more findings than the start. Mechanism: a re-scan-and-compare gate wrapping snapshot/restore, verified by before/after `Finding` counts on the fixtures.

## Parallel Ownership Notes
Depends on d01 (validator/parity) and arc-16 for the `enforcer-coordination` crate skeleton (hub/ledger/event spine) it plugs into. Owns only `src/fix_loop.rs`, its `src/fix_loop/dispatch.rs` submodule, and the `tests/fix_loop/**` fixtures inside `enforcer-coordination` — disjoint from the arc-16 skeleton and from d26/d27 (other coordination feature modules) by file. Invoked by d06 `fix`. owns disjoint? = Y (deps arc-16 sequences it after the crate skeleton exists).
