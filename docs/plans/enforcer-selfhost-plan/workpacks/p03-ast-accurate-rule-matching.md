# p03 AST-Accurate Rule Matching Via Enforcer-Memory

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `AST-Accurate Rule Matching Via Enforcer-Memory`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-ts/src/ast_provider.rs`, `crates/enforcer-lang-ts/src/rules/ast_backed.rs`, `crates/enforcer-lang-ts/tests/ast_backed.rs`, `crates/enforcer-lang-ts/tests/fixtures/ast_backed/**`, and the additive `[features] ast` + optional `enforcer-memory` dep entry in `crates/enforcer-lang-ts/Cargo.toml`
- deps: `arc-07`, `arc-05`, `x06`, `d01`
- tier: `P1` (T1 detection + d01 5-way parity; feature-gated)

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
`crates/enforcer-memory` (the x06 Rust port of codebase-memory-mcp) ships tree-sitter grammars (Rust, TypeScript/JavaScript, Python first-class, ~90 grammar deps), a `code_graph`, `data_flow`, `impact`, and `complexity` — and today NO crate consumes it except the `enforcer-ui` frontend. Its single-call parse entry point is `enforcer_memory::parsers::parse_file(Language, source, rel_path) -> Option<ParsedFile>`. Meanwhile every `enforcer-lang-ts` rule does REGEX / line / substring scanning: `source_scan.rs` matches static `needles` with a comment-only-line guard, `frontend_react.rs` hand-rolls line scanners, and the crate's own `text_scan.rs` module doc explicitly documents the substring false-positive problem and the "double-dispatch gotcha." The `enforcer_validator::Validator` trait doc already ANTICIPATES this: "Anything richer (parsed AST...) is a concern for the lang-specific crates that build on this trait." So the seam is designed-for but unbuilt.

## Where We Want To Be
An OPTIONAL, feature-gated AST PROVIDER seam inside `enforcer-lang-ts` (`src/ast_provider.rs`) that, when the `ast` feature is enabled, parses a source file once via `enforcer_memory::parsers::parse_file(Language::TypeScript, ..)` and hands the `ParsedFile` to AST-backed validators; when the feature is OFF (default), the existing substring/regex path is the fallback so the lean scan path pays no tree-sitter cost. At least one high-false-positive TS rule family is MIGRATED to AST queries in `src/rules/ast_backed.rs`, with fixture parity proving the AST path produces FEWER false positives than the regex path on the same input while still catching the true positives. Regex stays as the fallback for languages/grammars `enforcer-memory` does not cover. This is the first consumer of `enforcer-memory` outside the UI, and the template for migrating further families later (Python via arc-08, Rust via arc-06) once proven.

Candidate migration shortlist (from reading `source_scan.rs` / `frontend_react.rs` / `text_scan.rs`), in decreasing FP-severity:
1. **`TS-6.3`/`TS-6.4`/`TS-6.5` type assertions & non-null (`as` / `as X as Y` / `!`)** — the worst offender: `needles: &[" as "]` matches inside strings, comments, and `import ... as`; the bespoke `find_non_null_assertions` byte-scanner is exactly the documented "gotcha". AST `as_expression` / `non_null_expression` nodes make it exact. **Recommended first migration.**
2. **`TS-6.1` no-`any` / `FE-TS-1.5` no-explicit-any** — `find_word(text, "any")` + a preceding-comment-waiver hack; matches `any` in strings/JSDoc/property names. AST `any_type` / `type_annotation` nodes are exact.
3. **`TS-6.24`/`TS-6.19`/`TS-6.18`/`TS-6.25` console/`JSON.parse`/`process.env`/throw-string** — raw substrings firing inside string literals and comments; AST call-/member-/throw-expression matching is accurate.

## Requirement Checklist
- [ ] An `ast` cargo feature on `enforcer-lang-ts` gating an OPTIONAL `enforcer-memory` path dependency (matching the workspace `default = []` / opt-in `dep:` precedent set by `enforcer-memory`'s own `Cargo.toml`); the feature is OFF by default so the lean scan path is unchanged.
- [ ] `src/ast_provider.rs` calls `enforcer_memory::parsers::parse_file(Language::TypeScript, source, rel_path)` once per file, exposes the `ParsedFile` to AST-backed validators, and returns `None`-degrades to the regex fallback when parsing fails or the feature is off (honest fallback, never a silent miss).
- [ ] `src/rules/ast_backed.rs` migrates at least ONE rule family (recommended: `TS-6.3/6.4/6.5` assertions & non-null) to AST-node queries; the regex implementation remains as the compiled-out fallback path for the feature-off build.
- [ ] Each migrated `ruleId` keeps its 5-way parity via the d01 oracle (`ruleId <-> doc <-> validator <-> {fail+pass fixtures} <-> detection test`); the AST validator is registered under the SAME `RuleId` so downstream registry/count-parity is unaffected.
- [ ] Fixture parity proves FEWER false positives: a fixture where the regex path FALSE-POSITIVES (e.g. `" as "` inside a string literal / a comment) and the AST path stays CLEAN, PLUS a true-positive fixture both paths flag — the AST advantage is demonstrated, not asserted.
- [ ] Feature-gating verified both ways: `cargo test -p enforcer-lang-ts --features ast` exercises the AST path; the default (no-`ast`) build compiles without `enforcer-memory` in its dependency graph.

## Acceptance And Proof
Tier P1 unit (T1) + d01 parity. Proof row `ast-backed-rule-parity` in TEST_PROOF_EXPECTATIONS.md asserts `cargo test -p enforcer-lang-ts --features ast` exits 0 AND the default build stays green:
- fail-fixture: a true `as`/non-null assertion in real code -> AST validator flags it (parity fail case trips).
- pass-fixture: `" as "` appearing only inside a string literal and a comment -> the AST validator stays clean while the regex baseline would fire (the false-positive-reduction fixture, asserted by comparing both paths on the same input).
- detection test + d01 parity: every migrated `ruleId` resolves to validator export + doc anchor + both fixtures or parity fails closed; the AST validator is registered under the same `RuleId` so the arc-07 count-parity test is unaffected.
- feature gate: the default (`--no-default-features`-equivalent, feature `ast` OFF) build has no `enforcer-memory` edge (asserted by a cargo-metadata/grep-style check); `--features ast` pulls it in.
Clean `cargo clippy --features ast` / `cargo fmt --check` (obey `[workspace.lints]`).

## Parallel Ownership Notes
Owns the new `src/ast_provider.rs` + `src/rules/ast_backed.rs` + `tests/ast_backed.rs` + `tests/fixtures/ast_backed/**` inside the arc-07 crate — disjoint by file from arc-07 (crate skeleton), `d12` (`layered_frontend.rs`), and `e-pack-frontend-react` (`frontend_react.rs`). The ONE shared surface is an ADDITIVE, append-only edit to `crates/enforcer-lang-ts/Cargo.toml` (a new `[features] ast` entry + an `optional = true` `enforcer-memory` dep) — marked `Y*` with arc-07 (which owns the crate skeleton incl. `Cargo.toml`), dep-sequenced after arc-07 lands, touching no existing dependency line. HARD-depends on `x06` because it builds `crates/enforcer-memory` — this pack is the FIRST consumer of that crate outside the UI (the recon gap the plan calls out). Deps arc-05 (the `Validator` trait whose doc already anticipates the AST handle) and d01 (5-way parity oracle over the migrated ruleIds). Orthogonal to `p01` (profiles) and `p02` (scan-ignore). `owns disjoint? = Y*` (arc-07 `Cargo.toml`, additive).
