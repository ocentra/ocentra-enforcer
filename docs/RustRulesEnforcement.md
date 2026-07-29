# Rust Integration Guide

<!-- ai-dense -->
```yaml
scope: "Rust surface only, not the system architecture doc -- see RUST_ARCHITECTURE.md and README.md for the full model"
implementation: enforcer-lang-rust crate (syn-backed); Rust is also the enforcer's OWN implementation language
routing: native Rust route is not wired; use finding rule IDs and focused scopes
contract_layer: enforcer-domain (serde + branded newtypes); TS types for the UI derived via ts_rs, never hand-written
meta_enforcement: "typed rule records and validator tests protect policy integrity"
```
<!-- /ai-dense -->

The enforcer is a standalone, multi-language enforcement platform. Rust is
one validated language family alongside TypeScript/JavaScript, Python, Dart,
CFML, and common security/generated-artifact checks. Proof, diagnostics, and
coordination also have engine crates, though not every public boundary is
wired. Rust is also the enforcer's own
implementation language end to end. This document explains the Rust
validator surface only; it is not the system architecture document.

For the full system model, use:

- [../README.md](../README.md)
- [../docs/plans/enforcer-selfhost-plan/RUST_ARCHITECTURE.md](plans/enforcer-selfhost-plan/RUST_ARCHITECTURE.md)
- [ENFORCED_CHECKS.md](ENFORCED_CHECKS.md)
- [COORDINATION.md](COORDINATION.md)
- [TARGET_REPO_WIRING.md](TARGET_REPO_WIRING.md)

## Consumption Model

Current native use is install-once plus explicit CLI validation:

```bash
enforcer install
enforcer scan crates/example/src/lib.rs
enforcer verify --mode local --all
```

Run validation from the target repository or pass explicit paths. The native
Rust scope grammar is paths, `--base`/`--head`, or `--all`.

## Indexed Rule Routing

The current native CLI and Rust MCP router do not expose a working rule-route
operation. Use the smallest explicit validation scope and open only the rule
records referenced by its findings.

`docs/RustRules.md` remains a fallback for broad Rust policy review, missing
registry entries, or unknown failures. The canonical form of every rule is
the typed record in `enforcer-rules`, not the prose file. The frozen Node
compatibility service still has routing, but that is not the Rust contract.

## Rust Gate Shape

Rust enforcement currently covers:

- toolchain and manifest determinism;
- lint suppression and validator bypass comments;
- unsafe, panic, unwrap, expect, debug/console macros, and erased errors;
- raw strings, raw primitives, raw public fields, raw aliases, and weak tuple
  newtypes in domain surfaces;
- wildcard imports and public Rust re-exports;
- clone/allocation/cast/indexing justification policy;
- async/runtime shape checks;
- dependency, cargo-deny, cargo-audit, fmt, clippy, test, and doc gates.

Use `rules/rust/*.md` for routed remediation details and
[ENFORCED_CHECKS.md](ENFORCED_CHECKS.md) for the high-level catalog.

## Meta-Enforcement

Rust rules are protected by typed records, validator coverage, fixtures, and
workspace tests. The current native CLI does not expose named
`rule-coverage`, `policy-integrity`, or `waiver-policy` commands. Use the
repository's focused validator tests and then the public workspace gate:

```bash
enforcer check --all
```

Immutable Rust rules cannot be disabled or downgraded by project config.
Waivers must be narrow, visible, expiring, and owner-backed where a rule
permits them.

## Contract Layer

`enforcer-domain` is the single serde-only, dependency-light contract source:
branded newtypes + serde own every DTO shape, and every id is a validated
branded newtype parsed at the boundary. It decodes configs, profiles,
registry data, route requests, scan reports, violations, init requests, and
MCP tool payloads. TS types for the optional UI are DERIVED from
`enforcer-domain` via `#[derive(ts_rs::TS)]`, guarded by a fail-closed
`cargo test` drift check (byte-compare committed vs freshly emitted) — never
hand-written.

## Target Repo Boundary

A target repo should keep product code, product-specific dev servers,
release packaging, and domain-specific proof meaning. The enforcer owns
reusable guards, coordination, compact diagnostics, proof running, policy
integrity, and architecture checks. Existing repo-local wrappers should stay
thin until old-vs-new parity is proven for file, crate/package, diff,
workspace, hook, and CI scopes.
