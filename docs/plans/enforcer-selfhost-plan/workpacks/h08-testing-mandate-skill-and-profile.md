# h08 Testing Mandate Skill And Profile

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Testing Mandate Skill And Profile`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `skills/security-testing/SKILL.md, crates/enforcer-security/src/policy_ingest.rs, crates/enforcer-security/src/policy_ingest/**, profiles/money-critical-security.json, crates/enforcer-security/tests/policy_ingest.rs, crates/enforcer-security/tests/fixtures/policy_ingest/**`
- deps: `d01, arc-19, b01`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The money-critical testing mandate has no agent-facing skill and no loadable profile. There is no mechanism to ingest an arbitrary project's security/testing spec and turn it into an enforced profile; the reference spec's §8 doctrine stays prose. The `enforcer-security` (arc-19) crate skeleton exists but hosts no `policy_ingest` module, and no neutral profile is committed.

## Where We Want To Be
Three deliverables.
1. A generic SKILL (`skills/security-testing/SKILL.md`, T3 advisory prose — human-canonical, no engine logic) that walks an agent through the mandate: route -> classify (h01) -> required categories (h02) -> threat-map (h03) -> invariants (h05) -> mechanics (h06) -> tooling/CI (h07). It produces no `Finding`s and gates nothing; it carries the mandatory `Tier: T3 advisory — no mechanization possible: <reason>` label so it never masquerades as enforcement (the LABELING is mechanized elsewhere by the d14 ideation-labeling `Validator` pattern; the judgment is not).
2. A neutral loadable profile `profiles/money-critical-security.json` (NO product/company/game branding) bundling the Track H `RuleId`s + severities (T1 block / T2 score / T3 label) + required-test-categories (§3.1–3.20) as committed DATA, shaped so `enforcer-config`/`enforcer-rules` can load it and the UI (arc-24) can render it.
3. POLICY-SPEC-INGESTION as a Rust module `crates/enforcer-security/src/policy_ingest.rs` (+ `src/policy_ingest/**` for the mapping/parse submodules): a `serde`-typed ingester that parses any project's `.mdc`/spec doc (parse-at-boundary into typed records, `thiserror` on malformed input) and maps it to a mechanized profile — generalizing "target repo owns policy". Backed rules (a `RuleId` with a real mechanized `Validator` in the registry) become ENABLED rules; un-backed asserted rules are emitted as a structured `Finding` flagging them for mechanization and fed to d01/d08 (never silently accepted). Impls the `Validator`/mapping seam on the `enforcer-validator` (arc-05) conventions, obeys `[workspace.lints]` (no `unwrap/expect/panic/print_*`; no `pub use` barrels), and reads/writes only `enforcer-domain` newtypes (`RuleId`, `Severity`, `Tier`).

## Requirement Checklist
Scaffolded via `enforcer rule new <ID>` (d01) where a rule/gate is involved; the profile + skill are committed artifacts the ingester and UI consume.
- [ ] SKILL sequences the seven Track H stages with routing preconditions; carries the T3 advisory label; produces no `Finding`s.
- [ ] Profile `profiles/money-critical-security.json` is neutral-named, lists `RuleId`s + severities (T1 block / T2 score / T3 label) + required-test-categories; round-trips through `serde` load.
- [ ] T1: ingesting the reference spec yields a profile whose categories + invariants match it (mapping equality asserted).
- [ ] T2: an ingested spec asserting an un-backed rule emits a `Finding` flagging it for mechanization (feeds d01/d08), never a silent accept.
- [ ] Malformed spec input -> typed `thiserror` boundary error, no silent default.
- [ ] No branding anywhere in skill/profile/ingest output.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Prove via `cargo test -p enforcer-security` (`crates/enforcer-security/tests/policy_ingest.rs`) over `crates/enforcer-security/tests/fixtures/policy_ingest/**`.
- pass `policy_ingest/good/ingest_reference_spec.mdc` -> profile whose required-test-categories + invariants equal the spec's §3 + §2.3 set (mapping equality), `#[test] policy_ingest_mapping`.
- fail `policy_ingest/bad/ingest_unbacked_rule.mdc` -> a spec asserting a rule with no mechanized backing emits a `Finding` (feeds d01/d08), not accepted, `#[test] policy_ingest_unbacked`.
- fail `policy_ingest/bad/malformed.mdc` -> typed boundary parse error, no silent default.
- `#[test] profile_shape` asserts `profiles/money-critical-security.json` deserializes into the typed profile record (rule ids + severities + categories present, neutral-named).
5-way parity oracle over any ingest/gate `RuleId`. Record artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
`owns:` is disjoint BY FILE: the `skills/security-testing/` prose dir, `crates/enforcer-security/src/policy_ingest.rs` + `src/policy_ingest/**`, `profiles/money-critical-security.json`, and the ingest tests/fixtures inside `enforcer-security`. Lands inside the `enforcer-security` crate whose SKELETON arc-19 owns — must NOT edit that skeleton, the no-bypass meta-check, or any sibling `src/rules/<name>.rs` (h01-h06 mechanics). References h01–h07 `RuleId`s by string only (does not open those packs). Depends on `d01` (mechanization engine — the un-backed-rule flag feeds it), `arc-19` (crate skeleton — sequences the ingest module after it exists), and `b01` (plan/profile scaffolder in `enforcer-plan`). The T3 skill's LABELING is enforced by the shared d14 ideation-labeling `Validator` (this pack does not redefine that check). `owns disjoint? = Y` (deps arc-19 sequences it after the crate skeleton exists).
