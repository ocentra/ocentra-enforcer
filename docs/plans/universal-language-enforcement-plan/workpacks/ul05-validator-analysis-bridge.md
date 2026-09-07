# UL05 - Validator Analysis Bridge

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL05 Validator Analysis Bridge`
> Kind: architect-owned compatibility seam.
> Read when: UL04 fact/outcome contract is green.
> Stop rule: keep existing text validators behavior-compatible while enabling declared facts.
> Proves: scan parses once and fact requirements cannot silently fall through.
> Does not prove: any migrated rule is more accurate.
> Proof rule: legacy, fact-backed, unavailable, and parser-failure paths all have executable tests.
<!-- /agent-capsule -->

- owns: `crates/enforcer-validator/src/analysis.rs`, additive `crates/enforcer-validator/src/validator.rs`, `crates/enforcer-validator/tests/analysis_contract.rs`, `crates/enforcer-scan/src/analysis_cache.rs`, additive `crates/enforcer-scan/src/engine.rs`, `crates/enforcer-scan/tests/analysis_dispatch.rs`
- deps: `UL04`
- tier: `P0 validator contract, P1 scan integration`

> Owner class: Sol/architect with validator and scan singleton integrators.
> Batch limit: one backward-compatible analysis dispatch seam.

## Where We Are

`ValidationInput` exposes only file, raw source, and scope. Every validator is text/path driven. Parser output cannot be shared, and a future optional AST path would encourage per-crate parsing and inconsistent fallback.

## Where We Want To Be

Scan prepares analysis once per file. Validators declare required capabilities. Legacy validators retain their raw-source contract. Fact-backed validators receive normalized facts. Missing requirements produce a typed coverage diagnostic or declared narrow fallback, never clean.

## Owns

- a backward-compatible prepared-analysis API and capability declaration;
- parse-once cache/dispatch inside scan;
- bridge tests only, not production rule migrations;
- additive edits to singleton validator/scan files through named integrators.

## Objective

Create one reusable enforcement seam for all languages and rule families without making each rule crate depend on syntax runtime or memory.

## Requirement Checklist

- [ ] Existing validator implementations compile and preserve fixture outcomes.
- [ ] Scan invokes a provider at most once per `(file, content hash, provider version)`.
- [ ] Fact-backed validator declares a closed capability set.
- [ ] Missing/partial/error outcome is visible and cannot be an empty finding list interpreted as pass.
- [ ] Declared text fallback has separate evidence and `doesNotProve`.
- [ ] Rule crates depend only on lightweight validator/domain contracts.
- [ ] File/crate/diff scopes and deterministic ordering remain stable.

## Acceptance And Proof

Run enforcer-validator and enforcer-scan suites, analysis-dispatch positive/negative tests, parse-count instrumentation fixture, cargo metadata/import-boundary checks, and scoped Enforcer gates.

## Stop conditions

Stop if compatibility requires a second independent validator hierarchy, if parsing occurs per validator, or if unavailable facts become zero findings without diagnostics.

## Parallel Ownership Notes

Validator and scan singleton edits are serialized. Read-only call-site inventory and test-fixture preparation may run in parallel.
