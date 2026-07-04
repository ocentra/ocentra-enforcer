# d14 Ideation Skills T3

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Ideation Skills T3`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `skills/ideation/devil.md, skills/ideation/think-with-me.md, skills/ideation/README.md, crates/enforcer-validator/src/rules/ideation_labeling.rs, crates/enforcer-validator/tests/fixtures/ideation_labeling/**`
- deps: `arc-05`
- tier: `P0 contract/schema`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The repo ships enforcement skills under `skills/ocentra-enforcer` and `skills/rust-rules-hard-gate`. ADBP includes ideation aids (a devil's-advocate pass, a think-with-me pass) that are inherently non-mechanizable judgment tools. There is no Rust `Validator` asserting that such advisory prose is honestly labeled and kept out of the gating registry.

## Where We Want To Be
Ship the ideation skills as-is under `skills/ideation/` (T3 prose — the judgment itself is unmechanizable), explicitly LABELED T3 (advisory, no mechanization possible + reason) so they never masquerade as enforcement, plus a T1 labeling `Validator` in `enforcer-validator` (arc-05) that mechanically enforces the label's presence. The mechanization is on the LABELING, not the judgment.

## Requirement Checklist
- [ ] Add `devil` and `think-with-me` skills under `skills/ideation/` as T3 advisory prose (Markdown; no engine logic).
- [ ] Each carries a mandatory header: `Tier: T3 advisory — no mechanization possible: <reason>`.
- [ ] `skills/ideation/README.md` states these produce no `Finding`s and gate nothing.
- [ ] A `Validator` impl (`ideation_labeling.rs`, built on the `enforcer-validator` trait) asserts every file under `skills/ideation/` contains the exact T3 label and emits a `Finding` fail-closed on an unlabeled ideation skill.
- [ ] These skills are excluded from any `enforcer-rules` enforcement/gating registry (no rule record routes to them).

## Acceptance And Proof
Tier T3 content, but the LABELING is enforced at T1, P0 contract/schema. Prove via `cargo test -p enforcer-validator` over `crates/enforcer-validator/tests/fixtures/ideation_labeling/**`: a labeled ideation skill file passes; an unlabeled file is flagged by the `Validator`; and a test asserts the ideation skills appear in no rule registry. Mechanism: a label-presence `Validator` over `skills/ideation/**` emitting structured `Finding`s (the mechanization is on the labeling, not the judgment). Record the detection-test artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
`deps: arc-05` only — the T3 prose is pure content; the sole code is the labeling `Validator`, which lands inside the `enforcer-validator` crate whose skeleton (trait + parity harness) arc-05 owns. This pack adds only `src/rules/ideation_labeling.rs` + its fixtures + the `skills/ideation/**` prose and must not edit the `Validator` trait itself. `owns:` is disjoint from d09 (agent docs) and d15 (README); fully concurrent once arc-05 lands.
