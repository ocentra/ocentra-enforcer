# d03 Deferred Work Gate

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Deferred Work Gate`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-common/src/rules/deferred_work.rs, crates/enforcer-lang-common/tests/fixtures/deferred_work/**`
- deps: `arc-09`, `d01-rule-mechanization-engine`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Stubs and TODO/FIXME/`unimplemented!`/`todo!`/`raise NotImplementedError`/`throw new Error("not implemented")` markers leak into merged code with no gate. ADBP treats "no silent deferral" as prose advice. arc-09 stands up the `enforcer-lang-common` crate skeleton (the cross-language/common validator family). This pack owns the `src/rules/deferred_work.rs` module (one `Validator` impl in that family, keyed to a `RuleId` in `enforcer-rules`) plus its fixtures — it does NOT own the whole crate.

## Where We Want To Be
A diff-scoped `Validator` that hard-fails any newly introduced stub/deferral marker unless it carries an explicit, structured `DEFERRED(#ref)[revisit:<date-or-milestone>]` annotation, emitting an `enforcer-domain` `Finding` per unmarked/malformed marker. Being in `enforcer-lang-common`, it detects markers across all supported target languages (Rust/TS/Py/Dart/CFML/etc. — this validates USER code, it is not the engine's own language).

## Requirement Checklist
- [ ] Detect a fixed vocabulary of deferral markers per target language (TODO, FIXME, stub throws, `unimplemented!`, `todo!`, `raise NotImplementedError`, `pass  # TODO`) — string/marker scan over scanned files, wired through the `enforcer-validator` trait (arc-05).
- [ ] Exempt only markers matching the exact `DEFERRED(#<ref>)[revisit:<value>]` grammar (a parse-at-boundary parser, typed error); malformed annotations still emit an error `Finding`.
- [ ] Diff-scoped: only lines added/changed in the working diff are gated, so legacy stubs do not block (composes with d02 baseline). Diff scope comes from the `enforcer-scan` run context.
- [ ] `#<ref>` must be non-empty; `revisit:` value must be non-empty.
- [ ] Emitted as a first-class rule record via d01 (record + fixtures + doc-anchor + `Validator` parity); obey `[workspace.lints]` (no `unwrap/expect/panic/print_*`).

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `cargo test -p enforcer-lang-common` over `crates/enforcer-lang-common/tests/fixtures/deferred_work/**`: unmarked stub in added lines fails; correctly annotated stub passes; malformed annotation fails; legacy stub outside the diff passes (fail-fixture `bad/`, pass-fixture `good/`). Mechanism: marker scan intersected with diff hunks, structured-annotation parser as the only escape hatch. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Deps `arc-09` (owns the `enforcer-lang-common` crate skeleton — `Cargo.toml`/`lib.rs`/family module root/`Validator` registration) and `d01-rule-mechanization-engine` (for `RuleId`/fixture parity). Owns only `src/rules/deferred_work.rs` + `tests/fixtures/deferred_work/**`, disjoint by file from the arc-09 skeleton, from other common-family rule modules, and from d02/d04/d05. Composes with d02 (diff-scoping vs baseline) but shares no files, so it runs concurrently with d02/d04 once arc-09 and d01 land.
