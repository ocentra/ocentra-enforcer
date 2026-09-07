# UL03 - Shared Syntax Extraction

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL03 Shared Syntax Extraction`
> Kind: architect-owned migration workpack.
> Read when: UL02 is committed and the grammar freeze is active.
> Stop rule: preserve behavior before adding facts, rules, or languages.
> Proves: syntax parsing is a deep shared module consumed by memory without regression.
> Does not prove: validator integration or new enforcement capability.
> Proof rule: existing parser/language/graph fixtures run unchanged across the move.
<!-- /agent-capsule -->

- owns: `crates/enforcer-syntax/**`; transferred paths named by UL02 under `crates/enforcer-memory/src/parsers/**`, `crates/enforcer-memory/src/languages/**`, `crates/enforcer-memory/vendor/**`, `crates/enforcer-memory/Cargo.toml`; integrator-only `Cargo.toml`, `Cargo.lock`, and `crates/enforcer-memory/src/lib.rs`
- deps: `UL02`
- tier: `P0 build boundary, P1 parity migration`

> Owner class: Sol/architect with a single workspace-manifest integrator.
> Batch limit: one behavior-preserving crate extraction; no new language/fact/rule.

## Where We Are

Grammar bindings, `Language`, `LangSpec`, parser dispatch, normalized `ParsedFile`, and memory persistence/graph/runtime live in one large crate. Enforcement cannot reuse parsing without importing unrelated memory dependencies.

## Where We Want To Be

`enforcer-syntax` exclusively owns classification, grammar providers, parser dispatch, safe parser boundaries, and normalized extraction. `enforcer-memory` consumes it for graph/persistence. Neither crate owns enforcement policy.

## Owns

- the new crate and only the paths transferred by the UL02 map;
- workspace manifest/lockfile through one named integrator;
- compatibility imports inside memory required by the move;
- no new fact field, validator, scan dispatch, or language behavior.

## Objective

Extract the deep reusable syntax mechanism with zero behavioral drift and a clear dependency direction: domain <- syntax <- memory/scan.

## Requirement Checklist

- [ ] Move, do not duplicate, grammar/parser implementation and vendor assets.
- [ ] `enforcer-syntax` has no SQLite, retrieval, embeddings, model, UI, coordination, rule, or memory-store dependency.
- [ ] All 160 identities and 156 structural routes remain represented.
- [ ] Existing unit-language, generic engine, unsafe-input, graph-ingest, and memory tests run unchanged or with import-only updates.
- [ ] Default/full CLI feature graphs remain intentional and documented.
- [ ] Workspace manifest and lockfile are edited once by the integrator.
- [ ] No protected vendor/CyberSkills path enters the diff.

## Acceptance And Proof

Run syntax and memory crate tests, all existing per-language fixtures, dependency-policy/import-boundary checks, cargo metadata comparison, format/clippy, and scoped Enforcer gates. The proof records before/after dependency graphs and fixture totals.

## Stop conditions

Stop on any parser output drift, missing grammar license/vendor asset, changed language count, workspace-manifest conflict, or temptation to add an enforcement shortcut during the move.

## Parallel Ownership Notes

The crate move is serialized. Read-only dependency/license audits may run in parallel. No language child edits the frozen source/destination during extraction.
