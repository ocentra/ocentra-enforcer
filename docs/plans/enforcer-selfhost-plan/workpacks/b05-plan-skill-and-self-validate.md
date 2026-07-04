# b05 Plan Skill And Self Validate

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Plan Skill And Self Validate`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `skills/plan/**, crates/enforcer-install/src/commands/plan.rs, crates/enforcer-plan/tests/self_validate.rs`
- deps: `arc-20-enforcer-plan, b01-plan-scaffolder, b02-plan-structure-validator, b03-capsule-index-templates`
- tier: `P4 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Planning is tribal knowledge: the OcentraParent methodology, the capsule contract, and the scaffold/validate loop live in people's heads and in scattered prose. There is no shipped skill or `/plan` command that hands an agent the whole plan-to-make-plans workflow, and nothing runs the PLAN-* validator against this very plan directory. b01 (scaffolder), b02 (validator), and b03 (templates) provide the mechanical backing; this capstone ties them into a skill + harness command + a self-validate gate. It owns the prose skill, the installer's `/plan` command emitter, and the self-validate `cargo test` — it does NOT own any validator or scaffolder source.

## Where We Want To Be
A `plan` skill under `skills/plan/**` that documents the workflow and wires b01 emit -> b02 validate -> b04 orchestrate; a harness `/plan` command emitted by the `enforcer-install` installer (a `commands/plan.rs` emitter that writes `.claude/commands/plan.md` for a Claude target, harness-neutral for others); and a self-validate `cargo test` (`crates/enforcer-plan/tests/self_validate.rs`) that runs b02's PLAN-* `Validator` against THIS very plan directory and asserts zero `Finding`s.

## Requirement Checklist
- [ ] `skills/plan/SKILL.md` describes the workflow: `enforcer plan new` (b01) -> author workpacks -> `enforcer plan check` (b02) -> orchestrate (b04).
- [ ] The skill states doctrine: rules are conditions, enforcement is mechanical (T1/T2/T3 ladder), no prose-without-check.
- [ ] The `enforcer-install` `commands/plan.rs` emitter exposes `/plan` (writing `.claude/commands/plan.md` for a Claude target; tool-neutral for other harnesses) invoking the scaffolder + validator through the `enforcer` binary — never a hand-written per-harness hook.
- [ ] The self-validate `cargo test` runs b02's PLAN-* `Validator` against `docs/plans/enforcer-selfhost-plan/` and requires zero `Finding`s.
- [ ] Skill doc links b01/b02/b03/b04 as the mechanical backing (concrete `ruleId`s / `Validator` entrypoints), not as prose to trust.
- [ ] Obeys `[workspace.lints]` for the Rust it owns (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier P4 (self-enforce green), T1, Rust-native (`cargo test`). Prove via `cargo test -p enforcer-plan` (self-validate) and `cargo test -p enforcer-install` (command emitter): the self-validate test (`crates/enforcer-plan/tests/self_validate.rs`) invokes the b02 `Validator` entrypoint against this plan dir and asserts zero PLAN-* `Finding`s; a command-emitter test asserts the `commands/plan.rs` emitter produces a `/plan` command that dispatches to the real scaffolder + validator via the `enforcer` binary (not a stub); a doc-parity check asserts every doctrine claim in `SKILL.md` cites a concrete `Validator`/`ruleId`. Name these detection tests in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Last workpack in Track B (capstone): deps `arc-20-enforcer-plan` (the crate whose `Validator` the self-validate test drives) and `b01-plan-scaffolder` (emit) + `b02-plan-structure-validator` (validate) + `b03-capsule-index-templates` (templates). Owns the `skills/plan/**` prose, the `crates/enforcer-install/src/commands/plan.rs` emitter, and `crates/enforcer-plan/tests/self_validate.rs` — disjoint by file from b01/b02/b03/b04's modules and from d25's verify-gates module (its `tests/self_validate.rs` is a distinct integration-test file, separate from every sibling's `tests/fixtures/<name>/**`). It consumes their entrypoints read-only and is the integration/self-proof capstone; it blocks nothing.
