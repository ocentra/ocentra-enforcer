# d10 Resilience Auditor

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Resilience Auditor`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-common/src/rules/resilience.rs, crates/enforcer-lang-common/tests/fixtures/resilience/**`
- deps: `d01-rule-mechanization-engine, d04-run-telemetry-ndjson, arc-09-enforcer-lang-common`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Nothing adversarially probes a change for missing failure-mode coverage. ADBP describes a "red team / resilience" reviewer as narrative. `enforcer-lang-common` (arc-09) holds the cross-language validator family but has no resilience `Validator` turning adversarial review into required-test obligations.

## Where We Want To Be
A resilience `Validator` module in `enforcer-lang-common` whose output is mechanized twofold: it emits required-test obligations (T1 `Finding`s) and T2 failure-mode "smell" scores (e.g. unhandled error path, missing timeout, unbounded retry). Registered in the crate's common rule set, built on the `enforcer-validator` trait.

## Requirement Checklist
- [ ] The resilience pass enumerates candidate failure modes for the changed surface (AST/structural analysis over the target code).
- [ ] Each accepted failure mode becomes a required-test obligation (T1) that must be satisfied by a matching test before review passes, surfaced as a `Finding`.
- [ ] Emit T2 smell `Finding`s (score + confidence in `[0.0, 1.0]`) for heuristically-detected failure-mode gaps; non-blocking (advisory severity).
- [ ] Required-test obligations reference real `RuleId`s + fail/pass fixtures via d01 parity; smells consume d04 telemetry records (from the `enforcer-core` NDJSON sink) for trend.
- [ ] Missing a required test fails closed (blocking `Finding`); smells never block. Obey `[workspace.lints]` (no `unwrap`/`panic`/`print_*`).

## Acceptance And Proof
Tier T1 (required-test obligations) + T2 (failure-mode smells), P1 unit. Prove via `cargo test -p enforcer-lang-common` over `crates/enforcer-lang-common/tests/fixtures/resilience/{bad,good}/**`: an unmet required-test obligation fires a `Finding` (fail fixture); a met one is silent (pass fixture); smell scores land in `[0.0, 1.0]` with a confidence and no gating effect. Mechanism: an obligation table (T1) plus a `syn`/tree-sitter heuristic scorer (T2), wired through the `enforcer-validator` parity harness and asserted against fixtures.

## Parallel Ownership Notes
Depends on d01 (rule/fixture parity), d04 (telemetry records), and arc-09 for the `enforcer-lang-common` crate skeleton (module root + validator registration) it plugs into. Owns only `src/rules/resilience.rs` and `tests/fixtures/resilience/**` inside `enforcer-lang-common` — disjoint from the arc-09 skeleton and from sibling common-family rule modules (d03/d16/d21/d22/d23) by file. Consumed by d06 `review`. owns disjoint? = Y (deps arc-09 sequences it after the crate skeleton exists).
