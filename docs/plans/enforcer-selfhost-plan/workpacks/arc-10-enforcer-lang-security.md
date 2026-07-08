# arc-10 Crate enforcer-lang-security

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-lang-security`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-security/**`
- deps: `arc-01`, `arc-02`, `arc-05`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Security-family source patterns (dangerous-call detection, secret/unsafe shapes) live in `src/source-policy-common-security*.mjs` and related `.mjs` as ad hoc JS. No crate implements the security language family against the `Validator` trait. (Distinct from arc-19 `enforcer-security`, which is Track H money-critical/security-testing validators; this is the per-family source-pattern validator crate.)

## Where We Want To Be
`enforcer-lang-security` is the per-family validator crate for security source patterns: `Validator` impls (built on `enforcer-validator`) covering the security rule family, each carrying its `ThreatId` (MITRE/OWASP) where applicable, with fail/pass fixtures and a `cargo test` detection test.

## Requirement Checklist
- [ ] Own the SEC-* rule SEMANTICS across BOTH backing validators (audit WAVE 3, MIS-MAP SEC-2). rules.json has exactly two SEC prefixes — SEC-1 (2 rules, validator `common/secret-scan`) and SEC-2 (20 rules, validator `generic-scanner`, `language: "common"`) — all `family: "security"`. arc-10 implements the RULE LOGIC + fixtures for all 22; there is NO gap and NO double-own (see engine split below).
    - [ ] SEC-1 (2, `validator: common/secret-scan`) — ported from `src/source-policy-common-security*.mjs` (`source-policy-common-security-rules.mjs` / `-sensitive.mjs` / `-manifest.mjs` / `-test-doubles.mjs`): SEC-1.1 inline-secrets forbidden (`appliesTo **/*`); SEC-1.2 sensitive-files forbidden (`.env*`, `*.pem/p12/pfx/key`, `google-services.json`, `GoogleService-Info.plist`). Both `lockLevel: immutable`, `canDisable/canDowngrade: false`.
    - [ ] SEC-2 (20, `validator: generic-scanner`, `language: common`) — SEC-2.1..SEC-2.20 (token/secret-shape detections: GitHub tokens, high-entropy secret assignments, etc.). arc-10 owns the SEC-2 RULE definitions + fail/pass fixtures; the shared `generic-scanner` ENGINE is owned by arc-09 (`src/generic-common-scanner.mjs` / `generic-scanner-shared.mjs`). arc-10 supplies the security rule slice + fixtures that arc-09's engine consumes — arc-09 provides the engine, arc-10 provides the SEC-2 semantics. So `source-policy-common-security*.mjs` alone would MISS SEC-2; this row closes that gap explicitly.
- [ ] Implement the security-family `Validator` impls per RUST_ARCHITECTURE.md, keyed to their `RuleId`s in `enforcer-rules` and tagging `ThreatId` (MITRE/OWASP) from `enforcer-domain` where applicable.
- [ ] Port the corresponding `.mjs` security detection logic (`src/source-policy-common-security*.mjs` and related) to Rust validators for the SEC-1 slice, and the SEC-2 security rule slice against arc-09's `generic-scanner` engine.
- [ ] Provide fail/pass fixtures per rule (SEC-2.* carry `requiresFailFixture: true` / `requiresPassFixture: true` in rules.json — these are provable rows); wire them through the `enforcer-validator` parity harness.
- [ ] `cargo test -p enforcer-lang-security` passes: every validator fires on its fail fixture and is silent on its pass fixture, for all 22 SEC-* rules.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-lang-security` exits 0 with fail/pass fixture coverage per rule. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-lang-security/**`. Deps arc-01/02/05. Parallel-safe with all sibling lang crates (arc-06..09, arc-11/12) and arc-13. Distinct from arc-19 (Track H security engine).

ENGINE BOUNDARY (audit WAVE 3, MIS-MAP SEC-2): SEC-2's 20 rules use the shared `generic-scanner` engine, which arc-09 owns (`generic-common-scanner.mjs` / `generic-scanner-shared.mjs`). arc-10 owns the SEC-2 rule SEMANTICS (definitions + fail/pass fixtures + `Validator` wiring of the security slice); arc-09 owns the engine that runs them. No double-own: arc-09 does NOT define the SEC-2 rule logic, and arc-10 does NOT re-implement the generic-scanner engine. SEC-1 (validator `common/secret-scan`) is fully self-contained in this crate.
