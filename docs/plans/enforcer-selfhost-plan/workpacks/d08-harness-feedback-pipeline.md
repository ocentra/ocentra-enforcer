# d08 Harness Feedback Pipeline

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Harness Feedback Pipeline`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-mechanization/src/feedback.rs, crates/enforcer-mechanization/src/feedback/classify.rs, crates/enforcer-mechanization/tests/feedback.rs, crates/enforcer-mechanization/tests/fixtures/feedback/**`
- deps: `d01-rule-mechanization-engine, arc-14-enforcer-mechanization`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
When a harness surfaces an escaped defect, the lesson dies in a chat log. `enforcer-harness` (arc-18) parses native-tool output into `enforcer-domain` diagnostics but never turns a failure into a candidate rule, and `enforcer-mechanization` (arc-14, the d01 scaffolder) is not yet fed by those failures. ADBP's "close the loop" is prose.

## Where We Want To Be
A feedback module in `enforcer-mechanization` that ingests harness failure diagnostics, classifies each as preventable (could have been a static `Validator`) vs detect-only, and for preventable ones auto-scaffolds a PROPOSED rule record via the d01 scaffolder in the same crate.

## Requirement Checklist
- [ ] Ingest structured harness failures as typed `enforcer-domain` diagnostics (reuse the `enforcer-harness` parse output shapes).
- [ ] Classify each failure into `prevent` vs `detect` via explicit signal rules encoded in Rust (mechanical match on diagnostic fields, not vibe).
- [ ] For `prevent`, call the d01 scaffolder (this crate's `scaffold`) to emit a `Validator` stub + doc anchor + fail/pass fixtures in an `enforcer-rules` record with a machine-readable `status = Proposed` (a `Tier`/status field on the rule record, not prose).
- [ ] PROPOSED rules do not gate builds until reviewed/promoted; the status is a typed enum in the registry so the scan engine can skip them.
- [ ] Classification decisions logged as d04 telemetry records (versioned serde struct in `enforcer-domain`) carrying the input `Sha256` fingerprint, appended via the `enforcer-core` NDJSON sink.

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `cargo test -p enforcer-mechanization` (`crates/enforcer-mechanization/tests/feedback.rs`) over `crates/enforcer-mechanization/tests/fixtures/feedback/**`: a preventable failure produces a PROPOSED registry record + fixtures that pass the d01 parity oracle; a detect-only failure produces none; PROPOSED rules are non-blocking in a scan. Mechanism: a classifier over parsed failure fields feeding the d01 scaffolder, asserted by the resulting registry state.

## Parallel Ownership Notes
Depends on d01 (the scaffolder + parity oracle in this same crate) and arc-14 for the `enforcer-mechanization` crate skeleton. Owns only `src/feedback.rs`, its `src/feedback/classify.rs` submodule, and the `tests/fixtures/feedback/**` fixtures inside `enforcer-mechanization` — disjoint from d01's `scaffold.rs`/`parity.rs` and the arc-14 skeleton by file, sequenced after them. Feeds d10 (auditor). owns disjoint? = Y (deps d01 + arc-14 sequence it after the scaffolder and crate skeleton exist).
