# CP09 - Native Capability Waves

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP09 Native Capability Waves`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-security/src/rules/cyberskills/<capability>/**`, `crates/enforcer-lang-security/tests/fixtures/cyberskills/<capability>/**`, and `proof/cyberskills/cp09/<batch-id>/**`; shared catalog/ledger application is integrator-only
- deps: `cp05`, `cp08`
- tier: `P2 T1`

> Owner class: Luna-safe only for an approved simple predicate; Sol handles complex semantics.
> Batch limit: one capability and no more than five skills.
> Depends on: CP05 and approved CP08 components.

## Where We Are

Approved native components need implementation without reverting to one file/rule per skill or accumulating end-of-program Enforcer debt.

## Where We Want To Be

Land one reusable capability at a time, map at most five skills, and prove every claim at file, crate, diff, and disposition levels.

## Objective

Implement one reusable deterministic capability that may satisfy narrowed components from several skills. Group by predicate and input shape, not by vendor directory.

## Candidate families

- typed cloud/IaC/config fields;
- HTTP headers, cookies, TLS, OAuth, JWT, and protocol configuration;
- source calls/imports/arguments through syntax facts;
- deterministic log/event signatures with labeled T2 confidence;
- manifest and CI workflow policy;
- report/telemetry schema checks.

## Requirement Checklist

- [ ] Boss approves exact component IDs, predicate, mechanism, rule IDs, files, and batch fixtures.
- [ ] Reuse an existing rule when semantics are identical; do not create aliases.
- [ ] Use typed Serde/domain parsing for stable structured formats.
- [ ] Use `enforcer-syntax` when code structure matters; no grammar dependency or raw Tree-sitter query in the rule crate.
- [ ] Keep genuinely textual matchers textual and prove boundary cases.
- [ ] Every component has source fingerprint/anchors, fail/pass/malformed/boundary fixtures, evidence, and `notProved`.
- [ ] T2 rules use a labeled benign/malicious corpus and assert score/confidence, not a disguised boolean.
- [ ] Derived coverage changes only for components exercised by the tests.

## Acceptance And Proof

After each fixture pair and validator increment, run the focused test through Enforcer. After the capability, run the changed crate, rule registry/parity, disposition gate, clippy/fmt, exact-file/crate/diff Enforcer checks, and detached-parent comparison.

## Stop conditions

Stop if the capability needs a new syntax fact, validator routing, graph traversal, external engine, or more than five skill mappings. Route it to UL04, UL05/UL06, UL13, or CP10.

## Parallel Ownership Notes

The boss replaces `<capability>` and `<batch-id>` with exact disjoint paths. The shared rule catalog and ledger are serialized integration points: `cyberskills-ledger-integrator` applies ledger changes even when validator modules are disjoint.
