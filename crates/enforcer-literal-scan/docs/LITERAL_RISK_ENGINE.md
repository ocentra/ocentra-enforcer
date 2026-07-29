# Literal Risk Engine Design

## Doctrine

Do not ban strings. Ban raw stringly domain surfaces through existing hard rules, and separately surface suspicious runtime literals as deterministic advisory risk.

A string literal risk is not a proof of bad code. It is a mechanically found location where a domain/protocol/state value may be encoded as anonymous text.

## Code targets

Literal-risk scanning applies to code files only:

- Rust: `.rs`
- TS/JS: `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`, `.mts`, `.cts`
- Python: `.py`, `.pyw`
- Many other programming languages through language-family lexers

It does not apply to Markdown, JSON, TOML, YAML, ENV, TXT, CSV, or other common text/config formats as code. Those files should be handled by common security/config/docs/provenance rules.

## Hard vs soft

Hard by default:

- secret-like literals
- literals at/above `--fail-above` when explicitly configured

Soft by default:

- event names
- routes/URLs
- HTTP/protocol strings
- state/status literals
- SQL/shell fragments
- repeated magic strings

## Scoring

Scores are additive and deterministic:

- base code literal: `+5`
- domain file: `+20`
- boundary file: `+5`
- test file: `-30`
- generated file: `-50`
- dotted event literal: `+45`
- route literal: `+40`
- URL literal: `+40`
- protocol/header/media literal: `+30`
- state/status literal: `+30`
- SQL fragment: `+60`
- shell fragment: `+70`
- comparison/control-flow context: `+35`
- repeated in two files: `+10`
- repeated in three or more files: `+20`

Scores clamp to `0..100`.

## Severity

- `0..39`: low, hidden unless `--include-low`
- `40..69`: info
- `70..84`: warning
- `85..100`: high-risk warning
- secret-like or `--fail-above`: error / blocking

## Required future Ocentra wiring

1. Add `advise literals` command in Node.
2. Call this binary and merge `hardFindings` into normal violations.
3. Put `literalRisks` into a separate advisory bucket.
4. Do not mix literal risks with raw domain string hard rules.
5. Optional strict PR-ready profile can require disposition above a score threshold.

## `LIT-2.1` — the universal literal-scan T2 advisory bridge (e01)

`src/bridge.rs`'s `LiteralScanBridgeValidator` is the Rust-native realization of the "required future Ocentra wiring" above, at the `enforcer-validator` (arc-05) `Validator` boundary rather than a Node CLI merge step:

- It implements `Validator` and runs over every scan target, independent of which (if any) bespoke language-family `Validator` also matched the file — the always-on universal T2 floor for every language this crate's registry recognizes (~65+ languages, including `.dart`, `.cfc`, `.cfm`).
- It is pure: no filesystem I/O inside `validate` — it composes `detect_language` + `classify_file_role` + `lex_literals` + `classify_literal` directly over the in-memory `ValidationInput::source`, matching the trait's purity contract.
- It maps this crate's own scored `Finding` (`score`/`confidence`) into an `enforcer_domain::findings::Finding` when `score >= min_score` (default: `DEFAULT_MIN_SCORE`, i.e. the same "info" floor as `## Severity` above), and drops anything scoring lower.
- It is structurally non-blocking: every mapped finding carries `Severity::Warning`, never `Severity::Error`, regardless of this crate's own `blocking` flag for that finding — `enforcer_domain::findings::Violation::try_from` requires `Severity::Error`, so a `LIT-2.1` finding can never promote to a blocking `Violation` or flip a `Report.ok` to `false` on its own.
- Fixtures proving the threshold-crossing behavior live at `tests/fixtures/universal/{fail,pass}/` (Dart + CFML pairs); the detection tests are `src/bridge.rs`'s `#[cfg(test)]` module plus `tests/bridge.rs` (the `cargo test -p enforcer-literal-scan` entry point named in `TEST_PROOF_EXPECTATIONS.md`: `literal-scan-universal-threshold`, `literal-scan-graceful-skip`).
- CFML (`coldfusion`, extensions `cfc`/`cfm`) is a new additive row in `src/language-registry.rs` under `LanguageFamily::Markup`, added by this pack alongside Dart (already present from arc-13) so both new-language packs (`e-pack-dart`, `e-pack-cfml`) can rely on the universal floor.
