# d17 Rust Error Handling

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rust Error Handling`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-rust/src/rules/error_handling.rs, crates/enforcer-lang-rust/tests/fixtures/error_handling/**`
- deps: `d01, arc-06, arc-05, arc-04`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The `enforcer-lang-rust` crate (arc-06) has partial error-handling coverage of a user's Rust code: `#[from]` cause preservation is backed, one `ExitCode` hit exists, and `SAFETY` strings are present, but the depth ADBP requires (rows 51-67, 130 of ADBP_GAPS.md) is missing. There is no ban on `.unwrap()`/`.expect()`/`panic!` in non-test paths, no `thiserror`-for-lib / `anyhow`-for-bin split, no `#[non_exhaustive]` requirement on public error enums, no `.with_context` at `?` boundaries, and no Rust-specific hardening (exhaustive match, captured format idents, no-lossy-`as`, borrow-not-own, thin `main.rs`). (These rules govern the Rust code the enforcer validates — including, under dogfood, its own crates.)

## Where We Want To Be
A Rust error-handling + hardening `Validator` family in `enforcer-lang-rust` (arc-06) — one `src/rules/error_handling.rs` module, fixtures `crates/enforcer-lang-rust/tests/fixtures/error_handling/**` — scaffolded via d01 (arc-14) with full 5-way parity. Each rule is a `Validator` impl (built on the `enforcer-validator` trait) that parses the target Rust with `syn`, walks the AST, and emits structured `Finding`s plus a terse `Fix:` hint. T1 blocks the deterministic structural rules; T2 scores the style/design rules on the `enforcer-literal-scan` scored model.

## Requirement Checklist
Each rule is scaffolded via d01, landing a doc-anchor in its `enforcer-rules` record, a `syn`-based `Validator` in `src/rules/error_handling.rs`, and a fail+pass fixture pair under `crates/enforcer-lang-rust/tests/fixtures/error_handling/<rule>/{bad,good}/`.

- [ ] **T1 RUST-ERR-NONEXHAUSTIVE — public error enums `#[non_exhaustive]`.** fail `pub enum ConfigError { NotFound }`; pass `#[non_exhaustive] pub enum ConfigError {...}`.
- [ ] **T1 (this pack's core) no `.unwrap()`/`.expect()`/`panic!` in non-test paths.** fail `let v = parse().unwrap();` in `src/` non-`#[cfg(test)]`; pass `let v = parse()?;`. Validator excludes `#[cfg(test)]` / `tests/` / `benches/`.
- [ ] **T1 RUST-SAFETY-COMMENT — `unsafe` needs `// SAFETY:`.** fail `unsafe { *ptr }` with no comment; pass `// SAFETY: ...` immediately above.
- [ ] **T1 RUST-MATCH-NO-WILDCARD / RUST-MATCH-OVER-IFLET — no catch-all `_ =>` on internal enum; prefer `match`.** fail `match s { A => .., _ => .. }` on an internal closed enum; pass exhaustive per-variant arms.
- [ ] **T1 RUST-CAST-NO-AS-LOSSY — no lossy `as`.** fail `x as u8` from a wider int; pass `u8::try_from(x)?`.
- [ ] **T1 RUST-FN-MAX-PARAMS — max 5 params (Rust-specific count gate; cross-language count gate is d19/d22).** fail `fn f(a,b,c,d,e,f)`; pass `fn f(input: FooInput)`.
- [ ] **T1 RUST-ALLOW-1.1 — `#[allow(...)]`/`#[expect]` must carry `reason = "..."`.** fail `#[allow(dead_code)]`; pass `#[allow(dead_code, reason = "...")]`.
- [ ] **T1 RUST-LAYER-1.1/2.1 / RUST-MCP-1.1 — Rust layer/MCP lane.** No forbidden-crate import in `src/domain/` and no I/O macros in domain; rmcp crate must not write `io::stdout`/`print!` (stdout is the protocol channel) — write to stderr/tracing. fail `use reqwest;` in `src/domain/x.rs` and `io::stdout().write(...)` in the rmcp crate; pass pure domain + stderr/tracing writes.
- [ ] **T2 RUST-ERR-CONTEXT / RUST-ERR-1.1 — `thiserror` lib / `anyhow` bin split; `.with_context` at `?`; cause preserved.** fail bare `read_to_string(p)?` in `commands/` and error enum without `#[from]` losing cause; pass `.with_context(|| format!("reading {p}"))?`.
- [ ] **T2 RUST-ERR-MSG-STYLE — error messages lowercase, no trailing punctuation.** fail `#[error("File Not Found.")]`; pass `#[error("config file not found")]`.
- [ ] **T2 RUST-ERR-SENTINEL / RUST-SENTINEL-1.1 — no sentinel returns; use `Result`/`Option`.** fail `fn find() -> i64 { -1 }`; pass `-> Option<T>`/`-> Result<T, E>`.
- [ ] **T2 RUST-ERR-MAIN-EXITCODE — `main` -> `ExitCode`/`anyhow::Result<()>`; no scattered `process::exit`.** fail scattered `std::process::exit(1)`; pass `fn main() -> ExitCode`.
- [ ] **T2 RUST-FMT-CAPTURED-IDENT / RUST-FMT-1.1 — inline captured format args.** fail `format!("{}", path)`; pass `format!("{path}")`.
- [ ] **T2 RUST-DOC-PUBLIC-ITEM — `///` on every public item with `# Errors`/`# Panics`.** fail `pub fn foo()` with no `///`; pass `/// Summary` above.
- [ ] **T2 RUST-FN-COMPLEXITY — cyclomatic < 10, nesting <= 3.** fail 12-branch / 5-deep fn; pass guard-claused fn.
- [ ] **T2 RUST-BORROW-1.1 — borrow read-only params.** fail `fn f(s: String)` for read-only; pass `fn f(s: &str)`.
- [ ] **T2 RUST-ARCH-1.1 — no logic in `main.rs`.** fail business fn in `main.rs`; pass `main.rs` only parse + `run()`.
- [ ] **T1/T2 RUST-NO-UTILS-MODULE — no catch-all `utils.rs`/`helpers` dumping ground >50 lines.** fail `src/utils.rs` >50 lines; pass responsibility-named module. (T1 on the banned name, T2 on the >50-line split threshold.)

### Explicitly NOT a new rule (already covered)
- **Bounded concurrency vs unbounded `tokio::spawn`: PARTIAL / already-covered.** This is backed by the existing async-runtime family (RR-8.18 / RR-8.19 / RR-8.20). Do NOT create `RUST-ASYNC-BOUNDED-CONCURRENCY`; note the mapping to RR-8.18/8.19/8.20 in the doc and defer to that family. ADBP_GAPS row 63 is superseded here.

## Acceptance And Proof
Tier P0. Prove via `cargo test -p enforcer-lang-rust`. The T1 rules block; the T2 rules score on the `enforcer-literal-scan` model (fixtures assert the score crosses the fail threshold and stays under it on pass). Non-test-path exclusion (`#[cfg(test)]`, `tests/`, `benches/`) is itself proven by a pass fixture that keeps `.unwrap()` inside a `#[cfg(test)]` module clean. For every ruleId the fail-fixture is flagged and the pass-fixture stays clean under its `Validator` impl in `src/rules/error_handling.rs`; the detection test asserts both. No T3 in this pack. Re-run the d01 `rule-scaffold-parity` oracle and record detection-test artifact paths in TEST_PROOF_EXPECTATIONS.md.

Representative triples (fixtures are `.rs` sample code the `syn`-based `Validator` parses):
- RUST-ERR-NONEXHAUSTIVE: fail `crates/enforcer-lang-rust/tests/fixtures/error_handling/nonexhaustive/bad/enum.rs`, pass `.../good/enum.rs`, `#[test] rust_err_nonexhaustive`.
- unwrap-ban: fail `crates/enforcer-lang-rust/tests/fixtures/error_handling/unwrap/bad/nontest.rs`, pass `.../good/test_module.rs` (unwrap under `#[cfg(test)]`), `#[test] rust_err_unwrap`.
- RUST-ERR-CONTEXT (T2): fail `crates/enforcer-lang-rust/tests/fixtures/error_handling/context/bad/bare_question.rs`, pass `.../good/with_context.rs`, `#[test] rust_err_context`.

## Parallel Ownership Notes
Owns `crates/enforcer-lang-rust/src/rules/error_handling.rs` and `crates/enforcer-lang-rust/tests/fixtures/error_handling/**` exclusively; disjoint from siblings. Lands inside the `enforcer-lang-rust` crate whose skeleton arc-06 owns — must not edit the crate skeleton, the baseline Rust validators (including the `no-reexports` syn check), or the language registration arc-06 owns. Depends on d01, arc-05 (Validator trait), arc-04 (rule records). The Rust layer/MCP lane here is distinct from the FSM (d16) and security (d18) packs; the async-runtime family (RR-8.18/8.19/8.20) is owned elsewhere and must not be edited from this pack.
