# d09 Per-Stack Agents And Doc-Rule Parity

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Per-Stack Agents And Doc-Rule Parity`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `docs/agents/**, crates/enforcer-validator/src/doc_rule_parity.rs, crates/enforcer-validator/tests/doc_rule_parity.rs, crates/enforcer-validator/tests/fixtures/doc_rule_parity/**`
- deps: `d01-rule-mechanization-engine, arc-05-enforcer-validator`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Per-stack agent guidance (must/never advice for rust/typescript/python/common) exists only as prose, and ADBP ships per-stack agent personas as narrative. The enforcer's rules are now structured records in `enforcer-rules` (typed `RuleId`s), but nothing verifies each imperative bullet in the prose is backed by a real `RuleId`. This is the T3 prose layer plus its T1 citation check; per RUST_ARCHITECTURE the AI consumes the STRUCTURED rule, and `.md` prose is the human-canonical reading only.

## Where We Want To Be
Per-stack agent docs under `docs/agents/**` (the T3 persona prose, human-canonical) PLUS a T1 `Validator` in `enforcer-validator` (`doc_rule_parity`) asserting every must/never bullet cites a `RuleId` that exists in the `enforcer-rules` registry. Prose is allowed only where it hangs off a real, mechanized rule.

## Requirement Checklist
- [ ] Author per-stack agent docs under `docs/agents/**` (T3 persona layer, clearly labeled advisory prose; not the machine-consumed source).
- [ ] Each must/never bullet carries an explicit `[ruleId]` citation resolvable to a `RuleId` newtype.
- [ ] A T1 `Validator` impl (`doc_rule_parity`, built on the `enforcer-validator` trait/harness) parses the bullets and asserts each cited id parses as a `RuleId` and exists in the `enforcer-rules` registry map (via the d01 mechanization/registry).
- [ ] Uncited must/never bullets fail closed (emit a `Finding`); the persona free-text wording itself is not gated.
- [ ] Reverse check optional: flag high-value rules with no agent-doc mention as a T2 advisory `Finding` (score + confidence, non-blocking).

## Acceptance And Proof
Tier T3 (persona prose) + T1 (citation parity), P1 unit. Prove via `cargo test -p enforcer-validator` (`crates/enforcer-validator/tests/doc_rule_parity.rs`) over `crates/enforcer-validator/tests/fixtures/doc_rule_parity/**`: a bullet citing a real id passes (no `Finding`); an uncited or dangling-id bullet fails (emits a `Finding`); persona free-text is ignored by the gate. Mechanism: a markdown bullet parser extracting `[ruleId]` tokens, checked against the registry map from d01/`enforcer-rules`.

## Parallel Ownership Notes
Depends on d01 for the registry map and arc-05 for the `enforcer-validator` crate skeleton (the `Validator` trait + parity harness) it builds on. Owns `docs/agents/**` (prose) plus only `src/doc_rule_parity.rs` and its `tests/doc_rule_parity.rs` + fixtures inside `enforcer-validator` — disjoint from the arc-05 skeleton by file, and disjoint from d15 (README) and d14 (skills) prose. owns disjoint? = Y (deps arc-05 sequences the validator after the crate skeleton exists; the `docs/agents/**` prose tree is owned by no other pack).
