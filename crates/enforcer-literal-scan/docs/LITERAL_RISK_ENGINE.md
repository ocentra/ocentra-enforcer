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
