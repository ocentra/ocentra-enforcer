# d01 Rule Mechanization Engine

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rule Mechanization Engine`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-mechanization/src/scaffold.rs, crates/enforcer-mechanization/src/parity.rs, crates/enforcer-mechanization/tests/scaffold.rs, crates/enforcer-mechanization/tests/parity.rs, crates/enforcer-mechanization/tests/fixtures/scaffold/**, crates/enforcer-mechanization/tests/fixtures/parity/**`
- deps: `arc-14`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Adding a rule today means hand-editing the rule registry, minting an id, writing a validator, a doc section, and pass/fail fixtures — with nothing checking they agree. ADBP describes "rule packs" only as prose. In the Rust engine, rules are STRUCTURED DATA: typed rule records in `enforcer-rules` (arc-04), each carrying `ruleId <-> validator <-> {fail+pass fixtures} <-> doc-anchor <-> tier`. arc-14 stands up the `enforcer-mechanization` crate SKELETON (`Cargo.toml`, `src/lib.rs`, module root); this pack owns the two feature modules inside it — the scaffolder (`src/scaffold.rs`) and the fail-closed parity oracle (`src/parity.rs`) — plus their `cargo test` fixtures. There is no scaffolder and no single parity oracle yet.

## Where We Want To Be
An `enforcer rule new <ID>` subcommand (wired through `enforcer-cli`, driven by `enforcer-mechanization::scaffold`) scaffolds all five artifacts in lockstep — a typed rule record in `enforcer-rules`, a `Validator` impl stub (built on the `enforcer-validator` trait, arc-05), a doc anchor, and pass+fail fixtures — plus a hard `parity` oracle (an `enforcer-validator` check emitting `Finding`s) that fails closed on any `RuleId <-> Validator <-> doc-anchor <-> {fail,pass} fixtures <-> registry-record` mismatch. This is the keystone every other Track D borrow rides.

## Requirement Checklist
- [ ] `scaffold` emits: a typed rule record into `enforcer-rules` (arc-04), a `Validator` impl stub against the arc-05 trait, a resolvable `doc#anchor`, and both a pass and a fail fixture under `crates/enforcer-mechanization/tests/fixtures/scaffold/<id>/{good,bad}/`.
- [ ] `parity` asserts every registry `RuleId` (branded newtype, `enforcer-domain`) has a firing `Validator`, a `doc#anchor` that resolves, and the required pass/fail fixtures per the record's `requiresPassFixture`/`requiresFailFixture` flags; the oracle emits structured `Finding`s, never a `println`/`exit` binary (obey `[workspace.lints]` — no `unwrap/expect/panic/print_*`).
- [ ] Parity is fail-closed: an unknown validator, a dangling doc anchor, or a missing fixture returns an error `Finding`, not a warning; a rule is only accepted if its validator fires on the fail fixture and is silent on the pass fixture (reuse the `enforcer-validator` fixture/parity harness).
- [ ] Reverse direction checked: no orphan validator/doc/fixture without a registry record.
- [ ] Scaffolder output re-validates green under the parity oracle (round-trip: scaffold -> load record -> parity passes).

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `crates/enforcer-mechanization/tests/parity.rs` (`cargo test -p enforcer-mechanization` — parity across the live rule registry) and `crates/enforcer-mechanization/tests/scaffold.rs` (scaffold a temp rule into a `tempdir`, assert the five artifacts exist and re-pass parity), with fail/pass fixtures under `crates/enforcer-mechanization/tests/fixtures/{scaffold,parity}/**`. Named oracle: the `parity` `Validator` in `enforcer-mechanization`, also invocable from the CLI. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Keystone of Track D: d02/d03/d04/d05 (and d06/d07/d08/d12/d13) build on this scaffolder + parity oracle. Deps `arc-14` (which owns the `enforcer-mechanization` crate skeleton — `Cargo.toml`/`lib.rs`/module root — and transitively arc-01/02/04/05 for core/domain/rules/validator). This pack owns only the two feature files `src/{scaffold,parity}.rs` + their `tests/fixtures/{scaffold,parity}/**` (disjoint by file from the arc-14 skeleton and from every sibling), so it can start as soon as the skeleton exists while siblings scaffold their own rule modules.
