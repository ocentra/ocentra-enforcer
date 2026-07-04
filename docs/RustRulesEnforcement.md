# Rust Integration Guide

<!-- ai-dense -->
```yaml
scope: "Rust surface only, not the system architecture doc -- see RUST_ARCHITECTURE.md and README.md for the full model"
implementation: enforcer-lang-rust crate (syn-backed); Rust is also the enforcer's OWN implementation language
routing: enforcer route / mcp__enforcer__route -- do not default-load the monolithic RustRules.md
contract_layer: enforcer-domain (serde + branded newtypes); TS types for the UI derived via ts_rs, never hand-written
meta_enforcement: "enforcer check rule-coverage/policy-integrity/waiver-policy -- immutable rules cannot be disabled by project config"
```
<!-- /ai-dense -->

The enforcer is a standalone, multi-language enforcement platform. Rust is
one validated language family alongside TypeScript/JavaScript, Python, Dart,
CFML, and common security/generated-artifact checks, proof harnessing,
compact diagnostics, and coordination — and Rust is also the enforcer's own
implementation language end to end. This document explains the Rust
validator surface only; it is not the system architecture document.

For the full system model, use:

- [../README.md](../README.md)
- [../docs/plans/enforcer-selfhost-plan/RUST_ARCHITECTURE.md](plans/enforcer-selfhost-plan/RUST_ARCHITECTURE.md)
- [ENFORCED_CHECKS.md](ENFORCED_CHECKS.md)
- [COORDINATION.md](COORDINATION.md)
- [TARGET_REPO_WIRING.md](TARGET_REPO_WIRING.md)

## Consumption Model

Default use is install-once, MCP/CLI:

```bash
enforcer init --root <repo> --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
enforcer scan --root <repo> --files crates/example/src/lib.rs
enforcer cargo --root <repo> --crate example-crate
```

Each tool call must pass the target repo as `root`. The enforcer binary
itself never needs to live inside the target repo.

## Indexed Rule Routing

Agents should call:

```text
mcp__enforcer__route
```

Use the returned rule records instead of loading a monolithic Rust rulebook
by default. `docs/RustRules.md` remains a fallback for broad Rust policy
review, missing registry entries, or unknown failures — the canonical form
of every rule is the typed record in `enforcer-rules`, not the prose file.

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

Rust rules are protected by the shared policy layer:

```bash
enforcer check rule-coverage --root <repo>
enforcer check policy-integrity --root <repo>
enforcer check waiver-policy --root <repo>
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
